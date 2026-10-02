use std::{
    num::NonZeroUsize,
    path::PathBuf,
    sync::mpsc::{self, Receiver, RecvTimeoutError},
    thread,
    time::Duration,
};

use crossterm::event::{self, Event};
use lru::LruCache;
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout, Size},
};
use ratatui_image::{picker::Picker, protocol::Protocol};
use serde::{Deserialize, Serialize};

use crate::{
    audio::AudioPlayer,
    library::LibraryState,
    mpris::MprisAction,
    queue::QueueManager,
    state::State,
    task::TaskManager,
    ui::{
        library::LibraryWidget, queue::QueueWidget, search::SearchBar, sidebar::SidebarWidget,
        tag_editor::TagEditor, transport::TransportState,
    },
    waveform::WaveformData,
};

mod actions;
mod input;
mod persist;

#[derive(PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum ActiveView {
    Library,
    Sidebar,
    Queue,
}

pub enum AppEvent {
    Input(Event),
    Waveform(usize, Box<WaveformData>), // box for clippy warning about large size difference
    Cover(usize, Option<Protocol>),     // cover art in transport
    Thumbnail(PathBuf, Size, Option<Protocol>), // cover arts in library
    Mpris(MprisAction),
}

pub struct App {
    active_view: ActiveView,
    transport: TransportState,

    library: LibraryState,
    library_widget: LibraryWidget,

    tag_editor: Option<TagEditor>,

    sidebar: SidebarWidget,
    player: AudioPlayer,

    queue: QueueManager,
    queue_widget: QueueWidget,
    show_queue: bool,

    search: SearchBar,
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

    pub fn new(picker: Picker) -> Self {
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

        let mut app = Self {
            active_view: ActiveView::Library,
            quit: false,
            sidebar: SidebarWidget::new(&library),
            library,
            library_widget,
            tag_editor: None,
            player: AudioPlayer::new(events_tx.clone()).expect("Could not create audio player"),
            queue: QueueManager::new(),
            queue_widget: QueueWidget::new(),
            show_queue: true,
            search: SearchBar::new(),
            tasks: TaskManager::new(picker, events_tx.clone()),
            cover: None,
            thumbnails: LruCache::new(NonZeroUsize::new(128).unwrap()),
            visible: Vec::new(),
            events: events_rx,
            waveform: None,
            transport: TransportState::default(),
        };
        app.restore(State::load());
        app
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

            self.check_track_finished();
            self.player.sync_mpris_position();
        }

        let _ = self.to_state().save();

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
            AppEvent::Mpris(action) => self.handle_mpris(action),
        }
    }

    fn draw(&mut self, frame: &mut Frame) {
        let [search_area, main_area, transport_area] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(6),
        ])
        .areas(frame.area());

        // queue can show and hide, so we must split this into library_area (sidebar + library) and
        // queue_area
        let [sidebar_area, content_area] =
            Layout::horizontal([Constraint::Max(40), Constraint::Fill(1)]).areas(main_area);

        let (library_area, queue_area) = if self.show_queue {
            let [lib, q] =
                Layout::horizontal([Constraint::Fill(1), Constraint::Max(40)]).areas(content_area);
            (lib, Some(q))
        } else {
            (content_area, None)
        };

        self.sidebar.render(
            self.is_pane_focused(ActiveView::Sidebar),
            sidebar_area,
            frame.buffer_mut(),
        );

        self.search
            .input
            .render(frame, search_area);

        self.library_widget.render(
            self.is_pane_focused(ActiveView::Library),
            &self.library,
            &mut self.thumbnails,
            &mut self.visible,
            library_area,
            frame.buffer_mut(),
        );

        if let Some(area) = queue_area {
            self.queue_widget.render(
                self.is_pane_focused(ActiveView::Queue),
                &self.queue,
                &mut self.thumbnails,
                &mut self.visible,
                area,
                frame.buffer_mut(),
            );
        }

        self.transport.render(
            &self.player,
            self.waveform.as_ref(),
            self.cover.as_ref(),
            transport_area,
            frame.buffer_mut(),
        );

        if let Some(editor) = &mut self.tag_editor {
            editor.render(frame, frame.area());
        }
    }
}
