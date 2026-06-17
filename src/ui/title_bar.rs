//! Top title bar — app name, current location, and the open file.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{App, Screen};

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let theme = &app.theme;
    let mut spans: Vec<Span<'static>> = Vec::new();

    spans.push(Span::styled("  🌳 Canopy ", theme.s_accent()));

    match &app.screen {
        Screen::Browser => {
            spans.push(Span::styled(
                format!("· {}  ", app.browser.cwd().display()),
                theme.s_subtle(),
            ));
        }
        Screen::Editor { .. } => {
            if let Some(editor) = &app.editor {
                if let Some(dir) = editor.path().and_then(|p| p.parent()) {
                    spans.push(Span::styled(format!("· {} ", dir.display()), theme.s_subtle()));
                }
                let name = editor
                    .path()
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "[scratch]".to_string());
                let dirty = if editor.is_dirty() { " ●" } else { "" };
                spans.push(Span::styled(
                    format!("· {name}{dirty}"),
                    theme.s_normal().add_modifier(Modifier::BOLD),
                ));
            }
        }
    }

    let p = Paragraph::new(Line::from(spans))
        .style(Style::default().bg(theme.bg_alt).fg(theme.fg));
    frame.render_widget(p, area);
}
