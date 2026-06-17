//! AI assistant panel — docked on the right, streaming from a local Ollama
//! model. Conversation on top, input line on the bottom.

use ratatui::layout::{Constraint, Direction, Layout, Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, Focus};
use crate::ui::pane_block;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let theme = &app.theme;
    let panel = &app.ai;
    let focused = app.focus == Focus::Ai;

    let title = format!("✦ Assistant · {}", app.config.ai_model);
    let block = pane_block(&title, focused, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.width == 0 || inner.height < 2 {
        return;
    }

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(inner);

    // ── Conversation ──
    let mut lines: Vec<Line> = Vec::new();
    if panel.messages.is_empty() {
        lines.push(Line::styled(
            "Ask about LaTeX — syntax, math,",
            theme.s_subtle(),
        ));
        lines.push(Line::styled("fixing errors. Ctrl-A to focus.", theme.s_subtle()));
    }
    for msg in &panel.messages {
        let (tag, tag_style) = if msg.role == "user" {
            ("❯ you", theme.s_accent())
        } else {
            (
                "✦ assistant",
                Style::default().fg(theme.accent_alt).add_modifier(Modifier::BOLD),
            )
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
        frame.render_widget(
            Paragraph::new(Line::styled(
                "… streaming — Esc to stop",
                theme.s_subtle().add_modifier(Modifier::ITALIC),
            )),
            rows[1],
        );
    } else {
        let prompt = Paragraph::new(Line::from(vec![
            Span::styled("❯ ", theme.s_accent()),
            Span::styled(panel.input.clone(), theme.s_normal()),
        ]));
        frame.render_widget(prompt, rows[1]);
        if focused {
            let cx = rows[1].x + 2 + panel.input.chars().count() as u16;
            let max_x = rows[1].x + rows[1].width.saturating_sub(1);
            frame.set_cursor_position(Position::new(cx.min(max_x), rows[1].y));
        }
    }
}
