use std::{
    env::home_dir,
    fs::{self},
    io,
    path::{Path, PathBuf},
};

use color_eyre::eyre::Context;

#[derive(Clone)]
pub struct Track {
    pub title: String,
    pub path: PathBuf,
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
            .map(|path| {
                let file_name = path
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap();

                Track {
                    title: file_name.to_string(),
                    path,
                }
            })
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
