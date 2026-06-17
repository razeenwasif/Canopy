//! File browser view: a themed list of directory entries with the selection
//! highlighted. Directories are suffixed with `/`.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState};
use ratatui::Frame;

use crate::app::App;
use crate::ui::pane_block;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let theme = &app.theme;
    let browser = &app.browser;

    let items: Vec<ListItem> = browser
        .entries()
        .iter()
        .map(|e| {
            let (icon, style) = if e.is_dir {
                ("▸ ", Style::default().fg(theme.accent_alt).add_modifier(Modifier::BOLD))
            } else {
                ("  ", Style::default().fg(theme.fg))
            };
            let name = if e.is_dir {
                format!("{}/", e.name)
            } else {
                e.name.clone()
            };
            ListItem::new(Line::from(vec![
                Span::styled(icon, style),
                Span::styled(name, style),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(pane_block("Files", true, theme))
        .highlight_style(theme.s_selection())
        .highlight_symbol("");

    let mut state = ListState::default();
    if !browser.entries().is_empty() {
        state.select(Some(browser.selected()));
    }
    frame.render_stateful_widget(list, area, &mut state);
}
