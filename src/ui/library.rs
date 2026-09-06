use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    symbols::border,
    widgets::{Block, Cell, Row, StatefulWidget, Table, TableState},
};

use crate::library::LibraryState;

pub struct LibraryWidgetState {
    pub state: TableState,
}

impl Default for LibraryWidgetState {
    fn default() -> Self {
        let mut state = TableState::default();
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
        let header = Row::new(["Artists", "Title", "Length"]).style(Style::new().bold());
        let rows: Vec<Row> = library
            .tracks
            .iter()
            .map(|track| {
                let duration_secs = track.length.as_secs();
                let duration_display =
                    format!("{:02}:{:02}", duration_secs / 60, duration_secs % 60);

                Row::new([
                    Cell::from(track.artist.as_str()),
                    Cell::from(track.title.as_str()),
                    Cell::from(duration_display),
                ])
            })
            .collect();

        let widths = [
            Constraint::Percentage(30),
            Constraint::Percentage(50),
            Constraint::Percentage(20),
        ];

        let table = Table::new(rows, widths)
            .header(header)
            .block(
                Block::bordered()
                    .title(" Library ")
                    .border_set(border::THICK),
            )
            .column_spacing(1)
            .row_highlight_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        StatefulWidget::render(table, area, buf, &mut self.state);
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
