use std::{fs::File, io::BufReader, time::Duration};

use color_eyre::eyre::Context;
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player};

use crate::library::Track;

// TODO: add MPRIS support
pub struct AudioPlayer {
    _device_sink: MixerDeviceSink,
    player: Player,
    current_track: Option<Track>,
}

impl AudioPlayer {
    pub fn new() -> color_eyre::Result<Self> {
        let device_sink = DeviceSinkBuilder::open_default_sink()
            .map_err(|e| color_eyre::eyre::eyre!("Failed to open audio device: {e}"))?;

        let player = Player::connect_new(device_sink.mixer());

        Ok(Self {
            _device_sink: device_sink,
            player,
            current_track: None,
        })
    }

    pub fn play(&mut self, track: &Track) -> color_eyre::Result<()> {
        let file = File::open(&track.path)
            .wrap_err_with(|| format!("Failed to open file: {:?}", track.path))?;
        let reader = BufReader::new(file);

        let source = Decoder::try_from(reader).wrap_err("Failed to decode audio file")?;

        self.player.stop();
        self.player.append(source);
        self.player.play();

        self.current_track = Some(track.clone());
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
    }

    pub fn current_track(&self) -> Option<&Track> {
        self.current_track.as_ref()
    }

    pub fn elapsed(&self) -> Duration {
        self.player.get_pos()
    }
}
