//! PDF preview pane — shows the compiled output inline (Overleaf-style).

use ratatui::layout::Rect;
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use ratatui_image::StatefulImage;

use crate::app::App;
use crate::ui::pane_block;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let theme = &app.theme;

    let name = app
        .preview_pdf
        .as_ref()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string());
    let title = match &name {
        Some(n) => format!("PDF · {n}"),
        None => "PDF".to_string(),
    };

    let block = pane_block(&title, false, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // If we have a rasterized page, draw it; otherwise a hint.
    let mut guard = app.preview_protocol.borrow_mut();
    if let Some(protocol) = guard.as_mut() {
        frame.render_stateful_widget(StatefulImage::default(), inner, protocol);
    } else {
        let hint = if app.status.starts_with("compiling") {
            "compiling…"
        } else {
            "Ctrl-B / :make to compile and preview the PDF here."
        };
        frame.render_widget(Paragraph::new(hint).style(theme.s_subtle()), inner);
    }
}
