//! Editor view — themed buffer with a line-number gutter (active line in
//! accent) and the terminal's hardware cursor. Long lines are clipped.

use ratatui::layout::{Position, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;
use crate::editor::Mode;
use crate::ui::pane_block;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let theme = &app.theme;
    let Some(editor) = app.editor.as_ref() else { return };
    let rope = editor.rope();

    let name = editor
        .path()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "[scratch]".to_string());
    let title = format!("{}{}", name, if editor.is_dirty() { " ●" } else { "" });

    let block = pane_block(&title, true, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let scroll = editor.scroll_row();
    let total_lines = rope.len_lines();
    let (cur_line, cur_col) = editor.cursor_line_col();
    let gutter_w = total_lines.to_string().len() as u16 + 1;

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
        let gutter_style = if idx == cur_line {
            theme.s_gutter_active()
        } else {
            theme.s_gutter()
        };
        lines.push(Line::from(vec![
            Span::styled(gutter, gutter_style),
            Span::styled(text, theme.s_normal()),
        ]));
    }
    frame.render_widget(Paragraph::new(lines), inner);

    // Hardware cursor — hidden while the `:` command line is active.
    if editor.mode() != Mode::Command && cur_line >= scroll && cur_line < scroll + inner.height as usize {
        let x = inner.x + gutter_w + cur_col as u16;
        let y = inner.y + (cur_line - scroll) as u16;
        let max_x = inner.x + inner.width.saturating_sub(1);
        frame.set_cursor_position(Position::new(x.min(max_x), y));
    }
}
