//! Rendering. All drawing is pure: it reads `App` state and paints a frame,
//! never mutating state. Layout mirrors Onyx: a one-row title bar on top, a
//! one-row mode/status line on the bottom, and the body in between.

mod browser;
mod editor;
mod finder;
mod preview;
mod status;
mod title_bar;

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::Span;
use ratatui::widgets::{Block, BorderType, Borders};
use ratatui::Frame;

use crate::app::{App, Screen};
use crate::theme::Theme;

pub fn render(app: &App, frame: &mut Frame) {
    let area = frame.area();
    // Paint the base background so gaps between panes use the theme color.
    frame.render_widget(Block::default().style(Style::default().bg(app.theme.bg)), area);

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // title bar
            Constraint::Min(0),    // body
            Constraint::Length(1), // mode / status line
        ])
        .split(area);

    title_bar::render(app, frame, outer[0]);

    match &app.screen {
        Screen::Browser => browser::render(app, frame, outer[1]),
        Screen::Editor { show_preview } => {
            if *show_preview {
                let panes = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
                    .split(outer[1]);
                editor::render(app, frame, panes[0]);
                preview::render(app, frame, panes[1]);
            } else {
                editor::render(app, frame, outer[1]);
            }
        }
    }

    status::render(app, frame, outer[2]);

    // Fuzzy finder overlay sits on top of everything.
    if app.finder.is_some() {
        finder::render(app, frame, area);
    }
}

/// A centered rect of (width, height) clamped inside `outer`.
pub(crate) fn centered_rect(width: u16, height: u16, outer: Rect) -> Rect {
    let w = width.min(outer.width.saturating_sub(4)).max(20);
    let h = height.min(outer.height.saturating_sub(2)).max(6);
    Rect {
        x: outer.x + (outer.width.saturating_sub(w)) / 2,
        y: outer.y + (outer.height.saturating_sub(h)) / 2,
        width: w,
        height: h,
    }
}

/// A themed pane block: rounded borders, accent + bold when focused.
pub(crate) fn pane_block(title: &str, focused: bool, theme: &Theme) -> Block<'static> {
    let border_style = if focused {
        theme.s_border_focus()
    } else {
        theme.s_border()
    };
    let title_style = if focused {
        theme.s_accent()
    } else {
        Style::default().fg(theme.fg_dim)
    };
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(Span::styled(format!(" {title} "), title_style))
        .style(Style::default().bg(theme.bg))
}
