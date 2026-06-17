//! Application state machine and the main event loop.
//!
//! Two screens — a file `Browser` and the `Editor`. The editor is **modal**
//! (vim-style): Normal / Insert / Command. The loop multiplexes terminal input
//! against compile results (from a background task) with `tokio::select!`, so a
//! long-running compile never blocks typing.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::Result;
use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use futures_util::StreamExt;
use ratatui::DefaultTerminal;
use tokio::sync::mpsc;

use crate::ai::{self, AiEvent, ChatMessage};
use crate::compile::{self, CompileOutcome, CompileRequest};
use crate::config::Config;
use crate::editor::{Editor, Mode};
use crate::finder::Finder;
use crate::fs::{Activate, Browser};
use crate::theme::Theme;
use crate::ui;

const AI_SYSTEM_PROMPT: &str = "You are a concise LaTeX writing assistant embedded in a terminal \
editor. Help with LaTeX syntax, fixing compile errors, math, and document structure. Prefer short \
answers and ready-to-paste LaTeX snippets.";

/// State for the AI assistant overlay.
pub struct AiPanel {
    /// Displayed conversation (user / assistant turns).
    pub messages: Vec<ChatMessage>,
    pub input: String,
    pub streaming: bool,
    pub scroll: u16,
}

impl AiPanel {
    fn new() -> Self {
        Self {
            messages: Vec::new(),
            input: String::new(),
            streaming: false,
            scroll: 0,
        }
    }
}

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
    /// The AI assistant overlay, when open.
    pub ai: Option<AiPanel>,
    /// First key of a pending two-key normal-mode sequence (`d`, `g`).
    pending_op: Option<char>,
    compile_rx: Option<mpsc::UnboundedReceiver<Result<CompileOutcome>>>,
    ai_rx: Option<mpsc::UnboundedReceiver<AiEvent>>,
    ai_cancel: Option<Arc<AtomicBool>>,
}

impl App {
    pub fn new(config: Config, start_path: PathBuf) -> Result<Self> {
        let browser = Browser::new(&start_path)?;
        // The finder is scoped to the project: the nearest enclosing git repo,
        // falling back to the launch directory.
        let root = project_root(&start_path);
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
            ai: None,
            pending_op: None,
            compile_rx: None,
            ai_rx: None,
            ai_cancel: None,
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
                Some(ev) = recv_optional(&mut self.ai_rx) => {
                    self.handle_ai_event(ev);
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

        // Overlays capture all input while open.
        if self.finder.is_some() {
            return self.handle_finder_key(key);
        }
        if self.ai.is_some() {
            return self.handle_ai_key(key);
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
        if is_ctrl(&key, 'a') {
            self.open_ai();
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

    // ─── AI assistant overlay ─────────────────────────────────────────────

    fn open_ai(&mut self) {
        if self.ai.is_none() {
            self.ai = Some(AiPanel::new());
        }
    }

    fn handle_ai_key(&mut self, key: KeyEvent) -> Result<()> {
        let streaming = self.ai.as_ref().map(|a| a.streaming).unwrap_or(false);
        match key.code {
            KeyCode::Esc => {
                if streaming {
                    self.cancel_ai();
                } else {
                    self.ai = None;
                }
            }
            KeyCode::Enter if !streaming => self.submit_ai(),
            KeyCode::PageUp => {
                if let Some(a) = self.ai.as_mut() {
                    a.scroll = a.scroll.saturating_sub(4);
                }
            }
            KeyCode::PageDown => {
                if let Some(a) = self.ai.as_mut() {
                    a.scroll = a.scroll.saturating_add(4);
                }
            }
            KeyCode::Backspace if !streaming => {
                if let Some(a) = self.ai.as_mut() {
                    a.input.pop();
                }
            }
            KeyCode::Char(c) if !streaming => {
                if let Some(a) = self.ai.as_mut() {
                    a.input.push(c);
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn cancel_ai(&mut self) {
        if let Some(c) = &self.ai_cancel {
            c.store(true, Ordering::Relaxed);
        }
        self.ai_cancel = None;
        self.ai_rx = None;
        if let Some(a) = self.ai.as_mut() {
            a.streaming = false;
        }
    }

    fn submit_ai(&mut self) {
        let input = match self.ai.as_mut() {
            Some(a) if !a.input.trim().is_empty() => std::mem::take(&mut a.input),
            _ => return,
        };

        // Build the request: system prompt + current document context + history.
        let mut send = vec![ChatMessage::system(AI_SYSTEM_PROMPT)];
        if let Some(editor) = &self.editor {
            let doc = editor.rope().to_string();
            if !doc.trim().is_empty() {
                let snippet: String = doc.chars().take(4000).collect();
                send.push(ChatMessage::system(format!(
                    "The user's current LaTeX document:\n\n{snippet}"
                )));
            }
        }
        if let Some(a) = self.ai.as_ref() {
            send.extend(a.messages.iter().cloned());
        }
        send.push(ChatMessage::user(input.clone()));

        // Update the displayed conversation: user turn + empty assistant turn.
        if let Some(a) = self.ai.as_mut() {
            a.messages.push(ChatMessage::user(input));
            a.messages.push(ChatMessage::assistant(String::new()));
            a.streaming = true;
            a.scroll = 0;
        }

        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::unbounded_channel();
        self.ai_rx = Some(rx);
        self.ai_cancel = Some(cancel.clone());
        let host = self.config.ollama_host.clone();
        let model = self.config.ai_model.clone();
        tokio::spawn(ai::chat_stream(host, model, send, cancel, tx));
    }

    fn handle_ai_event(&mut self, ev: AiEvent) {
        match ev {
            AiEvent::Delta(s) => {
                if let Some(a) = self.ai.as_mut() {
                    if let Some(last) = a.messages.last_mut() {
                        if last.role == "assistant" {
                            last.content.push_str(&s);
                        }
                    }
                }
            }
            AiEvent::Done => {
                self.ai_rx = None;
                self.ai_cancel = None;
                if let Some(a) = self.ai.as_mut() {
                    a.streaming = false;
                }
            }
            AiEvent::Error(e) => {
                self.ai_rx = None;
                self.ai_cancel = None;
                if let Some(a) = self.ai.as_mut() {
                    a.streaming = false;
                    if let Some(last) = a.messages.last_mut() {
                        if last.role == "assistant" && last.content.is_empty() {
                            last.content = format!("[error] {e}");
                        }
                    }
                }
                self.status = format!("ai: {e}");
            }
        }
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
            KeyCode::Char('a') => self.open_ai(),
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
        use crate::compile::CompileStatus;
        self.compile_rx = None;
        match outcome {
            Ok(out) => {
                let secs = out.duration_ms as f64 / 1000.0;
                self.status = match out.status {
                    CompileStatus::Success => {
                        let name = out
                            .pdf_path
                            .as_ref()
                            .and_then(|p| p.file_name())
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| "output.pdf".to_string());
                        format!("✓ compiled in {secs:.1}s → {name}")
                    }
                    CompileStatus::Failed => {
                        format!("✗ compile failed in {secs:.1}s — {}", last_error_line(&out.log))
                    }
                    CompileStatus::Timeout => format!("✗ compile timed out after {secs:.1}s"),
                };
            }
            Err(err) => self.status = format!("compile error: {err}"),
        }
    }
}

/// Pull the most informative line out of a TeX log (the first `! …` error).
fn last_error_line(log: &str) -> String {
    log.lines()
        .find(|l| l.starts_with('!'))
        .map(|l| l.trim().chars().take(80).collect())
        .unwrap_or_else(|| "see log".to_string())
}

/// The project root for the fuzzy finder: the nearest ancestor containing a
/// `.git` directory, or the launch directory if none is found.
fn project_root(start: &Path) -> PathBuf {
    let base = if start.is_dir() {
        start.to_path_buf()
    } else {
        start
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    };
    let base = base.canonicalize().unwrap_or(base);

    let mut cur = base.as_path();
    loop {
        if cur.join(".git").exists() {
            return cur.to_path_buf();
        }
        match cur.parent() {
            Some(parent) => cur = parent,
            None => break,
        }
    }
    base
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
