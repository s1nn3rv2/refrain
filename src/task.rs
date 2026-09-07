use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    thread,
};

use crate::waveform::{self, WaveformData};

pub enum TaskResult {
    Waveform(WaveformData),
}

type Request = (PathBuf, usize);

pub struct TaskManager {
    req_tx: Sender<Request>,
    res_rx: Receiver<(usize, TaskResult)>,
    generation: Arc<AtomicUsize>,
}

impl TaskManager {
    pub fn new() -> Self {
        let (req_tx, req_rx) = mpsc::channel::<Request>();
        let (res_tx, res_rx) = mpsc::channel::<(usize, TaskResult)>();

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
                    let _ = res_tx.send((generation, TaskResult::Waveform(data)));
                }
            }
        });

        Self {
            req_tx,
            res_rx,
            generation,
        }
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

    // everything that finished since last frame (doesnt include tracks that user has skipped past)
    pub fn poll(&self) -> Vec<TaskResult> {
        let current = self
            .generation
            .load(Ordering::Relaxed);

        self.res_rx
            .try_iter()
            .filter(|(generation, _)| *generation == current)
            .map(|(_, result)| result)
            .collect()
    }
}
