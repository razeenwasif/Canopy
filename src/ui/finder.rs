//! Fuzzy file finder overlay — a centered fzf-style popup with a `❯` prompt and
//! a ranked list of matching files.

use ratatui::layout::{Constraint, Direction, Layout, Position, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::app::App;
use crate::ui::{centered_rect, pane_block};

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let theme = &app.theme;
    let Some(finder) = app.finder.as_ref() else { return };

    let rect = centered_rect(80, 24, area);
    frame.render_widget(Clear, rect);

    let title = format!("Find Files  ({}/{})", finder.match_count(), finder.candidate_count());
    let block = pane_block(&title, true, theme);
    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(inner);

    // Prompt line.
    let prompt = Paragraph::new(Line::from(vec![
        Span::styled("❯ ", theme.s_accent()),
        Span::styled(finder.query.clone(), theme.s_normal()),
    ]));
    frame.render_widget(prompt, rows[0]);
    // Park the cursor at the end of the query.
    let cursor_x = rows[0].x + 2 + finder.query.chars().count() as u16;
    frame.set_cursor_position(Position::new(cursor_x.min(rows[0].x + rows[0].width.saturating_sub(1)), rows[0].y));

    // Results list.
    let items: Vec<ListItem> = finder
        .display_matches()
        .into_iter()
        .map(|p| ListItem::new(Line::from(Span::styled(p, Style::default().fg(theme.fg)))))
        .collect();
    let list = List::new(items)
        .highlight_style(theme.s_selection())
        .highlight_symbol("▌ ");

    let mut state = ListState::default();
    if finder.match_count() > 0 {
        state.select(Some(finder.selected));
    }
    frame.render_stateful_widget(list, rows[1], &mut state);
}
