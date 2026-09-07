mod audio;
mod library;
mod ui;
mod util;

use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout},
};

use crate::{
    audio::AudioPlayer,
    library::LibraryState,
    ui::{
        input::{InputAction, TextInput},
        library::LibraryWidgetState,
        transport::TransportState,
    },
};

pub enum ActiveView {
    Main,
    Search,
}

pub struct App {
    active_view: ActiveView,
    transport: TransportState,
    library: LibraryState,
    library_widget: LibraryWidgetState,
    player: AudioPlayer,
    search: TextInput,
    quit: bool,
}

impl App {
    fn new() -> Self {
        let library = LibraryState::new();
        let library_widget = LibraryWidgetState::new(&library);

        Self {
            active_view: ActiveView::Main,
            quit: false,
            library,
            library_widget,
            player: AudioPlayer::new().expect("Could not create audio player"),
            search: TextInput::new("Search", "Press '/' to search..."),
            transport: TransportState::default(),
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> color_eyre::Result<()> {
        while !self.quit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }

        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        let [search_area, main_area, transport_area] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(3),
        ])
        .areas(frame.area());

        self.search
            .render(frame, search_area);

        self.library_widget
            .render(&self.library, main_area, frame.buffer_mut());

        self.transport
            .render(&self.player, transport_area, frame.buffer_mut());
    }

    fn handle_events(&mut self) -> color_eyre::Result<()> {
        if event::poll(Duration::from_millis(250))? {
            match event::read()? {
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    self.handle_key_event(key_event)
                },
                _ => {},
            }
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match self.active_view {
            ActiveView::Search => {
                match self
                    .search
                    .handle_key_event(key_event)
                {
                    InputAction::Changed => {
                        // filter tracks
                        self.library_widget
                            .update_filter(&self.library, &self.search.value);
                    },
                    InputAction::Submitted | InputAction::Escaped => {
                        // go back to main view
                        self.active_view = ActiveView::Main
                    },
                    _ => {},
                }
            },
            ActiveView::Main => match key_event.code {
                KeyCode::Char('q') => self.exit(),
                KeyCode::Char('p') => self.player.resume_pause(),
                KeyCode::Char('r') => {
                    let _ = self.library.scan();
                    self.library_widget
                        .update_filter(&self.library, &self.search.value);
                },
                KeyCode::Char('/') => {
                    self.active_view = ActiveView::Search;
                    self.search.focus();
                },
                _ if let Some(index) = self
                    .library_widget
                    .handle_key_event(key_event) =>
                {
                    if let Some(track) = self.library.tracks.get(index) {
                        let _ = self.player.play(track);
                    }
                },
                _ => {},
            },
        }
    }

    fn exit(&mut self) {
        self.quit = true;
    }
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    ratatui::run(|terminal| App::new().run(terminal))
}
