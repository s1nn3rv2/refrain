use std::{
    env::home_dir,
    fs::{self},
    io,
    path::{Path, PathBuf},
    time::Duration,
};

use color_eyre::eyre::Context;
use lofty::{
    file::{AudioFile, TaggedFileExt},
    tag::Accessor as _,
};

#[derive(Clone)]
pub struct Track {
    pub path: PathBuf,

    /// tags
    pub title: String,
    pub artist: String,
    pub length: Duration,
}

impl Track {
    pub fn from_path(path: PathBuf) -> Self {
        let mut title = path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap_or("Unknown Track")
            .to_string();

        let mut artist = "Unknown Artist".to_string();
        let mut length = Duration::ZERO;

        if let Ok(tagged_file) = lofty::read_from_path(&path) {
            let properties = tagged_file.properties();
            length = properties.duration();

            if let Some(tag) = tagged_file
                .primary_tag()
                .or_else(|| tagged_file.first_tag())
            {
                if let Some(t) = tag.title().as_deref() {
                    title = t.to_string();
                }
                if let Some(a) = tag.artist().as_deref() {
                    artist = a.to_string()
                }
            }
        }

        Self {
            path,
            title,
            artist,
            length,
        }
    }
}

#[derive(Default)]
pub struct LibraryState {
    pub tracks: Vec<Track>,
}

impl LibraryState {
    pub fn new() -> Self {
        let mut state = Self::default();
        let _ = state.scan();
        state
    }

    pub fn scan(&mut self) -> color_eyre::Result<()> {
        let music_path = home_dir().unwrap().join("Music");
        let files = visit_dirs(&music_path).wrap_err("Failed to scan music directory")?;

        self.tracks = files
            .into_iter()
            .map(|path| -> Track { Track::from_path(path) })
            .collect();

        Ok(())
    }
}

fn visit_dirs(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_files(dir, &mut files)?;
    Ok(files)
}

fn collect_files(dir: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                collect_files(&path, files)?;
            } else {
                files.push(path);
            }
        }
    }
    Ok(())
}
