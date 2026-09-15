use std::{sync::mpsc::Sender, thread, time::Duration};

use mpris_server::{Metadata, PlaybackStatus, Player, Time, TrackId};

use crate::{AppEvent, cover, library::Track, util};

pub enum MprisAction {
    Play,
    Pause,
    PlayPause,
    Next,
    Stop,
    Quit,
    Seek(i64),
    SetPosition(Duration),
}

enum MprisUpdate {
    Track(Option<Track>),
    Status(PlaybackStatus),
    Position(Duration),
    Seeked(Duration),
    CanGoNext(bool),
}

pub struct MprisManager {
    update_tx: async_channel::Sender<MprisUpdate>,
}

impl MprisManager {
    pub fn new(events_tx: Sender<AppEvent>) -> Self {
        let (update_tx, update_rx) = async_channel::unbounded();

        thread::spawn(move || {
            mpris_server::zbus::block_on(async {
                let Ok(player) = Player::builder("refrain")
                    .identity("Refrain")
                    .desktop_entry("refrain")
                    .can_quit(true)
                    .can_play(true)
                    .can_pause(true)
                    .can_seek(true)
                    .can_go_next(false)
                    .can_go_previous(false) // not yet added
                    .shuffle(false) // disable shuffle and loop for now until its added
                    .loop_status(mpris_server::LoopStatus::None)
                    .build()
                    .await
                else {
                    return;
                };

                // uh this can surely be done better but im lazy
                let tx = events_tx.clone();
                player.connect_quit(move |_| {
                    let _ = tx.send(AppEvent::Mpris(MprisAction::Quit));
                });

                let tx = events_tx.clone();
                player.connect_play_pause(move |_| {
                    let _ = tx.send(AppEvent::Mpris(MprisAction::PlayPause));
                });

                let tx = events_tx.clone();
                player.connect_next(move |_| {
                    let _ = tx.send(AppEvent::Mpris(MprisAction::Next));
                });

                let tx = events_tx.clone();
                player.connect_play(move |_| {
                    let _ = tx.send(AppEvent::Mpris(MprisAction::Play));
                });

                let tx = events_tx.clone();
                player.connect_pause(move |_| {
                    let _ = tx.send(AppEvent::Mpris(MprisAction::Pause));
                });

                let tx = events_tx.clone();
                player.connect_stop(move |_| {
                    let _ = tx.send(AppEvent::Mpris(MprisAction::Stop));
                });

                let tx = events_tx.clone();
                player.connect_seek(move |_, offset| {
                    let _ = tx.send(AppEvent::Mpris(MprisAction::Seek(offset.as_micros())));
                });

                let tx = events_tx.clone();
                player.connect_set_position(move |player, trackid, position| {
                    // check if the track is still the same one
                    if player
                        .metadata()
                        .trackid()
                        .as_ref()
                        == Some(trackid)
                        && !position.is_negative()
                    {
                        let micros = position.as_micros() as u64;
                        let _ = tx.send(AppEvent::Mpris(MprisAction::SetPosition(
                            Duration::from_micros(micros),
                        )));
                    }
                });

                // we gotta listen to both d-bus and our app
                futures_lite::future::or(player.run(), async {
                    while let Ok(msg) = update_rx.recv().await {
                        match msg {
                            MprisUpdate::Track(Some(track)) => {
                                let id = format!(
                                    "/org/refrain/track/{:016x}",
                                    util::fnv1a(
                                        track
                                            .path
                                            .as_os_str()
                                            .as_encoded_bytes()
                                    )
                                );

                                let mut metadata = Metadata::builder()
                                    .title(&track.tags.title)
                                    .artist(track.tags.individual_artists())
                                    .length(Time::from_micros(track.length.as_micros() as i64));

                                if let Ok(trackid) = TrackId::try_from(id) {
                                    metadata = metadata.trackid(trackid);
                                }

                                if let Some(album) = &track.tags.album {
                                    metadata = metadata.album(album);
                                }

                                if let Some(cover) = cover::cached_file(&track.path) {
                                    metadata =
                                        metadata.art_url(format!("file://{}", cover.display()));
                                }

                                let _ = player
                                    .set_metadata(metadata.build())
                                    .await;
                                let _ = player.set_can_pause(true).await;
                                let _ = player.set_can_seek(true).await;
                            },
                            MprisUpdate::Track(None) => {
                                let _ = player
                                    .set_metadata(Metadata::new())
                                    .await;
                                player.set_position(Time::ZERO);
                                let _ = player.set_can_pause(false).await;
                                let _ = player.set_can_seek(false).await;
                            },
                            MprisUpdate::Status(status) => {
                                let _ = player
                                    .set_playback_status(status)
                                    .await;
                            },
                            MprisUpdate::Position(position) => {
                                player.set_position(Time::from_micros(position.as_micros() as i64));
                            },
                            MprisUpdate::Seeked(position) => {
                                let time = Time::from_micros(position.as_micros() as i64);
                                player.set_position(time);
                                let _ = player.seeked(time).await;
                            },
                            MprisUpdate::CanGoNext(can_go_next) => {
                                let _ = player
                                    .set_can_go_next(can_go_next)
                                    .await;
                            },
                        }
                    }
                })
                .await;
            });
        });

        Self { update_tx }
    }

    pub fn set_track(&self, track: Option<&Track>) {
        let _ = self
            .update_tx
            .send_blocking(MprisUpdate::Track(track.cloned()));
    }

    pub fn set_status(&self, status: PlaybackStatus) {
        let _ = self
            .update_tx
            .send_blocking(MprisUpdate::Status(status));
    }

    pub fn set_position(&self, position: Duration) {
        let _ = self
            .update_tx
            .send_blocking(MprisUpdate::Position(position));
    }

    pub fn seeked(&self, position: Duration) {
        let _ = self
            .update_tx
            .send_blocking(MprisUpdate::Seeked(position));
    }

    pub fn set_can_go_next(&self, can_go_next: bool) {
        let _ = self
            .update_tx
            .send_blocking(MprisUpdate::CanGoNext(can_go_next));
    }
}
