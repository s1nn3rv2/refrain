use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicUsize, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    thread,
};

use ratatui::layout::Size;
use ratatui_image::{Resize, picker::Picker, protocol::Protocol};

use crate::{
    AppEvent, cover,
    waveform::{self},
};

// char cell is twice as tall as it is wide (about)
// I give 1 more width because the ratio can get wonky sometimes
pub const COVER_SIZE: Size = Size::new(11, 5);
pub const THUMB_SIZE: Size = Size::new(7, 3);

/// Spawns `n` many threads dedicated to loading images
const COVER_WORKERS: usize = 2;

type Request = (PathBuf, usize);

struct Wanted {
    /// We use VecDeque as a queue, holding path to a track and the size (terminal character
    /// width/height) of it. We need Size because we keep two different image types: current playing
    /// (which is bigger, called COVER_SIZE) and thumbnails, which are smaller
    /// Mutex because only one thread should modify it
    ///
    /// We use VecDeque since we'll never need a slice
    requests: Mutex<VecDeque<(PathBuf, Size)>>,
    /// Signal, if queue is empty -> thread goes to sleep, if UI adds any new requests, wake workers
    /// up
    cv: Condvar,
}

impl Wanted {
    /// Pops the next thumbnail to process
    fn next(&self) -> (PathBuf, Size) {
        let mut requests = self.requests.lock().unwrap();

        loop {
            if let Some(request) = requests.pop_front() {
                return request;
            }
            // Queue is empty - there are no more thumbnails to process, put thread to sleep and unlock
            requests = self.cv.wait(requests).unwrap();
        }
    }

    /// Overwrites queue with currently visible items and wakes workers up, usually called on scroll
    /// or UI refresh
    fn replace(&self, new: VecDeque<(PathBuf, Size)>) {
        *self.requests.lock().unwrap() = new;
        self.cv.notify_all(); // wake up all sleeping workers
    }
}

pub struct TaskManager {
    track_tx: Sender<Request>,
    wanted: Arc<Wanted>,
    generation: Arc<AtomicUsize>,
}

impl TaskManager {
    pub fn new(picker: Picker, events_tx: Sender<AppEvent>) -> Self {
        let (track_tx, track_rx) = mpsc::channel::<Request>();

        let generation = Arc::new(AtomicUsize::new(0));
        let wanted = Arc::new(Wanted {
            requests: Mutex::new(VecDeque::new()),
            cv: Condvar::new(),
        });

        track_worker(
            picker.clone(),
            events_tx.clone(),
            track_rx,
            Arc::clone(&generation),
        );

        for _ in 0..COVER_WORKERS {
            cover_worker(picker.clone(), events_tx.clone(), Arc::clone(&wanted));
        }

        Self {
            track_tx,
            wanted,
            generation,
        }
    }

    pub fn set_track(&self, path: &Path) {
        let generation = self
            .generation
            .fetch_add(1, Ordering::Relaxed)
            + 1;

        let _ = self
            .track_tx
            .send((path.to_path_buf(), generation));
    }

    // call once per frame for track rows that still need building
    pub fn request_covers(&self, requests: Vec<(PathBuf, Size)>) {
        self.wanted
            .replace(requests.into());
    }

    pub fn is_current(&self, generation: usize) -> bool {
        generation
            == self
                .generation
                .load(Ordering::Relaxed)
    }
}

/// Takes a picker, path and size and builds into a full protocol using ratatui-image ready to display on the terminnal
fn build(picker: &Picker, path: &Path, size: Size) -> Option<Protocol> {
    let image = cover::load_or_extract(path)?;

    picker
        .new_protocol(image, size, Resize::Fit(None))
        .ok()
}

/// Worker taking care of current playing track cover art in transport
/// Track as in Song, not "keep track of"
fn track_worker(
    picker: Picker,
    events_tx: Sender<AppEvent>,
    track_rx: Receiver<Request>,
    current: Arc<AtomicUsize>,
) {
    thread::spawn(move || {
        // Wait for a song to play, put to sleep until we get a request
        while let Ok((mut path, mut generation)) = track_rx.recv() {
            // If we have more requests queued up, discard every previous one and immediately start
            // working on this one
            while let Ok((newer_path, newer_generation)) = track_rx.try_recv() {
                path = newer_path;
                generation = newer_generation;
            }

            // Check if request is still valid
            if generation != current.load(Ordering::Relaxed) {
                continue;
            }

            // cover first, as getting it is faster than waveform
            // means we don't have to wait for waveform to finish loading, we should always put the
            // faster operation first so we don't have to wait on the other one
            let protocol = build(&picker, &path, COVER_SIZE);

            if generation == current.load(Ordering::Relaxed) {
                let _ = events_tx.send(AppEvent::Cover(generation, protocol));
            }

            let Some(data) = waveform::load_or_compute(&path) else {
                continue;
            };

            // Finally, send waveform
            if generation == current.load(Ordering::Relaxed) {
                let _ = events_tx.send(AppEvent::Waveform(generation, Box::new(data)));
            }
        }
    });
}

/// Worker taking care of many thumbnails in library
fn cover_worker(picker: Picker, events_tx: Sender<AppEvent>, wanted: Arc<Wanted>) {
    thread::spawn(move || {
        loop {
            // Ask worker for next thumbnail that needs building
            let (path, size) = wanted.next();

            let protocol = build(&picker, &path, size);

            // send finished thumbnail back to main loop
            if events_tx
                .send(AppEvent::Thumbnail(path, size, protocol))
                .is_err()
            // in case main loop quit, break out of loop and shut down cleanly
            {
                break;
            }
        }
    });
}
