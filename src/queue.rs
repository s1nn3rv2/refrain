use std::collections::VecDeque;

use crate::library::Track;

pub struct QueueManager {
    // Track queued by the user to play first
    pub user_queue: VecDeque<Track>,
    // TODO: add context based queue (for example current filtered library view)
}

impl QueueManager {
    pub fn new() -> Self {
        Self {
            user_queue: VecDeque::new(),
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

    pub fn pop_next(&mut self) -> Option<Track> {
        if let Some(track) = self.user_queue.pop_front() {
            return Some(track);
        }
        // no more tracks left!
        None
    }
}
