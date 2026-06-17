//! AI assistant overlay — a centered chat panel streaming from a local Ollama
//! model. Conversation on top, input line on the bottom.

use ratatui::layout::{Constraint, Direction, Layout, Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;
use crate::ui::{centered_rect, pane_block};

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let theme = &app.theme;
    let Some(panel) = app.ai.as_ref() else { return };

    let rect = centered_rect(100, 32, area);
    frame.render_widget(Clear, rect);

    let title = format!("Assistant · {}", app.config.ai_model);
    let block = pane_block(&title, true, theme);
    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(inner);

    // ── Conversation ──
    let mut lines: Vec<Line> = Vec::new();
    if panel.messages.is_empty() {
        lines.push(Line::styled(
            "Ask about LaTeX — syntax, math, fixing errors. Enter to send · Esc to close.",
            theme.s_subtle(),
        ));
    }
    for msg in &panel.messages {
        let (tag, tag_style) = if msg.role == "user" {
            ("❯ you", theme.s_accent())
        } else {
            ("✦ assistant", Style::default().fg(theme.accent_alt).add_modifier(Modifier::BOLD))
        };
        lines.push(Line::styled(tag, tag_style));
        for text_line in msg.content.split('\n') {
            lines.push(Line::styled(text_line.to_string(), theme.s_normal()));
        }
        lines.push(Line::raw(""));
    }

    let convo = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((panel.scroll, 0));
    frame.render_widget(convo, rows[0]);

    // ── Input line ──
    if panel.streaming {
        let p = Paragraph::new(Line::styled(
            "  … streaming — Esc to stop",
            theme.s_subtle().add_modifier(Modifier::ITALIC),
        ));
        frame.render_widget(p, rows[1]);
    } else {
        let prompt = Paragraph::new(Line::from(vec![
            Span::styled("❯ ", theme.s_accent()),
            Span::styled(panel.input.clone(), theme.s_normal()),
        ]));
        frame.render_widget(prompt, rows[1]);
        let cx = rows[1].x + 2 + panel.input.chars().count() as u16;
        let max_x = rows[1].x + rows[1].width.saturating_sub(1);
        frame.set_cursor_position(Position::new(cx.min(max_x), rows[1].y));
    }
}
