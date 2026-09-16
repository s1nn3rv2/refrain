mod audio;
mod config;
mod cover;
mod library;
mod mpris;
mod queue;
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
    config::Config,
    library::{LibraryState, Track, TrackTags},
    mpris::MprisAction,
    queue::QueueManager,
    task::TaskManager,
    ui::{
        input::InputAction,
        library::{LibraryAction, LibraryWidget},
        queue::{QueueAction, QueueWidget},
        search::SearchBar,
        sidebar::{SidebarAction, SidebarWidget},
        tag_editor::{TagEditor, TagEditorAction},
        transport::{TransportAction, TransportState},
    },
    waveform::WaveformData,
};

#[derive(PartialEq, Clone, Copy)]
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

            self.check_track_finished();
            self.player.sync_mpris_position();
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
            AppEvent::Mpris(action) => match action {
                MprisAction::PlayPause => self.player.resume_pause(),
                MprisAction::Next => self.play_next_track(),
                MprisAction::Play => {
                    if self.player.is_paused() {
                        self.player.resume_pause();
                    }
                },
                MprisAction::Pause => {
                    if !self.player.is_paused() {
                        self.player.resume_pause();
                    }
                },
                MprisAction::Stop => {
                    self.player.stop();
                    self.waveform = None;
                    self.cover = None;
                },
                MprisAction::Seek(offset) => {
                    if let Some(length) = self
                        .player
                        .current_track()
                        .map(|t| t.length)
                    {
                        let target = self.player.elapsed().as_micros() as i64 + offset;
                        if target >= length.as_micros() as i64 {
                            self.play_next_track();
                        } else {
                            let _ = self
                                .player
                                .seek(Duration::from_micros(target.max(0) as u64));
                        }
                    }
                },
                MprisAction::SetPosition(position) => {
                    if self
                        .player
                        .current_track()
                        .is_some_and(|t| position <= t.length)
                    {
                        let _ = self.player.seek(position);
                    }
                },
                MprisAction::Quit => self.exit(),
            },
        }
    }

    /// Returns all visible views, for example if queue is currently hidden it only returns Sidebar
    /// and Library
    fn visible_views(&self) -> Vec<ActiveView> {
        let mut views = vec![ActiveView::Sidebar, ActiveView::Library];

        if self.show_queue {
            views.push(ActiveView::Queue);
        }

        views
    }

    fn cycle_focus(&mut self, forward: bool) {
        let views = self.visible_views();
        if views.is_empty() {
            return;
        }

        let current_pos = views
            .iter()
            .position(|&v| v == self.active_view)
            .unwrap_or(0);

        // so it wraps around
        let next_pos = if forward {
            (current_pos + 1) % views.len()
        } else {
            (current_pos + views.len() - 1) % views.len()
        };

        self.active_view = views[next_pos];
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
        if let Some(editor) = &mut self.tag_editor {
            match editor.handle_key_event(key_event) {
                Some(TagEditorAction::Cancel) => self.tag_editor = None,
                Some(TagEditorAction::Save) => {
                    let _ = self.tag_editor_save_edited_tags();
                },
                None => {},
            }
            return;
        }

        // Search mode has exclusive keyobard input!
        if self.search.input.is_focused {
            match self
                .search
                .input
                .handle_key_event(key_event)
            {
                InputAction::Changed => {
                    self.library_widget
                        .update_filter(&self.library, &self.search.input.value);
                    self.sidebar.update_from_filtered(
                        &self.library,
                        &self
                            .library_widget
                            .filtered_indices,
                    );
                },
                InputAction::Submitted | InputAction::Escaped => {
                    self.search.input.unfocus();
                },
                _ => {},
            }
            return;
        }

        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Char('p') => self.player.resume_pause(),
            KeyCode::Char('r') => {
                let _ = self.library.scan();
                self.library_widget
                    .update_filter(&self.library, &self.search.input.value);
                self.sidebar.update_from_filtered(
                    &self.library,
                    &self
                        .library_widget
                        .filtered_indices,
                );
            },
            KeyCode::Char('n') | KeyCode::Char('>') => {
                self.play_next_track();
            },
            KeyCode::Char('u') => self.show_queue = !self.show_queue,
            KeyCode::Char('/') => {
                self.search.input.focus();
                return;
            },
            KeyCode::Tab => {
                self.cycle_focus(true);
                return;
            },
            KeyCode::BackTab => {
                self.cycle_focus(false);
                return;
            },
            _ => {},
        }

        match self.active_view {
            ActiveView::Sidebar => {
                match self
                    .sidebar
                    .handle_key_event(key_event)
                {
                    Some(SidebarAction::ApplyFilter { key, value }) => {
                        self.search
                            .set_tag(key, value.as_deref());
                        self.library_widget
                            .update_filter(&self.library, &self.search.input.value);
                        self.sidebar.update_from_filtered(
                            &self.library,
                            &self
                                .library_widget
                                .filtered_indices,
                        );
                    },
                    None => {},
                }
            },
            ActiveView::Library => {
                match self
                    .library_widget
                    .handle_key_event(key_event, &self.library)
                {
                    Some(LibraryAction::Play(idx)) => self.library_play(idx),
                    Some(LibraryAction::AddToQueue(idx)) => self.library_add_to_queue(idx),
                    Some(LibraryAction::PlayNext(idx)) => self.library_play_next(idx),
                    Some(LibraryAction::EditTags(idx)) => self.library_edit_tags(idx),
                    None => {},
                }
            },
            ActiveView::Queue => {
                match self
                    .queue_widget
                    .handle_key_event(key_event)
                {
                    Some(QueueAction::Play(idx)) => self.queue_play(idx),
                    Some(QueueAction::Remove(idx)) => self.queue_remove(idx),
                    Some(QueueAction::MoveTrackUp(idx)) => self.queue_move_track_up(idx),
                    Some(QueueAction::MoveTrackDown(idx)) => self.queue_move_track_down(idx),
                    None => {},
                }
            },
        }
    }

    // -- UI actions

    fn library_play(&mut self, idx: usize) {
        if let Some(track) = self
            .library
            .tracks
            .get(idx)
            .cloned()
        {
            self.play_track(track);
        }
    }

    fn library_add_to_queue(&mut self, idx: usize) {
        if let Some(track) = self
            .library
            .tracks
            .get(idx)
            .cloned()
        {
            if self
                .player
                .current_track()
                .is_none()
            {
                self.play_track(track);
            } else {
                self.queue.push_back(track);
                self.sync_mpris_queue_state();
            }
        }
    }

    fn library_play_next(&mut self, idx: usize) {
        if let Some(track) = self
            .library
            .tracks
            .get(idx)
            .cloned()
        {
            if self
                .player
                .current_track()
                .is_none()
            {
                self.play_track(track);
            } else {
                self.queue.push_front(track);
                self.sync_mpris_queue_state();
            }
        }
    }

    fn library_edit_tags(&mut self, idx: usize) {
        if let Some(track) = self.library.tracks.get(idx) {
            self.tag_editor = Some(TagEditor::new(idx, track));
        }
    }

    fn queue_play(&mut self, idx: usize) {
        // remove track from queue and play it
        if let Some(track) = self.queue.user_queue.remove(idx) {
            self.play_track(track);
            self.sync_mpris_queue_state();
        }
    }

    fn queue_remove(&mut self, idx: usize) {
        self.queue.user_queue.remove(idx);
        self.sync_mpris_queue_state();

        // clamp queue selection so cursor doesnt suddenly disappear
        let len = self.queue.user_queue.len();
        if len == 0 {
            self.queue_widget
                .state
                .select(None);
        } else if idx >= len {
            self.queue_widget
                .state
                .select(Some(len - 1));
        }
    }

    fn queue_move_track_up(&mut self, idx: usize) {
        if idx > 0 && idx < self.queue.user_queue.len() {
            self.queue
                .user_queue
                .swap(idx, idx - 1);
            self.queue_widget
                .state
                .select(Some(idx - 1));
        }
    }

    fn queue_move_track_down(&mut self, idx: usize) {
        if idx + 1 < self.queue.user_queue.len() {
            self.queue
                .user_queue
                .swap(idx, idx + 1);
            self.queue_widget
                .state
                .select(Some(idx + 1));
        }
    }

    fn tag_editor_save_edited_tags(&mut self) -> color_eyre::Result<()> {
        let Some(editor) = self.tag_editor.take() else {
            return Ok(());
        };
        let Some(track) = self
            .library
            .tracks
            .get_mut(editor.track_idx)
        else {
            return Ok(());
        };

        let album = (!editor
            .album
            .value
            .trim()
            .is_empty())
        .then_some(editor.album.value);
        let album_artists = (!editor
            .album_artists
            .value
            .trim()
            .is_empty())
        .then_some(editor.album_artists.value);
        let genre = (!editor
            .genre
            .value
            .trim()
            .is_empty())
        .then_some(editor.genre.value);
        let date = (!editor.date.value.trim().is_empty()).then_some(editor.date.value);
        let track_number = editor
            .track_number
            .value
            .trim()
            .parse::<u32>()
            .ok();
        let disc_number = editor
            .disc_number
            .value
            .trim()
            .parse::<u32>()
            .ok();

        track.save_tags(TrackTags {
            title: editor.title.value,
            artists: editor.artists.value,
            album,
            album_artists,
            track_number,
            disc_number,
            genre,
            date,
        })?;

        // update track in transport and queue too so tags are consistent
        self.player
            .refresh_if_current(track);
        self.queue.refresh_track(track);

        let _ = self.library.save_cache();

        self.library_widget
            .update_filter(&self.library, &self.search.input.value);
        self.sidebar.update_from_filtered(
            &self.library,
            &self
                .library_widget
                .filtered_indices,
        );

        Ok(())
    }

    // --

    fn play_track(&mut self, track: Track) {
        if self.player.play(&track).is_ok() {
            self.waveform = None;
            self.cover = None;
            self.tasks.set_track(&track.path);
        }
    }

    fn play_next_track(&mut self) {
        if let Some(next_track) = self.queue.pop_next() {
            self.play_track(next_track);
        } else {
            self.player.stop();
            self.waveform = None;
            self.cover = None;
        }
        self.sync_mpris_queue_state();
    }

    // exists, because I want so when search input field is focused, I want the rest of the views to
    // not have the highlighted borders, to bring more attention that youre focused on the search
    // input field
    fn is_pane_focused(&self, view: ActiveView) -> bool {
        !self.search.input.is_focused && self.active_view == view
    }

    fn exit(&mut self) {
        self.quit = true;
    }

    fn check_track_finished(&mut self) {
        if self.player.is_finished() {
            self.play_next_track();
        }
    }

    fn sync_mpris_queue_state(&self) {
        let has_next = !self.queue.user_queue.is_empty();
        self.player
            .sync_mpris_can_go_next(has_next);
    }
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let config = Config::load()?;
    Config::init(config);

    let picker = Picker::from_query_stdio()?;

    // enable mouse capture
    crossterm::execute!(std::io::stdout(), EnableMouseCapture)?;

    let res = ratatui::run(|terminal| App::new(picker).run(terminal));

    // disable mouse capture on exit
    let _ = crossterm::execute!(std::io::stdout(), DisableMouseCapture);

    res
}
