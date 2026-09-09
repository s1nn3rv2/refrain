mod audio;
mod cover;
mod library;
mod task;
mod ui;
mod util;
mod waveform;

use std::{
    num::NonZeroUsize,
    path::PathBuf,
    sync::mpsc::{self, Receiver, RecvTimeoutError},
    thread,
    time::Duration,
};

use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
};
use lru::LruCache;
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout, Size},
};
use ratatui_image::{picker::Picker, protocol::Protocol};

use crate::{
    audio::AudioPlayer,
    library::LibraryState,
    task::TaskManager,
    ui::{
        input::{InputAction, TextInput},
        library::LibraryWidget,
        transport::{TransportAction, TransportState},
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
    Cover(usize, Option<Protocol>),     // cover art in transport
    Thumbnail(PathBuf, Size, Option<Protocol>), // cover arts in library
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
    cover: Option<Protocol>, // transport cover art
    // Why do we keep size here? A compact mode or a grid viewer could be added, where image size
    // will be different
    thumbnails: LruCache<PathBuf, Option<(Size, Protocol)>>, // holds up to 128 protocols
    // contains all visible tracks (tracks that need cover arts to be loaded)
    visible: Vec<(PathBuf, Size)>,
    events: Receiver<AppEvent>,
    quit: bool,
}

impl App {
    /// we use it only for elapsed time (and playhead)
    const TICK: Duration = Duration::from_millis(250);

    fn new(picker: Picker) -> Self {
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
            tasks: TaskManager::new(picker, events_tx),
            cover: None,
            thumbnails: LruCache::new(NonZeroUsize::new(128).unwrap()),
            visible: Vec::new(),
            events: events_rx,
            waveform: None,
            transport: TransportState::default(),
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> color_eyre::Result<()> {
        while !self.quit {
            terminal.draw(|frame| self.draw(frame))?;

            // empty visible and pass all visible songs to request_covers
            // we empty visible because we recreate it on the next frame
            self.tasks
                .request_covers(std::mem::take(&mut self.visible));

            // sleep until event arrives (or 250ms passes)
            match self
                .events
                .recv_timeout(Self::TICK)
            {
                Ok(event) => {
                    self.apply(event);
                    // check if there are any other events piled up, if so, process them in that
                    // frame too
                    while let Ok(event) = self.events.try_recv() {
                        self.apply(event);
                    }
                },
                Err(RecvTimeoutError::Timeout) => {},
                Err(RecvTimeoutError::Disconnected) => self.quit = true,
            }
        }

        Ok(())
    }

    /// Receives background worker outputs and updates app state
    fn apply(&mut self, event: AppEvent) {
        match event {
            AppEvent::Input(event) => self.handle_event(event),
            AppEvent::Waveform(generation, data) => {
                if self.tasks.is_current(generation) {
                    self.waveform = Some(*data);
                }
            },
            AppEvent::Cover(generation, protocol) => {
                if self.tasks.is_current(generation) {
                    self.cover = protocol;
                }
            },
            AppEvent::Thumbnail(path, size, protocol) => {
                self.thumbnails
                    .put(path, protocol.map(|protocol| (size, protocol)));
            },
        }
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

        self.library_widget.render(
            &self.library,
            &mut self.thumbnails,
            &mut self.visible,
            main_area,
            frame.buffer_mut(),
        );

        self.transport.render(
            &self.player,
            self.waveform.as_ref(),
            self.cover.as_ref(),
            transport_area,
            frame.buffer_mut(),
        );
    }

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event);
            },
            Event::Mouse(mouse_event)
                if let Some(TransportAction::Seek(progress)) = self
                    .transport
                    .handle_mouse_event(mouse_event) =>
            {
                if let Some(track) = self.player.current_track() {
                    let target_time = track.length.mul_f64(progress);
                    let _ = self.player.seek(target_time);
                }
            },
            _ => {},
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
                        self.cover = None;
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

    let picker = Picker::from_query_stdio()?;

    // enable mouse capture
    crossterm::execute!(std::io::stdout(), EnableMouseCapture)?;

    let res = ratatui::run(|terminal| App::new(picker).run(terminal));

    // disable mouse capture on exit
    let _ = crossterm::execute!(std::io::stdout(), DisableMouseCapture);

    res
}
