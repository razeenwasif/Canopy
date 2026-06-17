//! Editor view — renders the buffer with a line-number gutter and positions the
//! terminal's hardware cursor. Long lines are clipped (no soft-wrap) so a
//! character's column maps directly to a screen column.

use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;
use crate::ui::panel;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let Some(editor) = app.editor.as_ref() else { return };
    let rope = editor.rope();

    // Title: file name (or [scratch]) plus a dirty marker.
    let name = editor
        .path()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "[scratch]".to_string());
    let title = format!(" {}{} ", name, if editor.is_dirty() { " ●" } else { "" });

    let block = panel(title);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let scroll = editor.scroll_row();
    let total_lines = rope.len_lines();
    // Gutter wide enough for the largest visible line number, plus a space.
    let gutter_w = total_lines.to_string().len() as u16 + 1;

    let gutter_style = Style::default().fg(Color::DarkGray);
    let mut lines: Vec<Line> = Vec::with_capacity(inner.height as usize);
    for row in 0..inner.height as usize {
        let idx = scroll + row;
        if idx >= total_lines {
            break;
        }
        let text = rope
            .line(idx)
            .to_string()
            .trim_end_matches(['\n', '\r'])
            .to_string();
        let gutter = format!("{:>width$} ", idx + 1, width = (gutter_w - 1) as usize);
        lines.push(Line::from(vec![
            Span::styled(gutter, gutter_style),
            Span::raw(text),
        ]));
    }
    frame.render_widget(Paragraph::new(lines), inner);

    // Position the hardware cursor.
    let (cur_line, cur_col) = editor.cursor_line_col();
    if cur_line >= scroll && cur_line < scroll + inner.height as usize {
        let x = inner.x + gutter_w + cur_col as u16;
        let y = inner.y + (cur_line - scroll) as u16;
        // Clamp to the pane so a long line doesn't park the cursor off-screen.
        let max_x = inner.x + inner.width.saturating_sub(1);
        frame.set_cursor_position(Position::new(x.min(max_x), y));
    }
}
