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
    widgets::{Block, Padding, Row, StatefulWidget, Table, TableState},
};
use ratatui_image::protocol::Protocol;

use crate::{
    library::LibraryState,
    task::THUMB_SIZE,
    ui::track::{ROW_MARGIN, render_visible_thumbnails, track_table_header, track_to_row},
    util,
};

// TODO: change usize to Track to avoid any indexing errors? will see though if its necessary
pub enum LibraryAction {
    Play(usize), // track index in library.tracks
    AddToQueue(usize),
    PlayNext(usize),
    EditTags(usize),
}

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
    // searchable tags, by key:value
    // TODO: add autocomplete suggestions for tag keys and known library values
    const TAG_KEYS: [&str; 5] = ["title", "artist", "album", "genre", "date"];

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

    // TODO: add marquee elements so text that goes beyond rect scrolls horizontally
    pub fn render(
        &mut self,
        is_focused: bool,
        library: &LibraryState,
        thumbnails: &mut LruCache<PathBuf, Option<(Size, Protocol)>>,
        visible: &mut Vec<(PathBuf, Size)>,
        area: Rect,
        buf: &mut Buffer,
    ) {
        let border_style = if is_focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let header = track_table_header().style(Style::new().bold());
        // track number & disc number should only display in context of album, not on themselves
        // (could add an option to config for that perhaps if someone wants that)
        let rows: Vec<Row> = self
            .filtered_indices
            .iter()
            .filter_map(|&idx| library.tracks.get(idx))
            .map(|track| track_to_row(track))
            .collect();

        let widths = [
            Constraint::Length(THUMB_SIZE.width),
            Constraint::Percentage(20),
            Constraint::Fill(1),
            Constraint::Percentage(20),
            Constraint::Percentage(8),
        ];

        let block = Block::bordered()
            .title(" Library ")
            .border_set(border::THICK)
            .padding(Padding::horizontal(1))
            .border_style(border_style);

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

        let paths = self
            .filtered_indices
            .iter()
            .filter_map(|&idx| library.tracks.get(idx))
            .map(|t| t.path.as_path());

        render_visible_thumbnails(
            paths,
            self.state.offset(),
            true,
            inner,
            thumbnails,
            visible,
            buf,
        );
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent) -> Option<LibraryAction> {
        match key_event.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.next();
                None
            },
            KeyCode::Char('k') | KeyCode::Up => {
                self.prev();
                None
            },
            KeyCode::Char('l') | KeyCode::Enter => self
                .selected_track_index()
                .map(LibraryAction::Play),
            KeyCode::Char('a') => self
                .selected_track_index()
                .map(LibraryAction::AddToQueue),
            KeyCode::Char('A') => self
                .selected_track_index()
                .map(LibraryAction::PlayNext),
            KeyCode::Char('e') => self
                .selected_track_index()
                .map(LibraryAction::EditTags),
            _ => None,
        }
    }

    // TODO: add a date key, where you can search by range too
    // TODO: add so you can match strictly (not fuzzy), by writing key=value instead of key:value
    // maybe?
    pub fn update_filter(&mut self, library: &LibraryState, query: &str) {
        let query = query.trim();

        if query.is_empty() {
            // return all tracks
            self.filtered_indices = (0..library.tracks.len()).collect();
            // reset table cur selected
            if self.filtered_indices.is_empty() {
                self.state.select(None);
            } else {
                self.state.select(Some(0));
            }
            return;
        }

        // separate tags (genre:xxx, artist:xxx, alum:xxx)
        let tokens = util::tokenize(query);
        let mut tags = Vec::new();
        // all non-tag words
        let mut generic_terms = Vec::new();

        for token in &tokens {
            match token.split_once(':') {
                Some((key, val)) if !val.is_empty() && Self::TAG_KEYS.contains(&key) => {
                    // strip surrounding quotes, for example: `"EDM"` -> `'EDM'`
                    let clean_val = val.trim_matches('"');
                    if !clean_val.is_empty() {
                        tags.push((
                            key,
                            Pattern::parse(clean_val, CaseMatching::Smart, Normalization::Smart),
                        ))
                    }
                },
                _ => generic_terms.push(token.as_str()),
            }
        }

        // this is everything except the tags
        let generic_query = generic_terms.join(" ");
        let generic_pat = if generic_query.is_empty() {
            None
        } else {
            Some(Pattern::parse(
                &generic_query,
                CaseMatching::Smart,
                Normalization::Smart,
            ))
        };

        let mut haystack_buf = String::with_capacity(128);
        let mut str_buf = Vec::new();

        let mut scored_matches: Vec<(u32, usize)> = library
            .tracks
            .iter()
            .enumerate()
            .filter_map(|(idx, track)| {
                // check if tags on this track match our tags from the search query
                let all_tags_match = tags.iter().all(|(key, pat)| {
                    let field: &str = match *key {
                        "title" => &track.tags.title,
                        "genre" => track
                            .tags
                            .genre
                            .as_deref()
                            .unwrap_or(""),
                        "artist" => &track.tags.formatted_artists(),
                        "album" => track
                            .tags
                            .album
                            .as_deref()
                            .unwrap_or(""),
                        "date" => track
                            .tags
                            .date
                            .as_deref()
                            .unwrap_or(""),
                        // unreachable
                        _ => return false,
                    };
                    pat.score(Utf32Str::new(field, &mut str_buf), &mut self.matcher)
                        .is_some()
                });

                if !all_tags_match {
                    return None;
                }

                if let Some(ref pat) = generic_pat {
                    haystack_buf.clear();
                    track
                        .tags
                        .write_search_haystack(&mut haystack_buf);

                    let score = pat.score(
                        Utf32Str::new(&haystack_buf, &mut str_buf),
                        &mut self.matcher,
                    )?;
                    Some((score, idx))
                } else {
                    Some((1, idx))
                }
            })
            .collect();

        // sort descending
        scored_matches.sort_by_key(|&(score, _)| Reverse(score));

        self.filtered_indices = scored_matches
            .into_iter()
            .map(|(_, idx)| idx)
            .collect();

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
