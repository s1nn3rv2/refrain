use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    symbols::border,
    widgets::{Block, Clear},
};

use crate::{library::Track, ui::input::TextInput};

pub enum TagEditorAction {
    Save,
    Cancel,
}

pub struct TagEditor {
    pub track_idx: usize, // stored so we know which song in library was edited
    pub title: TextInput,
    pub artists: TextInput,
    pub album: TextInput,
    pub album_artists: TextInput,
    pub track_number: TextInput,
    pub disc_number: TextInput,
    pub genre: TextInput,
    pub focused_field: usize,
}

impl TagEditor {
    const FIELD_COUNT: usize = 7;

    pub fn new(track_idx: usize, track: &Track) -> Self {
        let mut title = TextInput::new("").with_title("Title");
        title.value = track.title.clone();
        title.focus();

        let mut artists = TextInput::new("").with_title("Artists (separated with ;)");
        artists.value = track
            .unformatted_artists()
            .to_string();

        let mut album = TextInput::new("").with_title("Album");
        album.value = track
            .album
            .clone()
            .unwrap_or_default();

        let mut album_artists = TextInput::new("").with_title("Album artists");
        album_artists.value = track
            .unformatted_album_artists()
            .to_string();

        let mut track_number = TextInput::new("").with_title("Track Number");
        track_number.value = track
            .track_number
            .unwrap_or_default()
            .to_string();

        let mut disc_number = TextInput::new("").with_title("Disc Number");
        disc_number.value = track
            .disc_number
            .unwrap_or_default()
            .to_string();

        let mut genre = TextInput::new("").with_title("Genre");
        genre.value = track
            .genre
            .clone()
            .unwrap_or_default();

        Self {
            track_idx,
            title,
            artists,
            album,
            album_artists,
            track_number,
            disc_number,
            genre,
            focused_field: 0,
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> Option<TagEditorAction> {
        match key.code {
            KeyCode::Esc => Some(TagEditorAction::Cancel),
            KeyCode::Tab => {
                self.cycle_focus(true);
                None
            },
            KeyCode::BackTab => {
                self.cycle_focus(false);
                None
            },
            _ => {
                self.current_input_mut()
                    .handle_key_event(key);
                None
            },
        }
    }

    fn current_input_mut(&mut self) -> &mut TextInput {
        match self.focused_field {
            0 => &mut self.title,
            1 => &mut self.artists,
            2 => &mut self.album,
            3 => &mut self.album_artists,
            4 => &mut self.track_number,
            5 => &mut self.disc_number,
            _ => &mut self.genre,
        }
    }

    fn cycle_focus(&mut self, forward: bool) {
        self.current_input_mut().unfocus();

        self.focused_field = if forward {
            (self.focused_field + 1) % Self::FIELD_COUNT
        } else {
            (self.focused_field + Self::FIELD_COUNT - 1) % Self::FIELD_COUNT
        };

        self.current_input_mut().focus();
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let popup_area = area.centered(Constraint::Length(70), Constraint::Length(25));

        frame.render_widget(Clear, popup_area);

        let block = Block::bordered()
            .border_set(border::THICK)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" Edit Tags ");

        let inner = block.inner(popup_area);
        frame.render_widget(block, popup_area);

        let rows = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(inner);

        self.title.render(frame, rows[0]);
        self.artists.render(frame, rows[1]);
        self.album.render(frame, rows[2]);
        self.album_artists
            .render(frame, rows[3]);
        self.track_number
            .render(frame, rows[4]);
        self.disc_number
            .render(frame, rows[5]);
        self.genre.render(frame, rows[6]);
    }
}
