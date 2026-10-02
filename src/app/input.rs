use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};

use crate::{
    app::{ActiveView, App},
    ui::{
        input::InputAction, library::LibraryAction, queue::QueueAction, sidebar::SidebarAction,
        tag_editor::TagEditorAction, transport::TransportAction,
    },
};

impl App {
    pub fn handle_event(&mut self, event: Event) {
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

    pub fn handle_key_event(&mut self, key_event: KeyEvent) {
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
                    self.refresh_filter();
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
                self.refresh_filter();
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
                        self.refresh_filter();
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

    pub fn cycle_focus(&mut self, forward: bool) {
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

    /// Returns all visible views, for example if queue is currently hidden it only returns Sidebar
    /// and Library
    pub fn visible_views(&self) -> Vec<ActiveView> {
        let mut views = vec![ActiveView::Sidebar, ActiveView::Library];

        if self.show_queue {
            views.push(ActiveView::Queue);
        }

        views
    }

    // exists, because I want so when search input field is focused, I want the rest of the views to
    // not have the highlighted borders, to bring more attention that youre focused on the search
    // input field
    pub fn is_pane_focused(&self, view: ActiveView) -> bool {
        !self.search.input.is_focused && self.active_view == view
    }
}
