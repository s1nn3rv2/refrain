use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent};
use lru::LruCache;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect, Size},
    style::{Color, Modifier, Style},
    symbols::border,
    text::{Line, Span, Text},
    widgets::{Block, Cell, Padding, Row, StatefulWidget, Table, TableState},
};
use ratatui_image::protocol::Protocol;

use crate::{
    queue::QueueManager,
    task::THUMB_SIZE,
    ui::track::{ROW_MARGIN, render_visible_thumbnails},
    util::DurationExt,
};

pub enum QueueAction {
    Play(usize), // Play immediately
    Remove(usize),
}

pub struct QueueWidget {
    pub state: TableState,
}

impl Default for QueueWidget {
    fn default() -> Self {
        let mut state = TableState::default();
        state.select(Some(0));
        Self { state }
    }
}

impl QueueWidget {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn next(&mut self) {
        self.state.select_next();
    }

    pub fn prev(&mut self) {
        self.state.select_previous();
    }

    /// Returns real track index from library.tracks
    pub fn selected_track_index(&self) -> Option<usize> {
        self.state.selected()
    }

    pub fn render(
        &mut self,
        is_focused: bool,
        queue: &QueueManager,
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

        let title = format!(" Queue ({}) ", queue.user_queue.len());

        let block = Block::bordered()
            .title(title)
            .border_set(border::THICK)
            .padding(Padding::horizontal(1))
            .border_style(border_style);

        let inner = block.inner(area);

        let rows: Vec<Row> = queue
            .user_queue
            .iter()
            .map(|track| {
                let time = track.length.format_time();
                let album_name = track
                    .album
                    .as_deref()
                    .unwrap_or("Single");

                // Line 1: Title (bold)
                let line_title = Line::from(Span::styled(
                    track.title.as_str(),
                    Style::default().add_modifier(Modifier::BOLD),
                ));

                // Line 2: Artist (light gray)
                let line_artist = Line::from(Span::styled(
                    track.formatted_artists(),
                    Style::default().fg(Color::Gray),
                ));

                // Line 3: Album (dark gray) with Duration
                let line_album = Line::from(vec![
                    Span::styled(album_name, Style::default().fg(Color::DarkGray)),
                    Span::raw("  "),
                    Span::styled(time, Style::default().fg(Color::DarkGray)),
                ]);

                let card_text = Text::from(vec![line_title, line_artist, line_album]);

                Row::new([
                    Cell::from(""),        // Cell 1: reserved for 3-line cover art
                    Cell::from(card_text), // Cell 2: stacked 3-line details
                ])
                .height(THUMB_SIZE.height)
                .bottom_margin(ROW_MARGIN)
            })
            .collect();

        // 2 columns: thumbnail width (7) + remaining drawer width
        let widths = [Constraint::Length(THUMB_SIZE.width), Constraint::Fill(1)];

        let table = Table::new(rows, widths)
            .block(block)
            .column_spacing(1)
            .row_highlight_style(
                Style::default()
                    .fg(Color::White)
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            );

        StatefulWidget::render(table, area, buf, &mut self.state);

        let paths = queue
            .user_queue
            .iter()
            .map(|t| t.path.as_path());

        render_visible_thumbnails(
            paths,
            self.state.offset(),
            false,
            inner,
            thumbnails,
            visible,
            buf,
        );
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent) -> Option<QueueAction> {
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
                .state
                .selected()
                .map(QueueAction::Play),
            KeyCode::Char('d') => self
                .state
                .selected()
                .map(QueueAction::Remove),
            _ => None,
        }
    }
}
