use std::{cmp::Reverse, path::PathBuf};

use crossterm::event::{KeyCode, KeyEvent};
use lru::LruCache;
use nucleo_matcher::{
    Matcher, Utf32Str,
    pattern::{CaseMatching, Normalization, Pattern},
};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect, Size},
    style::{Color, Modifier, Style},
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Cell, Padding, Row, StatefulWidget, Table, TableState, Widget},
};
use ratatui_image::{Image, protocol::Protocol};

use crate::{library::LibraryState, task::THUMB_SIZE};

pub struct LibraryWidget {
    pub state: TableState,
    pub filtered_indices: Vec<usize>, // list of filtered indices (pointing to library.tracks)
    matcher: Matcher,
}

impl Default for LibraryWidget {
    fn default() -> Self {
        let mut state = TableState::default();
        state.select(Some(0));
        Self {
            state,
            filtered_indices: Vec::default(),
            matcher: Matcher::default(),
        }
    }
}

impl LibraryWidget {
    const ROW_MARGIN: u16 = 1;

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

    pub fn render(
        &mut self,
        library: &LibraryState,
        thumbnails: &mut LruCache<PathBuf, Option<(Size, Protocol)>>,
        visible: &mut Vec<(PathBuf, Size)>,
        area: Rect,
        buf: &mut Buffer,
    ) {
        let header = Row::new([
            Cell::from(""),
            Cell::from("Artists"),
            Cell::from("Title"),
            Cell::from(Line::from("Length").right_aligned()), // cell has no right_aligned lol
        ])
        .style(Style::new().bold());
        let rows: Vec<Row> = self
            .filtered_indices
            .iter()
            .filter_map(|&idx| library.tracks.get(idx))
            .map(|track| {
                let duration_secs = track.length.as_secs();
                let duration_display =
                    format!("{:02}:{:02}", duration_secs / 60, duration_secs % 60);

                let centered_artist =
                    Text::from(vec![Line::from(""), Line::from(track.formatted_artists())]);

                let centered_title =
                    Text::from(vec![Line::from(""), Line::from(track.title.as_str())]);

                let centered_duration = Text::from(vec![
                    Line::from(""),
                    Line::from(duration_display).right_aligned(),
                ]);

                Row::new([
                    Cell::from(""), // for cover art
                    Cell::from(centered_artist),
                    Cell::from(centered_title),
                    Cell::from(centered_duration),
                ])
                .height(THUMB_SIZE.height)
                .bottom_margin(Self::ROW_MARGIN)
            })
            .collect();

        let widths = [
            Constraint::Length(THUMB_SIZE.width),
            Constraint::Percentage(35),
            Constraint::Fill(1),
            Constraint::Percentage(8),
        ];

        let block = Block::bordered()
            .title(" Library ")
            .border_set(border::THICK)
            .padding(Padding::horizontal(1));

        let inner = block.inner(area);

        let table = Table::new(rows, widths)
            .header(header)
            .block(block)
            .column_spacing(1)
            .row_highlight_style(
                Style::default()
                    .fg(Color::White)
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            );

        StatefulWidget::render(table, area, buf, &mut self.state);

        let first_row_y = inner.y + 1;
        let first_row_x = inner.x;
        let offset = self.state.offset(); // scroll position, index of first visible row

        for (screen_row, &track_index) in self
            .filtered_indices
            .iter()
            .skip(offset)
            .enumerate()
        {
            let y = first_row_y + screen_row as u16 * (THUMB_SIZE.height + Self::ROW_MARGIN);
            if y + THUMB_SIZE.height > inner.y + inner.height {
                break;
            }

            let Some(track) = library.tracks.get(track_index) else {
                continue;
            };

            let rect = Rect {
                x: first_row_x,
                y,
                width: THUMB_SIZE.width,
                height: THUMB_SIZE.height,
            };

            match thumbnails.get(&track.path) {
                // cached at size we want, draw
                Some(Some((size, protocol))) if *size == THUMB_SIZE => {
                    Image::new(protocol).render(rect, buf);
                },
                // cached at other size, draw it and then ask for rebuild
                Some(Some((_, protocol))) => {
                    Image::new(protocol)
                        .allow_clipping(true)
                        .render(rect, buf);
                    visible.push((track.path.clone(), THUMB_SIZE));
                },
                // track has no art
                Some(None) => {},
                // never seen, ask for it
                None => visible.push((track.path.clone(), THUMB_SIZE)),
            }
        }
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
        // TODO: make so it doesn't always change when the track you already have selected appears
        // in the search results
        if self.filtered_indices.is_empty() {
            self.state.select(None);
        } else {
            self.state.select(Some(0));
        }
    }
}
