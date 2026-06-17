//! PDF preview pane — shows the compiled output inline (Overleaf-style), with
//! scroll/zoom/page navigation when focused.

use ratatui::layout::Rect;
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use ratatui_image::StatefulImage;

use crate::app::{App, Focus};
use crate::ui::pane_block;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let theme = &app.theme;
    let focused = app.focus == Focus::Preview;

    let mut guard = app.pdf_view.borrow_mut();

    let title = match guard.as_ref() {
        Some(view) => format!("PDF · {}", view.status()),
        None => "PDF".to_string(),
    };
    let block = pane_block(&title, focused, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if let Some(view) = guard.as_mut() {
        let mut picker = app.picker.borrow_mut();
        if let Some(protocol) = view.protocol_for(&mut picker, inner) {
            frame.render_stateful_widget(StatefulImage::default(), inner, protocol);
        }
    } else {
        let hint = if app.status.starts_with("compiling") {
            "compiling…"
        } else {
            "Ctrl-B / :make to compile and preview the PDF here."
        };
        frame.render_widget(Paragraph::new(hint).style(theme.s_subtle()), inner);
    }
}
