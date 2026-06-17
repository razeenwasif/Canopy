//! PDF preview pane — shows the compiled output inline.

use ratatui::layout::Rect;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;
use crate::ui::pane_block;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let theme = &app.theme;
    // TODO(phase-4): render the rasterized PDF page with
    // `ratatui_image::StatefulImage`, plus PageUp/PageDown to change pages.
    let placeholder = Paragraph::new("Compile (Ctrl-B) to preview the PDF here.")
        .style(theme.s_subtle())
        .block(pane_block("Preview", false, theme));
    frame.render_widget(placeholder, area);
}
