//! Bottom mode/status line — a colored mode block, cursor position, the active
//! `:` command line, and context hints.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{App, Screen};
use crate::editor::Mode;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let theme = &app.theme;
    let mut spans: Vec<Span<'static>> = Vec::new();

    // ── Mode block ──
    let (label, block_style) = mode_block(app);
    spans.push(Span::styled(format!(" {label} "), block_style));

    // ── Editor: cursor position + line count ──
    if let (Screen::Editor { .. }, Some(editor)) = (&app.screen, &app.editor) {
        let (line, col) = editor.cursor_line_col();
        spans.push(Span::styled(
            format!(" {}:{}  {} lines ", line + 1, col + 1, editor.rope().len_lines()),
            theme.s_subtle(),
        ));
    }

    // ── Command line takes over, otherwise show status/hint ──
    if app.editor.as_ref().map(|e| e.mode()) == Some(Mode::Command) {
        spans.push(Span::styled(
            format!(":{}", app.cmdline),
            theme.s_accent().add_modifier(Modifier::BOLD),
        ));
        // Block cursor at the end of the command.
        spans.push(Span::styled(" ", theme.s_selection()));
    } else if !app.status.is_empty() {
        spans.push(Span::styled(format!("· {} ", app.status), Style::default().fg(theme.info)));
    } else {
        spans.push(Span::styled(hint(app).to_string(), theme.s_subtle()));
    }

    let p = Paragraph::new(Line::from(spans)).style(Style::default().bg(theme.bg_alt).fg(theme.fg));
    frame.render_widget(p, area);
}

/// (label, style) for the colored mode block on the left of the status line.
fn mode_block(app: &App) -> (&'static str, Style) {
    let theme = &app.theme;
    match &app.screen {
        Screen::Browser => (
            "BROWSE",
            Style::default()
                .fg(theme.bg)
                .bg(theme.fg_subtle)
                .add_modifier(Modifier::BOLD),
        ),
        Screen::Editor { .. } => match app.editor.as_ref().map(|e| e.mode()).unwrap_or(Mode::Normal) {
            Mode::Insert => (
                "INSERT",
                theme.s_accent().add_modifier(Modifier::REVERSED),
            ),
            Mode::Command => (
                "COMMAND",
                Style::default()
                    .fg(theme.bg)
                    .bg(theme.info)
                    .add_modifier(Modifier::BOLD),
            ),
            Mode::Normal => (
                "NORMAL",
                Style::default()
                    .fg(theme.bg)
                    .bg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
        },
    }
}

fn hint(app: &App) -> &'static str {
    match &app.screen {
        Screen::Browser => "· j/k move · gg/G ends · Enter open · h up · q quit",
        Screen::Editor { .. } => {
            "· i insert · :w write · :q quit · Ctrl-B compile · Ctrl-P preview · Ctrl-O files"
        }
    }
}
