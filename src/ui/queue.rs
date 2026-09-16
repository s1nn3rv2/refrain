use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use lru::LruCache;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect, Size},
    style::{Color, Modifier, Style},
    symbols::border,
    widgets::{Block, Padding, Row, StatefulWidget, Table, TableState},
};
use ratatui_image::protocol::Protocol;

use crate::{
    queue::QueueManager,
    task::THUMB_SIZE,
    ui::track::{render_visible_thumbnails, track_to_compact_row},
};

pub enum QueueAction {
    Play(usize), // Play immediately
    Remove(usize),
    MoveTrackUp(usize),
    MoveTrackDown(usize),
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

        // num of chars available for text in the card
        let text_width = inner
            .width
            .saturating_sub(THUMB_SIZE.width + 1) as usize; // + 1 for column spacing

        let rows: Vec<Row> = queue
            .user_queue
            .iter()
            .map(|track| track_to_compact_row(track, text_width))
            .collect();

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
            inner.x,
            inner,
            thumbnails,
            visible,
            buf,
        );
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent) -> Option<QueueAction> {
        match key_event.code {
            KeyCode::Char('J') => self
                .state
                .selected()
                .map(QueueAction::MoveTrackDown),
            KeyCode::Down
                if key_event
                    .modifiers
                    .contains(KeyModifiers::SHIFT) =>
            {
                self.state
                    .selected()
                    .map(QueueAction::MoveTrackDown)
            },
            KeyCode::Char('K') => self
                .state
                .selected()
                .map(QueueAction::MoveTrackUp),
            KeyCode::Up
                if key_event
                    .modifiers
                    .contains(KeyModifiers::SHIFT) =>
            {
                self.state
                    .selected()
                    .map(QueueAction::MoveTrackUp)
            },
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
