//! Application state machine and the main event loop.
//!
//! Two screens — a file `Browser` and the `Editor`. The editor is **modal**
//! (vim-style): Normal / Insert / Command. The loop multiplexes terminal input
//! against compile results (from a background task) with `tokio::select!`, so a
//! long-running compile never blocks typing.

use std::path::{Path, PathBuf};

use anyhow::Result;
use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use futures_util::StreamExt;
use ratatui::DefaultTerminal;
use tokio::sync::mpsc;

use crate::compile::{self, CompileOutcome, CompileRequest};
use crate::config::Config;
use crate::editor::{Editor, Mode};
use crate::finder::Finder;
use crate::fs::{Activate, Browser};
use crate::theme::Theme;
use crate::ui;

pub enum Screen {
    Browser,
    Editor { show_preview: bool },
}

pub struct App {
    pub config: Config,
    pub theme: Theme,
    pub screen: Screen,
    pub browser: Browser,
    pub editor: Option<Editor>,
    pub should_quit: bool,
    pub status: String,
    /// Active `:` command-line text (without the leading colon).
    pub cmdline: String,
    /// Project root the fuzzy finder walks (the launch directory).
    pub root: PathBuf,
    /// The fuzzy file finder overlay, when open.
    pub finder: Option<Finder>,
    /// First key of a pending two-key normal-mode sequence (`d`, `g`).
    pending_op: Option<char>,
    compile_rx: Option<mpsc::UnboundedReceiver<Result<CompileOutcome>>>,
}

impl App {
    pub fn new(config: Config, start_path: PathBuf) -> Result<Self> {
        let browser = Browser::new(&start_path)?;
        // The finder root is the launch directory (parent of a file argument).
        let root = if start_path.is_dir() {
            start_path.clone()
        } else {
            start_path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from("."))
        };
        let (screen, editor, status) = if start_path.is_file() {
            let ed = Editor::open(&start_path)?;
            (
                Screen::Editor { show_preview: false },
                Some(ed),
                format!("opened {}", start_path.display()),
            )
        } else {
            (Screen::Browser, None, "select a file to edit".to_string())
        };

        Ok(Self {
            config,
            theme: Theme::pink(),
            screen,
            browser,
            editor,
            should_quit: false,
            status,
            cmdline: String::new(),
            root,
            finder: None,
            pending_op: None,
            compile_rx: None,
        })
    }

    pub async fn run(&mut self, mut terminal: DefaultTerminal) -> Result<()> {
        let mut events = EventStream::new();

        if !compile::docker_available().await {
            self.status = "Docker not reachable — compile disabled".to_string();
        }

        while !self.should_quit {
            let size = terminal.size()?;
            if let (Some(editor), Screen::Editor { .. }) = (&mut self.editor, &self.screen) {
                let viewport = size.height.saturating_sub(4) as usize; // title + borders + status
                editor.ensure_visible(viewport);
            }

            terminal.draw(|frame| ui::render(self, frame))?;

            tokio::select! {
                maybe_event = events.next() => {
                    match maybe_event {
                        Some(Ok(event)) => self.handle_terminal_event(event)?,
                        Some(Err(err)) => self.status = format!("input error: {err}"),
                        None => self.should_quit = true,
                    }
                }
                Some(outcome) = recv_optional(&mut self.compile_rx) => {
                    self.handle_compile_result(outcome);
                }
            }
        }

        Ok(())
    }

    fn handle_terminal_event(&mut self, event: Event) -> Result<()> {
        let Event::Key(key) = event else { return Ok(()) };
        if key.kind == KeyEventKind::Release {
            return Ok(());
        }
        // Ctrl-C always quits, regardless of mode.
        if is_ctrl(&key, 'c') {
            self.should_quit = true;
            return Ok(());
        }

        // The fuzzy finder overlay captures all input while open.
        if self.finder.is_some() {
            return self.handle_finder_key(key);
        }

        match self.screen {
            Screen::Browser => self.handle_browser_key(key)?,
            Screen::Editor { .. } => match self.editor_mode() {
                Mode::Normal => self.handle_normal_key(key)?,
                Mode::Insert => self.handle_insert_key(key),
                Mode::Command => self.handle_command_key(key)?,
            },
        }
        Ok(())
    }

    fn editor_mode(&self) -> Mode {
        self.editor.as_ref().map(|e| e.mode()).unwrap_or(Mode::Normal)
    }

    // ─── Browser ──────────────────────────────────────────────────────────

    fn handle_browser_key(&mut self, key: KeyEvent) -> Result<()> {
        if is_ctrl(&key, 'q') {
            self.should_quit = true;
            return Ok(());
        }
        if is_ctrl(&key, 'f') {
            self.open_finder();
            return Ok(());
        }
        match key.code {
            KeyCode::Char('/') => self.open_finder(),
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('j') | KeyCode::Down => self.browser.select_next(),
            KeyCode::Char('k') | KeyCode::Up => self.browser.select_prev(),
            KeyCode::Char('g') => self.browser.select_first(),
            KeyCode::Char('G') => self.browser.select_last(),
            KeyCode::Char('h') | KeyCode::Left | KeyCode::Backspace => self.browser.go_up()?,
            KeyCode::Enter | KeyCode::Char('l') => match self.browser.activate()? {
                Activate::Navigated => {}
                Activate::OpenFile(path) => self.open_file(&path)?,
            },
            _ => {}
        }
        Ok(())
    }

    // ─── Fuzzy finder overlay ─────────────────────────────────────────────

    fn open_finder(&mut self) {
        self.finder = Some(Finder::new(&self.root));
    }

    fn handle_finder_key(&mut self, key: KeyEvent) -> Result<()> {
        // Ctrl-N / Ctrl-P move the selection (fzf-style), alongside arrows.
        if is_ctrl(&key, 'n') {
            if let Some(f) = self.finder.as_mut() {
                f.move_down();
            }
            return Ok(());
        }
        if is_ctrl(&key, 'p') {
            if let Some(f) = self.finder.as_mut() {
                f.move_up();
            }
            return Ok(());
        }

        match key.code {
            KeyCode::Esc => self.finder = None,
            KeyCode::Down => {
                if let Some(f) = self.finder.as_mut() {
                    f.move_down();
                }
            }
            KeyCode::Up => {
                if let Some(f) = self.finder.as_mut() {
                    f.move_up();
                }
            }
            KeyCode::Backspace => {
                if let Some(f) = self.finder.as_mut() {
                    f.pop();
                }
            }
            KeyCode::Enter => {
                let path = self.finder.as_ref().and_then(|f| f.selected_path()).map(Path::to_path_buf);
                self.finder = None;
                if let Some(path) = path {
                    self.open_file(&path)?;
                }
            }
            KeyCode::Char(c) => {
                if let Some(f) = self.finder.as_mut() {
                    f.push(c);
                }
            }
            _ => {}
        }
        Ok(())
    }

    // ─── Editor: Normal mode ──────────────────────────────────────────────

    fn handle_normal_key(&mut self, key: KeyEvent) -> Result<()> {
        // Resolve a pending two-key sequence first (dd, gg).
        if let Some(op) = self.pending_op.take() {
            if let (Some(editor), KeyCode::Char(c)) = (self.editor.as_mut(), key.code) {
                match (op, c) {
                    ('d', 'd') => editor.delete_line(),
                    ('g', 'g') => editor.move_first_line(),
                    _ => {}
                }
            }
            return Ok(());
        }

        // Ctrl-modified commands work in any editor mode.
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return self.handle_editor_ctrl(key);
        }

        let Some(editor) = self.editor.as_mut() else { return Ok(()) };
        match key.code {
            KeyCode::Char('h') | KeyCode::Left => editor.move_left(),
            KeyCode::Char('l') | KeyCode::Right => editor.move_right(),
            KeyCode::Char('j') | KeyCode::Down => editor.move_down(),
            KeyCode::Char('k') | KeyCode::Up => editor.move_up(),
            KeyCode::Char('0') | KeyCode::Home => editor.move_home(),
            KeyCode::Char('$') | KeyCode::End => editor.move_end(),
            KeyCode::Char('w') => editor.move_word_forward(),
            KeyCode::Char('b') => editor.move_word_back(),
            KeyCode::Char('G') => editor.move_last_line(),
            KeyCode::Char('g') => self.pending_op = Some('g'),
            KeyCode::Char('d') => self.pending_op = Some('d'),
            KeyCode::Char('x') => editor.delete_under(),
            KeyCode::Char('D') => editor.delete_to_eol(),
            KeyCode::Char('i') => editor.enter_insert(),
            KeyCode::Char('a') => editor.enter_insert_after(),
            KeyCode::Char('A') => editor.enter_insert_eol(),
            KeyCode::Char('I') => editor.enter_insert_bol(),
            KeyCode::Char('o') => editor.open_below(),
            KeyCode::Char('O') => editor.open_above(),
            KeyCode::PageDown => editor.page(20),
            KeyCode::PageUp => editor.page(-20),
            KeyCode::Char(':') => {
                editor.set_mode(Mode::Command);
                self.cmdline.clear();
            }
            _ => {}
        }
        Ok(())
    }

    // ─── Editor: Insert mode ──────────────────────────────────────────────

    fn handle_insert_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            let _ = self.handle_editor_ctrl(key);
            return;
        }
        let Some(editor) = self.editor.as_mut() else { return };
        match key.code {
            KeyCode::Esc => editor.exit_insert(),
            KeyCode::Up => editor.move_up(),
            KeyCode::Down => editor.move_down(),
            KeyCode::Left => editor.move_left(),
            KeyCode::Right => editor.move_right(),
            KeyCode::Home => editor.move_home(),
            KeyCode::End => editor.move_end(),
            KeyCode::PageUp => editor.page(-20),
            KeyCode::PageDown => editor.page(20),
            KeyCode::Enter => editor.insert_newline(),
            KeyCode::Tab => {
                editor.insert_char(' ');
                editor.insert_char(' ');
            }
            KeyCode::Backspace => editor.backspace(),
            KeyCode::Delete => editor.delete_forward(),
            KeyCode::Char(ch) => editor.insert_char(ch),
            _ => {}
        }
    }

    // ─── Editor: Command mode (`:`) ───────────────────────────────────────

    fn handle_command_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => self.leave_command(),
            KeyCode::Enter => {
                let cmd = self.cmdline.trim().to_string();
                self.leave_command();
                self.execute_command(&cmd);
            }
            KeyCode::Backspace => {
                if self.cmdline.pop().is_none() {
                    self.leave_command();
                }
            }
            KeyCode::Char(c) => self.cmdline.push(c),
            _ => {}
        }
        Ok(())
    }

    fn leave_command(&mut self) {
        self.cmdline.clear();
        if let Some(editor) = self.editor.as_mut() {
            editor.set_mode(Mode::Normal);
        }
    }

    fn execute_command(&mut self, cmd: &str) {
        match cmd {
            "w" => self.save(),
            "q" => {
                if self.editor.as_ref().is_some_and(|e| e.is_dirty()) {
                    self.status = "unsaved changes — use :q! to discard".to_string();
                } else {
                    self.should_quit = true;
                }
            }
            "q!" => self.should_quit = true,
            "wq" | "x" => {
                self.save();
                self.should_quit = true;
            }
            "e" => {
                self.screen = Screen::Browser;
                self.status = "file browser".to_string();
            }
            "make" => self.start_compile(),
            "" => {}
            other => self.status = format!("unknown command: :{other}"),
        }
    }

    // ─── Shared editor commands (Ctrl-…) ──────────────────────────────────

    fn handle_editor_ctrl(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('s') => self.save(),
            KeyCode::Char('b') => self.start_compile(),
            KeyCode::Char('p') => self.toggle_preview(),
            KeyCode::Char('f') => self.open_finder(),
            KeyCode::Char('o') => {
                self.screen = Screen::Browser;
                self.status = "file browser".to_string();
            }
            KeyCode::Char('d') => {
                if let Some(editor) = self.editor.as_mut() {
                    editor.page(10);
                }
            }
            KeyCode::Char('u') => {
                if let Some(editor) = self.editor.as_mut() {
                    editor.page(-10);
                }
            }
            _ => {}
        }
        Ok(())
    }

    // ─── Actions ──────────────────────────────────────────────────────────

    fn open_file(&mut self, path: &Path) -> Result<()> {
        self.editor = Some(Editor::open(path)?);
        self.screen = Screen::Editor { show_preview: false };
        self.status = format!("opened {}", path.display());
        Ok(())
    }

    fn save(&mut self) {
        if let Some(editor) = self.editor.as_mut() {
            match editor.save() {
                Ok(()) => self.status = "written".to_string(),
                Err(err) => self.status = format!("save failed: {err}"),
            }
        }
    }

    fn toggle_preview(&mut self) {
        if let Screen::Editor { show_preview } = self.screen {
            self.screen = Screen::Editor {
                show_preview: !show_preview,
            };
        }
    }

    fn start_compile(&mut self) {
        let path = match self.editor.as_ref().and_then(|e| e.path()) {
            Some(p) => p.to_path_buf(),
            None => {
                self.status = "save the file before compiling".to_string();
                return;
            }
        };
        if let Some(editor) = self.editor.as_mut() {
            if let Err(err) = editor.save() {
                self.status = format!("save failed: {err}");
                return;
            }
        }

        let work_dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
        let main_tex = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let config = self.config.clone();

        let (tx, rx) = mpsc::unbounded_channel();
        self.compile_rx = Some(rx);
        self.status = "compiling…".to_string();

        tokio::spawn(async move {
            let result = compile::compile(&config, CompileRequest { work_dir, main_tex }).await;
            let _ = tx.send(result);
        });
    }

    fn handle_compile_result(&mut self, outcome: Result<CompileOutcome>) {
        self.compile_rx = None;
        match outcome {
            Ok(out) => self.status = format!("compile {:?}", out.status),
            Err(err) => self.status = format!("compile error: {err}"),
        }
    }
}

async fn recv_optional<T>(rx: &mut Option<mpsc::UnboundedReceiver<T>>) -> Option<T> {
    match rx {
        Some(rx) => rx.recv().await,
        None => std::future::pending().await,
    }
}

fn is_ctrl(key: &KeyEvent, ch: char) -> bool {
    key.code == KeyCode::Char(ch) && key.modifiers.contains(KeyModifiers::CONTROL)
}
