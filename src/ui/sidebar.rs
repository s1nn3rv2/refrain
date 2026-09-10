use std::cmp::Reverse;

use crossterm::event::{KeyCode, KeyEvent};
use nucleo_matcher::{
    Matcher, Utf32Str,
    pattern::{CaseMatching, Normalization, Pattern},
};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols::{self, border},
    widgets::{
        Block, HighlightSpacing, List, ListItem, ListState, Paragraph, StatefulWidget, Tabs, Widget,
    },
};

use crate::library::LibraryState;

pub enum SidebarAction {
    /// Sets a filter like `genre:EDM` or `None` to clear the tag
    // Done this way so user-query will still be kept in the search while we can still clear the
    // applied tag
    ApplyFilter {
        key: &'static str,     // "genre", "album", etc.
        value: Option<String>, // Some("EDM") or None for example for All
    },
}

pub enum SidebarTab {
    Genres,
    Albums,
    Artists,
}

pub struct SidebarWidget {
    selected_tab: usize,
    list_state: ListState,
    // This probably should be converted to a separate struct called SidebarCategory where it could
    // have a key, label and items. However it is fine for now. Only when we'll want more
    // configuration we should do that instead!
    genres: Vec<String>,
    albums: Vec<String>,
    artists: Vec<String>,
    pub filtered_indices: Vec<usize>, // indices into current_items()
    matcher: Matcher,
}

impl SidebarWidget {
    const TABS: [&str; 3] = ["Genres", "Albums", "Artists"];

    pub fn new(library: &LibraryState) -> Self {
        let mut widget = Self {
            selected_tab: 0,
            list_state: ListState::default(),
            genres: Vec::new(),
            albums: Vec::new(),
            artists: Vec::new(),
            filtered_indices: Vec::new(),
            matcher: Matcher::default(),
        };
        widget.list_state.select(Some(0));
        // show all
        let all_indices: Vec<usize> = (0..library.tracks.len()).collect();
        widget.update_from_filtered(library, &all_indices);
        widget
    }

    /// Returns items for the current active tab
    pub fn current_items(&self) -> &[String] {
        match self.selected_tab {
            0 => &self.genres,
            1 => &self.albums,
            2 => &self.artists,
            _ => &[],
        }
    }

    pub fn current_tag_key(&self) -> &'static str {
        match self.selected_tab {
            0 => "genre",
            1 => "album",
            2 => "artist",
            _ => "genre",
        }
    }

    /// Filters sidebar items so it shows based on search query
    /// For example, if you search for "genre:EDM" it only shows EDM albums in albums!
    pub fn update_from_filtered(&mut self, library: &LibraryState, filtered_indices: &[usize]) {
        self.genres.clear();
        self.albums.clear();
        self.artists.clear();

        for &idx in filtered_indices {
            if let Some(track) = library.tracks.get(idx) {
                if let Some(g) = &track.genre {
                    let trimmed = g.trim();
                    if !trimmed.is_empty() {
                        self.genres
                            .push(trimmed.to_string());
                    }
                }
                if let Some(a) = &track.album {
                    let trimmed = a.trim();
                    if !trimmed.is_empty() {
                        self.albums
                            .push(trimmed.to_string());
                    }
                }
                for artist in track.individual_artists() {
                    self.artists
                        .push(artist.to_string());
                }
            }
        }

        self.genres.sort();
        self.genres.dedup();

        self.albums.sort();
        self.albums.dedup();

        self.artists.sort();
        self.artists.dedup();

        self.list_state.select(Some(0));
    }

    fn next(&mut self) {
        self.selected_tab = (self.selected_tab + 1) % Self::TABS.len();
    }

    fn prev(&mut self) {
        self.selected_tab = (self.selected_tab + Self::TABS.len() - 1) % Self::TABS.len();
    }

    pub fn render(&mut self, is_focused: bool, area: Rect, buf: &mut Buffer) {
        let border_style = if is_focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let block = Block::bordered()
            .border_set(border::THICK)
            .border_style(border_style)
            .title(" Sidebar ");

        let inner = block.inner(area);
        block.render(area, buf);

        let [top, main] =
            Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(inner);

        // sucks that ratatui has no centering in native tab widget, but nothing we can do
        let tab_areas: [Rect; 3] = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(1),
        ])
        .areas(top);

        for (i, (area, title)) in tab_areas
            .into_iter()
            .zip(Self::TABS)
            .enumerate()
        {
            let style = if i == self.selected_tab {
                // highlighted style
                Style::default()
                    .fg(Color::White)
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Cyan)
            };

            Paragraph::new(title)
                .alignment(Alignment::Center)
                .style(style)
                .render(area, buf);
        }

        let items: Vec<ListItem> = std::iter::once(ListItem::new("All"))
            .chain(
                self.current_items()
                    .iter()
                    .map(|s| ListItem::new(s.clone())),
            )
            .collect();
        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .fg(Color::White)
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_spacing(HighlightSpacing::Always);

        StatefulWidget::render(list, main, buf, &mut self.list_state)
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent) -> Option<SidebarAction> {
        match key_event.code {
            KeyCode::Char('[') => {
                self.prev();
                None
            },
            KeyCode::Char(']') => {
                self.next();
                None
            },
            KeyCode::Char('j') | KeyCode::Down => {
                self.list_state.select_next();
                None
            },
            KeyCode::Char('k') | KeyCode::Up => {
                self.list_state.select_previous();
                None
            },
            KeyCode::Enter => {
                let key = self.current_tag_key();
                let selected_idx = self
                    .list_state
                    .selected()
                    .unwrap_or(0);
                let value = if selected_idx == 0 {
                    None
                } else {
                    self.current_items()
                        .get(selected_idx - 1)
                        .cloned()
                };

                Some(SidebarAction::ApplyFilter { key, value })
            },
            _ => None,
        }
    }
}
