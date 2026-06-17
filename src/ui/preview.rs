//! PDF preview pane — shows the compiled output inline.

use ratatui::layout::Rect;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;
use crate::ui::panel;

pub fn render(_app: &App, frame: &mut Frame, area: Rect) {
    // TODO(phase-4): render the rasterized PDF page with
    // `ratatui_image::StatefulImage`, plus PageUp/PageDown to change pages.
    // Bytes come from `pdf::rasterize_page` over the compiler's output PDF.
    let placeholder = Paragraph::new("Compile (Ctrl-B) to preview the PDF here (Phase 4).")
        .block(panel(" Preview ".to_string()));
    frame.render_widget(placeholder, area);
}
