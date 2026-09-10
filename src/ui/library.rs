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

use crate::{library::LibraryState, task::THUMB_SIZE, util};

pub enum LibraryAction {
    Play(usize), // track index in library.tracks
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

        let header = Row::new([
            Cell::from(""),
            Cell::from("Artists"),
            Cell::from("Title"),
            Cell::from("Album"),
            Cell::from(Line::from("Length").right_aligned()), // cell has no right_aligned lol
        ])
        .style(Style::new().bold());
        // track number & disc number should only display in context of album, not on themselves
        // (could add an option to config for that perhaps if someone wants that)
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

                let centered_album = Text::from(vec![
                    Line::from(""),
                    Line::from(
                        track
                            .album
                            .as_deref()
                            .unwrap_or(""),
                    ),
                ]);

                let centered_duration = Text::from(vec![
                    Line::from(""),
                    Line::from(duration_display).right_aligned(),
                ]);

                Row::new([
                    Cell::from(""), // for cover art
                    Cell::from(centered_artist),
                    Cell::from(centered_title),
                    Cell::from(centered_album),
                    Cell::from(centered_duration),
                ])
                .height(THUMB_SIZE.height)
                .bottom_margin(Self::ROW_MARGIN)
            })
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
                        "title" => &track.title,
                        "genre" => track
                            .genre
                            .as_deref()
                            .unwrap_or(""),
                        "artist" => &track.formatted_artists(),
                        "album" => track
                            .album
                            .as_deref()
                            .unwrap_or(""),
                        "date" => track.date.as_deref().unwrap_or(""),
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
                    track.write_search_haystack(&mut haystack_buf);

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
