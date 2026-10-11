use std::collections::VecDeque;

use rand::seq::{IndexedRandom, SliceRandom};
use serde::{Deserialize, Serialize};

use crate::library::Track;

#[derive(Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum ShuffleMode {
    Off,
    Shuffle,
    Random,
    RandomAll,
}

impl ShuffleMode {
    pub fn next(self) -> Self {
        match self {
            Self::Off => Self::Shuffle,
            Self::Shuffle => Self::Random,
            Self::Random => Self::RandomAll,
            Self::RandomAll => Self::Off,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "shuffle off",
            Self::Shuffle => "shuffle",
            Self::Random => "random",
            Self::RandomAll => "random (all)",
        }
    }
}

pub struct QueueManager {
    // Track queued by the user to play first
    pub user_queue: VecDeque<Track>,
    // snapshot of filtered library view
    context: Vec<Track>,
    // last track played of context
    context_pos: usize,
    // only in shuffle mode, it's indices in context that are left to play in shuffle order
    shuffle_order: Vec<usize>,
    pub mode: ShuffleMode,
}

impl QueueManager {
    pub fn new() -> Self {
        Self {
            user_queue: VecDeque::new(),
            context: Vec::new(),
            context_pos: 0,
            shuffle_order: Vec::new(),
            mode: ShuffleMode::Off,
        }
    }

    // Add to queue
    pub fn push_back(&mut self, track: Track) {
        self.user_queue.push_back(track);
    }

    // Play next
    pub fn push_front(&mut self, track: Track) {
        self.user_queue.push_front(track);
    }

    pub fn set_context(&mut self, context: Vec<Track>, current: usize) {
        self.context = context;
        self.context_pos = current;
        self.reshuffle();
    }

    pub fn set_mode(&mut self, mode: ShuffleMode) {
        self.mode = mode;
        self.reshuffle();
    }

    pub fn pop_next(&mut self, library: &[Track]) -> Option<Track> {
        if let Some(track) = self.user_queue.pop_front() {
            return Some(track);
        }

        // user queue is out! now lets continue with context

        let next = match self.mode {
            ShuffleMode::RandomAll => {
                return library
                    .choose(&mut rand::rng())
                    .cloned();
            },
            ShuffleMode::Off => self.context_pos + 1,
            ShuffleMode::Shuffle => self.shuffle_order.pop()?,
            ShuffleMode::Random if self.context.is_empty() => return None,
            ShuffleMode::Random => rand::random_range(0..self.context.len()),
        };

        let track = self.context.get(next)?.clone();
        self.context_pos = next;
        Some(track)
    }

    pub fn has_next(&self, library: &[Track]) -> bool {
        !self.user_queue.is_empty()
            || match self.mode {
                ShuffleMode::Off => self.context_pos + 1 < self.context.len(),
                ShuffleMode::Shuffle => !self.shuffle_order.is_empty(),
                ShuffleMode::Random => !self.context.is_empty(),
                ShuffleMode::RandomAll => !library.is_empty(),
            }
    }

    /// Finds all occurences of track in queue and refreshes them with a new one
    pub fn refresh_track(&mut self, track: &Track) {
        for queued in self
            .user_queue
            .iter_mut()
            .chain(self.context.iter_mut())
            .filter(|q| q.path == track.path)
        {
            queued.clone_from(track);
        }
    }

    fn reshuffle(&mut self) {
        // skip cur track, we dont want shuffle to start with cur track
        self.shuffle_order = (0..self.context.len())
            .filter(|&i| i != self.context_pos)
            .collect();
        self.shuffle_order
            .shuffle(&mut rand::rng());
    }
}
