mod audio;
mod library;
mod ui;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout},
    style::Stylize,
    symbols::border,
    text::Line,
    widgets::Block,
};

use crate::{
    audio::AudioPlayer,
    library::LibraryState,
    ui::{library::LibraryWidgetState, transport::TransportState},
};

pub struct App {
    transport: TransportState,
    library: LibraryState,
    library_widget: LibraryWidgetState,
    player: AudioPlayer,
    quit: bool,
}

impl App {
    fn new() -> Self {
        Self {
            quit: false,
            library: LibraryState::new(),
            library_widget: LibraryWidgetState::default(),
            player: AudioPlayer::new().expect("Could not create audio player"),
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
        let [main_area, transport_area] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(3)]).areas(frame.area());

        let title = Line::from(" Test ".bold());
        let block = Block::bordered()
            .title(title.centered())
            .border_set(border::THICK);

        frame.render_widget(block, main_area);

        self.library_widget
            .render(&self.library, main_area, frame.buffer_mut());

        let current_track = self
            .player
            .current_track(&self.library);

        self.transport
            .render(current_track, transport_area, frame.buffer_mut());
    }

    fn handle_events(&mut self) -> color_eyre::Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            },
            _ => {},
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Char('p') => self.player.resume_pause(),
            _ if let Some(index) = self
                .library_widget
                .handle_key_event(key_event) =>
            {
                if let Some(track) = self.library.tracks.get(index) {
                    let _ = self.player.play(index, track);
                }
            },
            _ => {},
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
