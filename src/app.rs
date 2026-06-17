//! Application state machine and the main event loop.
//!
//! Two screens — a file `Browser` and the `Editor` (with an optional PDF
//! preview pane). The loop multiplexes terminal input against compile results
//! (delivered over a channel from a background task) with `tokio::select!`, so
//! a long-running compile never blocks typing.

use std::path::{Path, PathBuf};

use anyhow::Result;
use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use futures_util::StreamExt;
use ratatui::DefaultTerminal;
use tokio::sync::mpsc;

use crate::compile::{self, CompileOutcome, CompileRequest};
use crate::config::Config;
use crate::editor::Editor;
use crate::fs::{Activate, Browser};
use crate::ui;

pub enum Screen {
    Browser,
    Editor { show_preview: bool },
}

pub struct App {
    pub config: Config,
    pub screen: Screen,
    pub browser: Browser,
    pub editor: Option<Editor>,
    pub should_quit: bool,
    pub status: String,
    compile_rx: Option<mpsc::UnboundedReceiver<Result<CompileOutcome>>>,
}

impl App {
    pub fn new(config: Config, start_path: PathBuf) -> Result<Self> {
        let browser = Browser::new(&start_path)?;

        // If launched on a file, open it straight into the editor.
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
            screen,
            browser,
            editor,
            should_quit: false,
            status,
            compile_rx: None,
        })
    }

    pub async fn run(&mut self, mut terminal: DefaultTerminal) -> Result<()> {
        let mut events = EventStream::new();

        // Warn early if Docker isn't reachable (compile will need it).
        if !compile::docker_available().await {
            self.status = "note: Docker not reachable — compile disabled".to_string();
        }

        while !self.should_quit {
            // Keep the cursor on-screen before drawing.
            let size = terminal.size()?;
            if let (Some(editor), Screen::Editor { .. }) = (&mut self.editor, &self.screen) {
                let viewport = size.height.saturating_sub(3) as usize; // borders + status
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
        // Ignore key-release events (crossterm emits them on some platforms).
        if key.kind == KeyEventKind::Release {
            return Ok(());
        }
        // Global quit.
        if is_ctrl(&key, 'c') || is_ctrl(&key, 'q') {
            self.should_quit = true;
            return Ok(());
        }
        match self.screen {
            Screen::Browser => self.handle_browser_key(key)?,
            Screen::Editor { .. } => self.handle_editor_key(key)?,
        }
        Ok(())
    }

    fn handle_browser_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Up | KeyCode::Char('k') => self.browser.select_prev(),
            KeyCode::Down | KeyCode::Char('j') => self.browser.select_next(),
            KeyCode::Backspace | KeyCode::Left | KeyCode::Char('h') => self.browser.go_up()?,
            KeyCode::Enter | KeyCode::Char('l') => match self.browser.activate()? {
                Activate::Navigated => {}
                Activate::OpenFile(path) => self.open_file(&path)?,
            },
            _ => {}
        }
        Ok(())
    }

    fn handle_editor_key(&mut self, key: KeyEvent) -> Result<()> {
        // Editor commands (Ctrl-modified) first.
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('s') => self.save()?,
                KeyCode::Char('b') => self.start_compile(),
                KeyCode::Char('p') => self.toggle_preview(),
                _ => {}
            }
            return Ok(());
        }

        let Some(editor) = self.editor.as_mut() else { return Ok(()) };
        match key.code {
            KeyCode::Esc => self.screen = Screen::Browser,
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
        Ok(())
    }

    fn open_file(&mut self, path: &Path) -> Result<()> {
        self.editor = Some(Editor::open(path)?);
        self.screen = Screen::Editor { show_preview: false };
        self.status = format!("opened {}", path.display());
        Ok(())
    }

    fn save(&mut self) -> Result<()> {
        if let Some(editor) = self.editor.as_mut() {
            match editor.save() {
                Ok(()) => self.status = "saved".to_string(),
                Err(err) => self.status = format!("save failed: {err}"),
            }
        }
        Ok(())
    }

    fn toggle_preview(&mut self) {
        if let Screen::Editor { show_preview } = self.screen {
            self.screen = Screen::Editor {
                show_preview: !show_preview,
            };
        }
    }

    /// Save, then kick off a sandboxed compile in the background.
    fn start_compile(&mut self) {
        // Resolve the path as an owned value first, releasing the borrow before
        // we save (which needs a mutable borrow of the editor).
        let path = match self.editor.as_ref().and_then(|e| e.path()) {
            Some(p) => p.to_path_buf(),
            None => {
                self.status = "save the file before compiling".to_string();
                return;
            }
        };
        // Best-effort save before compiling.
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
            Ok(out) => {
                self.status = format!("compile {:?}", out.status);
                // TODO(phase-4): if Success, load out.pdf_path into the preview pane.
            }
            Err(err) => self.status = format!("compile error: {err}"),
        }
    }
}

/// Await an mpsc receiver that may not exist. Without a channel we return a
/// future that never resolves, so `select!` ignores this branch.
async fn recv_optional<T>(rx: &mut Option<mpsc::UnboundedReceiver<T>>) -> Option<T> {
    match rx {
        Some(rx) => rx.recv().await,
        None => std::future::pending().await,
    }
}

fn is_ctrl(key: &KeyEvent, ch: char) -> bool {
    key.code == KeyCode::Char(ch) && key.modifiers.contains(KeyModifiers::CONTROL)
}
