use std::{fs::File, sync::mpsc::Sender, time::Duration};

use color_eyre::eyre::Context;
use mpris_server::PlaybackStatus;
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player};

use crate::{AppEvent, library::Track, mpris::MprisManager};

pub struct AudioPlayer {
    _device_sink: MixerDeviceSink,
    player: Player,
    current_track: Option<Track>,
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
            mpris: MprisManager::new(events_tx),
        })
    }

    pub fn play(&mut self, track: &Track) -> color_eyre::Result<()> {
        let file = File::open(&track.path)
            .wrap_err_with(|| format!("Failed to open file: {:?}", track.path))?;

        // if you pass in a BufReader instead of directly the file, seeking backwards doesn't seem
        // to work lol, so keep that in mind
        let source = Decoder::try_from(file).wrap_err("Failed to decode audio file")?;

        self.player.stop();
        self.player.append(source);
        self.player.play();

        self.current_track = Some(track.clone());
        self.mpris.set_track(Some(track));
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
}
