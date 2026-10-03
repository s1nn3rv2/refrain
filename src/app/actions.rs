use std::time::Duration;

use crate::{app::App, library::Track, mpris::MprisAction, ui::tag_editor::TagEditor};

impl App {
    pub fn handle_mpris(&mut self, action: MprisAction) {
        match action {
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
                self.stop();
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
        }
    }

    pub fn library_play(&mut self, idx: usize) {
        if let Some(track) = self
            .library
            .tracks
            .get(idx)
            .cloned()
        {
            self.play_track(track);
        }
    }

    pub fn library_add_to_queue(&mut self, idx: usize) {
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

    pub fn library_add_all_to_queue(&mut self) {
        let mut tracks = self
            .library_widget
            .filtered_indices
            .iter()
            .filter_map(|&idx| {
                self.library
                    .tracks
                    .get(idx)
                    .cloned()
            })
            .collect::<Vec<Track>>()
            .into_iter();

        if self
            .player
            .current_track()
            .is_none()
            && let Some(first) = tracks.next()
        {
            self.play_track(first);
        }

        self.queue
            .user_queue
            .extend(tracks);
        self.sync_mpris_queue_state();
    }

    pub fn library_play_next(&mut self, idx: usize) {
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

    pub fn library_edit_tags(&mut self, idx: usize) {
        if let Some(track) = self.library.tracks.get(idx) {
            self.tag_editor = Some(TagEditor::new(idx, track));
        }
    }

    pub fn queue_play(&mut self, idx: usize) {
        // remove track from queue and play it
        if let Some(track) = self.queue.user_queue.remove(idx) {
            self.play_track(track);
            self.sync_mpris_queue_state();
        }
    }

    pub fn queue_remove(&mut self, idx: usize) {
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

    pub fn queue_clear(&mut self) {
        self.queue.user_queue.clear();
        self.queue_widget
            .state
            .select(None);
        self.sync_mpris_queue_state();
    }

    pub fn queue_move_track_up(&mut self, idx: usize) {
        if idx > 0 && idx < self.queue.user_queue.len() {
            self.queue
                .user_queue
                .swap(idx, idx - 1);
            self.queue_widget
                .state
                .select(Some(idx - 1));
        }
    }

    pub fn queue_move_track_down(&mut self, idx: usize) {
        if idx + 1 < self.queue.user_queue.len() {
            self.queue
                .user_queue
                .swap(idx, idx + 1);
            self.queue_widget
                .state
                .select(Some(idx + 1));
        }
    }

    pub fn tag_editor_save_edited_tags(&mut self) -> color_eyre::Result<()> {
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

        track.save_tags(editor.to_tags())?;

        self.player
            .refresh_if_current(track);
        self.queue.refresh_track(track);

        let _ = self.library.save_cache();
        self.refresh_filter();

        Ok(())
    }

    // --

    pub fn play_track(&mut self, track: Track) {
        if self.player.play(&track).is_ok() {
            self.waveform = None;
            self.cover = None;
            self.tasks.set_track(&track.path);
        }
    }

    pub fn play_next_track(&mut self) {
        if let Some(next_track) = self.queue.pop_next() {
            self.play_track(next_track);
        } else {
            self.stop();
        }
        self.sync_mpris_queue_state();
    }

    /// Stops playback and clears old track waveform and cover from transport
    pub fn stop(&mut self) {
        self.player.stop();
        self.waveform = None;
        self.cover = None;
    }

    pub fn exit(&mut self) {
        self.quit = true;
    }

    pub fn check_track_finished(&mut self) {
        if self.player.is_finished() {
            self.play_next_track();
        }
    }

    pub fn sync_mpris_queue_state(&self) {
        let has_next = !self.queue.user_queue.is_empty();
        self.player
            .sync_mpris_can_go_next(has_next);
    }

    pub fn refresh_filter(&mut self) {
        self.library_widget
            .update_filter(&self.library, &self.search.input.value);
        self.sidebar.update_from_filtered(
            &self.library,
            &self
                .library_widget
                .filtered_indices,
        );
    }
}
