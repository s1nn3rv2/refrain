use std::{path::PathBuf, time::Duration};

use crate::{app::App, library::Track, state::State};

impl App {
    pub fn to_state(&self) -> State {
        State {
            current_track: self
                .player
                .current_track()
                .map(|t| t.path.clone()),
            position_ms: self.player.elapsed().as_millis() as u64,
            queue: self
                .queue
                .user_queue
                .iter()
                .map(|t| t.path.clone())
                .collect(),
            selected_track: self
                .library_widget
                .selected_track_index()
                .and_then(|idx| self.library.tracks.get(idx))
                .map(|t| t.path.clone()),
            search: self.search.input.value.clone(),
            sort_key: Some(self.library_widget.sort_key),
            sort_direction: Some(self.library_widget.sort_direction),
            show_queue: Some(self.show_queue),
            active_view: Some(self.active_view),
        }
    }

    pub fn restore(&mut self, state: State) {
        let find = |path: &PathBuf| {
            self.library
                .tracks
                .iter()
                .position(|t| &t.path == path)
        };
        let selected = state
            .selected_track
            .as_ref()
            .and_then(find);
        let queued: Vec<Track> = state
            .queue
            .iter()
            .filter_map(find)
            .map(|idx| self.library.tracks[idx].clone())
            .collect();
        let current = state
            .current_track
            .as_ref()
            .and_then(find)
            .map(|idx| self.library.tracks[idx].clone());

        self.show_queue = state
            .show_queue
            .unwrap_or(self.show_queue);
        if let Some(view) = state.active_view
            && self
                .visible_views()
                .contains(&view)
        {
            self.active_view = view;
        }

        if let Some(key) = state.sort_key {
            self.library_widget.sort_key = key;
        }
        if let Some(dir) = state.sort_direction {
            self.library_widget.sort_direction = dir;
        }
        self.search.input.value = state.search;
        self.search.input.character_index = self
            .search
            .input
            .value
            .chars()
            .count();

        self.refresh_filter();

        if let Some(idx) = selected {
            self.library_widget
                .select_track(idx);
        }

        self.queue
            .user_queue
            .extend(queued);
        self.sync_mpris_queue_state();

        if let Some(track) = current
            && self.player.load(&track).is_ok()
        {
            self.tasks.set_track(&track.path);
            let _ = self
                .player
                .seek(Duration::from_millis(state.position_ms));
        }
    }
}
