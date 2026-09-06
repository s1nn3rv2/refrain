use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, StatefulWidget, Widget},
};

use crate::library::LibraryState;

pub struct LibraryWidgetState {
    pub state: ListState,
}

impl Default for LibraryWidgetState {
    fn default() -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Self { state }
    }
}

impl LibraryWidgetState {
    pub fn next(&mut self) {
        self.state.select_next();
    }

    pub fn prev(&mut self) {
        self.state.select_previous();
    }

    pub fn selected(&self) -> Option<usize> {
        self.state.selected()
    }

    pub fn render(&mut self, library: &LibraryState, area: Rect, buf: &mut Buffer) {
        let items: Vec<ListItem> = library
            .tracks
            .iter()
            .map(|track| ListItem::new(track.title.as_str()))
            .collect();
        let list = List::new(items)
            .block(
                Block::default()
                    .title("Library")
                    .borders(Borders::ALL),
            )
            .highlight_symbol("> ")
            .highlight_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            );

        StatefulWidget::render(list, area, buf, &mut self.state);
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent) -> Option<usize> {
        match key_event.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.next();
                None
            },
            KeyCode::Char('k') | KeyCode::Up => {
                self.prev();
                None
            },
            KeyCode::Char('l') | KeyCode::Enter => self.selected(),
            _ => None,
        }
    }
}
