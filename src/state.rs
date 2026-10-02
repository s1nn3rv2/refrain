use std::{fs, path::PathBuf};

use crate::{ActiveView, config::Column, ui::library::SortDirection};

// if app ever gets big, should probably use separate states for different elements of the app
// instead of one like this
pub struct State {
    pub current_track: Option<PathBuf>,
    pub position_ms: u64, // could use duration, but this is just simpler lol, Duration gives a
    // table like {secs = 5, nanos = xxxxx }
    pub queue: Vec<PathBuf>,
    pub selected_track: Option<PathBuf>,
    pub search: String,
    pub sort_key: Option<Column>,
    pub sort_direction: Option<SortDirection>,
    pub show_queue: Option<bool>,
    pub active_view: Option<ActiveView>,
}

impl State {
    fn path() -> Option<PathBuf> {
        dirs::state_dir().or_else(dirs::data_local_dir).map(|dir| dir.join("refrain").join("state.toml"))
    }

    fn load() -> Self {
        Self::path().and_then(|path| fs::read_to_string(path).ok()).and_then(|content| toml::from_str(&content).ok()).unwrap_or_default()
    }
}
