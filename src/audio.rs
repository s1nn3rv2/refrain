use std::{fs::File, sync::mpsc::Sender, time::Duration};

use color_eyre::eyre::Context;
use mpris_server::PlaybackStatus;
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player};

use crate::{app::AppEvent, library::Track, mpris::MprisManager};

pub struct AudioPlayer {
    _device_sink: MixerDeviceSink,
    player: Player,
    current_track: Option<Track>,
    next: Option<Track>, // appended to rodio, but not playing yet
    mpris: MprisManager,
}

impl AudioPlayer {
    pub fn new(events_tx: Sender<AppEvent>) -> color_eyre::Result<Self> {
        let device_sink =
            DeviceSinkBuilder::open_default_sink().wrap_err("Failed to open audio device")?;

        let player = Player::connect_new(device_sink.mixer());

        Ok(Self {
            _device_sink: device_sink,
            player,
            current_track: None,
            next: None,
            mpris: MprisManager::new(events_tx),
        })
    }

    pub fn load(&mut self, track: &Track) -> color_eyre::Result<()> {
        let file = File::open(&track.path)
            .wrap_err_with(|| format!("Failed to open file: {:?}", track.path))?;

        // if you pass in a BufReader instead of directly the file, seeking backwards doesn't seem
        // to work lol, so keep that in mind
        let source = Decoder::try_from(file).wrap_err("Failed to decode audio file")?;

        self.player.clear();
        self.next = None;
        self.player.append(source);

        self.current_track = Some(track.clone());
        self.mpris.set_track(Some(track));
        self.sync_mpris_status();
        Ok(())
    }

    pub fn play(&mut self, track: &Track) -> color_eyre::Result<()> {
        self.load(track)?;
        self.player.play();
        self.sync_mpris_status();
        Ok(())
    }

    pub fn resume_pause(&mut self) {
        if self.current_track.is_none() {
            return;
        }
        if !self.player.is_paused() {
            self.player.pause();
        } else {
            self.player.play();
        }
        self.sync_mpris_status();
    }

    pub fn seek(&self, position: Duration) -> color_eyre::Result<()> {
        self.player
            .try_seek(position)
            .wrap_err("Failed to seek audio")?;
        self.mpris.seeked(position);
        Ok(())
    }

    pub fn current_track(&self) -> Option<&Track> {
        self.current_track.as_ref()
    }

    pub fn elapsed(&self) -> Duration {
        self.player.get_pos()
    }

    pub fn is_finished(&self) -> bool {
        self.current_track.is_some() && self.player.empty()
    }

    pub fn is_paused(&self) -> bool {
        self.player.is_paused()
    }

    pub fn stop(&mut self) {
        self.player.stop();
        self.next = None;
        self.current_track = None;
        self.mpris.set_track(None);
        self.mpris
            .set_status(PlaybackStatus::Stopped);
    }

    pub fn refresh_if_current(&mut self, track: &Track) {
        if let Some(current) = &self.current_track
            && current.path == track.path
        {
            self.current_track = Some(track.clone());
            self.mpris.set_track(Some(track));
        }
    }

    // only preload at last 5 seconds of the song ending
    pub fn needs_preload(&self) -> bool {
        let Some(track) = &self.current_track else {
            return false;
        };
        self.next.is_none()
            && track
                .length
                .saturating_sub(self.elapsed())
                < Duration::from_secs(5)
    }

    // this is used for gapless playback, rodio already supports it if you append
    pub fn preload(&mut self, track: &Track) -> color_eyre::Result<()> {
        let file = File::open(&track.path)
            .wrap_err_with(|| format!("Failed to open file: {:?}", track.path))?;
        let source = Decoder::try_from(file).wrap_err("Failed to decode audio file")?;
        self.player.append(source);
        self.next = Some(track.clone());
        Ok(())
    }

    pub fn poll_transition(&mut self) -> bool {
        if self.next.is_none() || self.player.len() != 1 {
            return false;
        }
        self.current_track = self.next.take();
        self.mpris
            .set_track(self.current_track.as_ref());
        self.sync_mpris_status();
        true
    }

    pub fn take_next(&mut self) -> Option<Track> {
        self.next.take()
    }

    pub fn has_next(&self) -> bool {
        self.next.is_some()
    }

    fn sync_mpris_status(&self) {
        let status = if self.current_track.is_none() {
            PlaybackStatus::Stopped
        } else if self.player.is_paused() {
            PlaybackStatus::Paused
        } else {
            PlaybackStatus::Playing
        };
        self.mpris.set_status(status);
    }

    pub fn sync_mpris_position(&self) {
        self.mpris
            .set_position(self.elapsed());
    }

    pub fn sync_mpris_can_go_next(&self, can_go_next: bool) {
        self.mpris
            .set_can_go_next(can_go_next);
    }

    pub fn sync_mpris_shuffle(&self, shuffle: bool) {
        self.mpris.set_shuffle(shuffle);
    }
}
