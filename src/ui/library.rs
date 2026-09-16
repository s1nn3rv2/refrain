use std::{cmp::Reverse, path::PathBuf};

use crossterm::event::{KeyCode, KeyEvent};
use lru::LruCache;
use nucleo_matcher::{
    Matcher, Utf32Str,
    pattern::{CaseMatching, Normalization, Pattern},
};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect, Size},
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

#[derive(Copy, Clone, PartialEq)]
pub enum SortKey {
    Title,
    Artist,
    Album,
    Length,
}

#[derive(Copy, Clone, PartialEq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

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
    pub sort_key: SortKey,
    pub sort_direction: SortDirection,
    matcher: Matcher,
}

impl Default for LibraryWidget {
    fn default() -> Self {
        let mut state = TableState::default();
        state.select(Some(0));
        Self {
            state,
            filtered_indices: Vec::default(),
            sort_key: SortKey::Title,
            sort_direction: SortDirection::Ascending,
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

        let block = Block::bordered()
            .title(" Library ")
            .border_set(border::THICK)
            .padding(Padding::horizontal(1))
            .border_style(border_style);

        let inner = block.inner(area);

        let header =
            track_table_header(self.sort_key, self.sort_direction).style(Style::new().bold());

        let widths = [
            Constraint::Length(THUMB_SIZE.width),
            Constraint::Percentage(20),
            Constraint::Fill(1),
            Constraint::Percentage(20),
            Constraint::Percentage(8),
        ];

        let [_, artist_col, title_col, album_col, _] = Layout::horizontal(widths)
            .spacing(1)
            .areas(inner);

        let offset = self.state.offset();
        // how many tracks can fit on screen, + 2 at end: +1 for table header row and +1 for safety
        let visible_count = (inner.height / (THUMB_SIZE.height + ROW_MARGIN)) as usize + 2;
        let visible_range = offset..offset + visible_count;

        // track number & disc number should only display in context of album, not on themselves
        // (could add an option to config for that perhaps if someone wants that)
        let rows: Vec<Row> = self
            .filtered_indices
            .iter()
            .enumerate()
            .filter_map(|(i, &idx)| {
                library
                    .tracks
                    .get(idx)
                    .map(|t| (i, t))
            })
            .map(|(i, track)| {
                if visible_range.contains(&i) {
                    track_to_row(
                        track,
                        artist_col.width as usize,
                        title_col.width as usize,
                        album_col.width as usize,
                    )
                } else {
                    // off-screen, we dont use marquee then
                    track_to_row(track, usize::MAX, usize::MAX, usize::MAX)
                }
            })
            .collect();

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

    pub fn handle_key_event(
        &mut self,
        key_event: KeyEvent,
        library: &LibraryState,
    ) -> Option<LibraryAction> {
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
            KeyCode::Char('s') => {
                self.sort_key = match self.sort_key {
                    SortKey::Artist => SortKey::Title,
                    SortKey::Title => SortKey::Album,
                    SortKey::Album => SortKey::Length,
                    SortKey::Length => SortKey::Artist,
                };
                self.sort(library);
                None
            },
            KeyCode::Char('S') => {
                self.sort_direction = match self.sort_direction {
                    SortDirection::Ascending => SortDirection::Descending,
                    SortDirection::Descending => SortDirection::Ascending,
                };
                self.sort(library);
                None
            },
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
            self.sort(library);
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

    pub fn sort(&mut self, library: &LibraryState) {
        match self.sort_key {
            SortKey::Title => self
                .filtered_indices
                .sort_by_cached_key(|&i| {
                    library.tracks[i]
                        .tags
                        .title
                        .to_lowercase()
                }),
            SortKey::Artist => self
                .filtered_indices
                .sort_by_cached_key(|&i| {
                    library.tracks[i]
                        .tags
                        .artists
                        .to_lowercase()
                }),
            SortKey::Album => self
                .filtered_indices
                .sort_by_cached_key(|&i| {
                    library.tracks[i]
                        .tags
                        .album
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                }),
            SortKey::Length => self
                .filtered_indices
                .sort_by_key(|&i| library.tracks[i].length),
        }

        if self.sort_direction == SortDirection::Descending {
            self.filtered_indices.reverse();
        }
    }
}
