use std::{
    env::home_dir,
    fs::{self, File},
    io::{self, BufWriter, Write},
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
    artists: String, // can have multiple, separated by a symbol (like ;)
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

        let mut artists = "Unknown Artist".to_string();
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
                    artists = a.to_string()
                }
            }
        }

        // We do not keep it in search memory, it has derived fields and is cheap to build so no
        // need to keep it in cache, although in future perhaps we could make it be in cache for
        // faster loading? we'll see
        Self {
            path,
            title,
            artists,
            length,
        }
    }

    /// Returns artists in a nice format ("Artist1;Artist2" -> "Artist1, Artist2")
    pub fn format_artists(raw: &str) -> String {
        raw.split(';')
            .map(|s| s.trim()) // remove any remaining whitespaces, so "Artist1; Artist2" and "Artist1;Artist2" behave the same
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Formatted as "Artist 1, Artist 2, Artist 3"
    pub fn formatted_artists(&self) -> String {
        Self::format_artists(&self.artists)
    }

    /// Returns a search haystack string, containing all searchable fields
    pub fn write_search_haystack(&self, out: &mut String) {
        use std::fmt::Write as _;
        let _ = write!(out, "{} {}", self.title, self.formatted_artists());
    }
}

#[derive(Default)]
pub struct LibraryState {
    pub tracks: Vec<Track>,
}

impl LibraryState {
    pub fn new() -> Self {
        if let Ok(tracks) = Self::load_cache() {
            return Self { tracks };
        }

        // if cant load cache (on first startup most likely), scan library first and then save cache
        let mut state = Self::default();
        let _ = state.scan();
        let _ = state.save_cache();
        state
    }

    fn cache_path() -> PathBuf {
        home_dir()
            .unwrap()
            .join(".cache/refrain/library.tsv")
    }

    pub fn cache_exists() -> bool {
        fs::exists(Self::cache_path()).unwrap_or(false)
    }

    pub fn save_cache(&self) -> color_eyre::Result<()> {
        let path = Self::cache_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        for t in &self.tracks {
            writeln!(
                writer,
                "{}\t{}\t{}\t{}",
                t.path.display(),
                t.title,
                t.artists,
                t.length.as_millis()
            )?;
        }

        writer.flush()?;
        Ok(())
    }

    pub fn load_cache() -> color_eyre::Result<Vec<Track>> {
        let content = fs::read_to_string(Self::cache_path())?;
        let mut tracks = Vec::new();

        for line in content.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() == 4 {
                let millis: u64 = parts[3].parse().unwrap_or(0);
                tracks.push(Track {
                    path: PathBuf::from(parts[0]),
                    title: parts[1].to_string(),
                    artists: parts[2].to_string(),
                    length: Duration::from_millis(millis),
                })
            }
        }

        Ok(tracks)
    }

    pub fn scan(&mut self) -> color_eyre::Result<()> {
        let music_path = home_dir().unwrap().join("Music");
        let files = visit_dirs(&music_path).wrap_err("Failed to scan music directory")?;

        self.tracks = files
            .into_iter()
            .map(|path| -> Track { Track::from_path(path) })
            .collect();

        self.save_cache()?;

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
