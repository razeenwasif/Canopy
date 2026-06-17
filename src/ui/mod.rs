//! Rendering. All drawing is pure: it reads `App` state and paints a frame,
//! never mutating state. The app loop calls `render` every tick.

mod browser;
mod editor;
mod preview;

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::{App, Screen};

pub fn render(app: &App, frame: &mut Frame) {
    // Body + a one-line status bar.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(frame.area());

    match &app.screen {
        Screen::Browser => browser::render(app, frame, chunks[0]),
        Screen::Editor { show_preview } => {
            if *show_preview {
                let panes = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
                    .split(chunks[0]);
                editor::render(app, frame, panes[0]);
                preview::render(app, frame, panes[1]);
            } else {
                editor::render(app, frame, chunks[0]);
            }
        }
    }

    render_status_bar(app, frame, chunks[1]);
}

fn render_status_bar(app: &App, frame: &mut Frame, area: Rect) {
    let hint = match app.screen {
        Screen::Browser => "↑/↓ move · Enter open · ← up · Ctrl-Q quit",
        Screen::Editor { .. } => "Ctrl-S save · Ctrl-B compile · Ctrl-P preview · Esc files",
    };
    let line = Line::from(format!(" {}  │  {hint}", app.status));
    let bar = Paragraph::new(line).style(Style::default().bg(Color::DarkGray).fg(Color::White));
    frame.render_widget(bar, area);
}

/// Shared helper: a titled bordered block.
pub(crate) fn panel(title: String) -> Block<'static> {
    Block::default().borders(Borders::ALL).title(title)
}
