mod audio;
mod library;
mod task;
mod ui;
mod util;
mod waveform;

use std::{
    sync::mpsc::{self, Receiver, RecvTimeoutError},
    thread,
    time::Duration,
};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout},
};

use crate::{
    audio::AudioPlayer,
    library::LibraryState,
    task::TaskManager,
    ui::{
        input::{InputAction, TextInput},
        library::LibraryWidget,
        transport::TransportState,
    },
    waveform::WaveformData,
};

pub enum ActiveView {
    Main,
    Search,
}

pub enum AppEvent {
    Input(Event),
    Waveform(usize, Box<WaveformData>), // box for clippy warning about large size difference
}

pub struct App {
    active_view: ActiveView,
    transport: TransportState,
    library: LibraryState,
    library_widget: LibraryWidget,
    player: AudioPlayer,
    search: TextInput,
    tasks: TaskManager,
    waveform: Option<WaveformData>,
    events: Receiver<AppEvent>,
    quit: bool,
}

impl App {
    /// we use it only for elapsed time (and playhead)
    const TICK: Duration = Duration::from_millis(250);

    fn new() -> Self {
        let library = LibraryState::new();
        let library_widget = LibraryWidget::new(&library);

        // terminal gets its own thread, so main loop can block on a single channel and wake on
        // keypress or finished task
        let (events_tx, events_rx) = mpsc::channel();

        let input_tx = events_tx.clone();
        thread::spawn(move || {
            while let Ok(event) = event::read() {
                if input_tx
                    .send(AppEvent::Input(event))
                    .is_err()
                {
                    break;
                }
            }
        });

        Self {
            active_view: ActiveView::Main,
            quit: false,
            library,
            library_widget,
            player: AudioPlayer::new().expect("Could not create audio player"),
            search: TextInput::new("Search", "Press '/' to search..."),
            tasks: TaskManager::new(events_tx),
            events: events_rx,
            waveform: None,
            transport: TransportState::default(),
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> color_eyre::Result<()> {
        while !self.quit {
            terminal.draw(|frame| self.draw(frame))?;

            match self
                .events
                .recv_timeout(Self::TICK)
            {
                Ok(AppEvent::Input(event)) => self.handle_event(event),
                Ok(AppEvent::Waveform(generation, data)) => {
                    if self.tasks.is_current(generation) {
                        self.waveform = Some(*data);
                    }
                },
                Err(RecvTimeoutError::Timeout) => {},
                Err(RecvTimeoutError::Disconnected) => self.quit = true,
            }
        }

        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        let [search_area, main_area, transport_area] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(6),
        ])
        .areas(frame.area());

        self.search
            .render(frame, search_area);

        self.library_widget
            .render(&self.library, main_area, frame.buffer_mut());

        self.transport.render(
            &self.player,
            self.waveform.as_ref(),
            transport_area,
            frame.buffer_mut(),
        );
    }

    fn handle_event(&mut self, event: Event) {
        if let Event::Key(key_event) = event
            && key_event.kind == KeyEventKind::Press
        {
            self.handle_key_event(key_event);
        }
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
                    if let Some(track) = self.library.tracks.get(index)
                        && self.player.play(track).is_ok()
                    {
                        self.waveform = None;
                        self.tasks.set_track(&track.path);
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
