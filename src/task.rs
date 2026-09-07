use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
        mpsc::{self, Sender},
    },
    thread,
};

use crate::{
    AppEvent,
    waveform::{self},
};

type Request = (PathBuf, usize);

pub struct TaskManager {
    req_tx: Sender<Request>,
    generation: Arc<AtomicUsize>,
}

impl TaskManager {
    pub fn new(events_tx: Sender<AppEvent>) -> Self {
        let (req_tx, req_rx) = mpsc::channel::<Request>();

        let generation = Arc::new(AtomicUsize::new(0));
        let current = Arc::clone(&generation);

        thread::spawn(move || {
            while let Ok((mut path, mut generation)) = req_rx.recv() {
                //Skip to newest request if more tracks were added to queue while were busy
                while let Ok((newer_path, newer_generation)) = req_rx.try_recv() {
                    path = newer_path;
                    generation = newer_generation;
                }

                if generation != current.load(Ordering::Relaxed) {
                    continue;
                }

                let Some(data) = waveform::load_or_compute(&path) else {
                    continue;
                };

                // decoding could take a while, so check again before sending it
                if generation == current.load(Ordering::Relaxed) {
                    let _ = events_tx.send(AppEvent::Waveform(generation, Box::new(data)));
                }
            }
        });

        Self { req_tx, generation }
    }

    pub fn set_track(&self, path: &Path) {
        let generation = self
            .generation
            .fetch_add(1, Ordering::Relaxed)
            + 1;

        let _ = self
            .req_tx
            .send((path.to_path_buf(), generation));
    }

    pub fn is_current(&self, generation: usize) -> bool {
        generation
            == self
                .generation
                .load(Ordering::Relaxed)
    }
}
