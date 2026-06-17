//! File browser view: a list of directory entries with the selection
//! highlighted. Directories are suffixed with `/`.

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{List, ListItem, ListState};
use ratatui::Frame;

use crate::app::App;
use crate::ui::panel;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let browser = &app.browser;

    let items: Vec<ListItem> = browser
        .entries()
        .iter()
        .map(|e| {
            let label = if e.is_dir {
                format!("{}/", e.name)
            } else {
                e.name.clone()
            };
            let style = if e.is_dir {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(Line::styled(label, style))
        })
        .collect();

    let title = format!(" {} ", browser.cwd().display());
    let list = List::new(items).block(panel(title)).highlight_style(
        Style::default()
            .bg(Color::Blue)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    );

    let mut state = ListState::default();
    if !browser.entries().is_empty() {
        state.select(Some(browser.selected()));
    }
    frame.render_stateful_widget(list, area, &mut state);
}
