use std::cmp::Reverse;

use crossterm::event::{KeyCode, KeyEvent};
use nucleo_matcher::{
    Matcher, Utf32Str,
    pattern::{CaseMatching, Normalization, Pattern},
};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    symbols::border,
    widgets::{Block, Cell, Row, StatefulWidget, Table, TableState},
};

use crate::library::LibraryState;

pub struct LibraryWidget {
    pub state: TableState,
    pub filtered_indices: Vec<usize>, // list of filtered indices (pointing to library.tracks)
    pub search_query: String,
    matcher: Matcher,
}

impl Default for LibraryWidget {
    fn default() -> Self {
        let mut state = TableState::default();
        state.select(Some(0));
        Self {
            state,
            filtered_indices: Vec::default(),
            search_query: String::default(),
            matcher: Matcher::default(),
        }
    }
}

impl LibraryWidget {
    pub fn new(library: &LibraryState) -> Self {
        let mut widget = Self::default();
        widget.update_filter(library, "");
        widget
    }

    pub fn next(&mut self) {
        self.state.select_next();
    }

    pub fn prev(&mut self) {
        self.state.select_previous();
    }

    /// Returns visual row in the table
    pub fn selected_row(&self) -> Option<usize> {
        self.state.selected()
    }

    /// Returns real track index from library.tracks
    pub fn selected_track_index(&self) -> Option<usize> {
        self.state
            .selected()
            .and_then(|row| {
                self.filtered_indices
                    .get(row)
                    .copied()
            })
    }

    pub fn render(&mut self, library: &LibraryState, area: Rect, buf: &mut Buffer) {
        let header = Row::new(["Artists", "Title", "Length"]).style(Style::new().bold());
        let rows: Vec<Row> = self
            .filtered_indices
            .iter()
            .filter_map(|&idx| library.tracks.get(idx))
            .map(|track| {
                let duration_secs = track.length.as_secs();
                let duration_display =
                    format!("{:02}:{:02}", duration_secs / 60, duration_secs % 60);

                Row::new([
                    Cell::from(track.formatted_artists()),
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
            KeyCode::Char('l') | KeyCode::Enter => self.selected_track_index(),
            _ => None,
        }
    }

    pub fn update_filter(&mut self, library: &LibraryState, query: &str) {
        let query = query.trim();

        if query.is_empty() {
            // return all tracks
            self.filtered_indices = (0..library.tracks.len()).collect();
        } else {
            let pattern = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart);
            let mut haystack_buf = String::with_capacity(128);
            let mut utf32_buf = Vec::new();

            let mut scored_matches: Vec<(u32, usize)> = library
                .tracks
                .iter()
                .enumerate()
                .filter_map(|(idx, track)| {
                    haystack_buf.clear();
                    track.write_search_haystack(&mut haystack_buf);

                    let haystack_utf32 = Utf32Str::new(&haystack_buf, &mut utf32_buf);
                    let score = pattern.score(haystack_utf32, &mut self.matcher)?;
                    Some((score, idx))
                })
                .collect();

            // sort descending
            scored_matches.sort_by_key(|&(score, _)| Reverse(score));

            self.filtered_indices = scored_matches
                .into_iter()
                .map(|(_, idx)| idx)
                .collect();
        }

        // Change table selection to first match
        if self.filtered_indices.is_empty() {
            self.state.select(None);
        } else {
            self.state.select(Some(0));
        }
    }
}
