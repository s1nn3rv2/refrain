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
    tag::{Accessor as _, ItemKey},
};

#[derive(Clone)]
pub struct Track {
    pub path: PathBuf,

    /// tags
    pub title: String,
    artists: String, // can have multiple, separated by a symbol (like ;)
    pub album: Option<String>,
    album_artists: Option<String>, // same story as artists
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    pub genre: Option<String>,
    pub date: Option<String>, // stored as string due to variety of different formats people use
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

        // personally, I don't like no album being shown as "Unknown Album", that's why I'm keeping
        // it as an option instead of just showing Unknown Album
        let mut album = None;
        let mut album_artists = None;
        let mut track_number = None;
        let mut disc_number = None;
        let mut genre = None;
        let mut date = None;

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
                if let Some(al) = tag.album().as_deref() {
                    album = Some(al.to_string());
                }
                if let Some(aa) = tag.get_string(ItemKey::AlbumArtists) {
                    album_artists = Some(aa.to_string());
                }

                track_number = tag.track();
                disc_number = tag.disk();
                genre = tag.genre().map(|g| g.to_string());

                date = tag
                    .get_string(ItemKey::ReleaseDate)
                    .or_else(|| tag.get_string(ItemKey::RecordingDate))
                    .or_else(|| tag.get_string(ItemKey::Year))
                    .map(|d| d.to_string());
            }
        }

        Self {
            path,
            title,
            artists,
            album,
            album_artists,
            track_number,
            disc_number,
            genre,
            date,
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
    // We do not keep it in search memory, it has derived fields and is cheap to build so no
    // need to keep it in cache, although in future perhaps we could make it be in cache for
    // faster loading? we'll see
    pub fn write_search_haystack(&self, out: &mut String) {
        use std::fmt::Write as _;
        let _ = write!(
            out,
            "{} {} {} {} {}",
            self.title,
            self.formatted_artists(),
            self.album.as_deref().unwrap_or(""),
            self.genre.as_deref().unwrap_or(""),
            self.date.as_deref().unwrap_or("")
        );
    }
}

#[derive(Default)]
pub struct LibraryState {
    pub tracks: Vec<Track>,
}

impl LibraryState {
    pub fn new() -> Self {
        if let Ok(tracks) = Self::load_cache()
            && !tracks.is_empty()
        {
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
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                t.path.display(),
                t.title,
                t.artists,
                t.album.as_deref().unwrap_or(""),
                t.album_artists
                    .as_deref()
                    .unwrap_or(""),
                t.track_number
                    .map(|n| n.to_string())
                    .unwrap_or_default(),
                t.disc_number
                    .map(|n| n.to_string())
                    .unwrap_or_default(),
                t.genre.as_deref().unwrap_or(""),
                t.date.as_deref().unwrap_or(""),
                t.length.as_millis()
            )?;
        }

        writer.flush()?;
        Ok(())
    }

    pub fn load_cache() -> color_eyre::Result<Vec<Track>> {
        let content = fs::read_to_string(Self::cache_path())?;
        let mut tracks = Vec::new();

        let opt_str = |s: &str| {
            if s.is_empty() {
                None
            } else {
                Some(s.to_string())
            }
        };

        for line in content.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() == 10 {
                let millis: u64 = parts[9].parse().unwrap_or(0);
                tracks.push(Track {
                    path: PathBuf::from(parts[0]),
                    title: parts[1].to_string(),
                    artists: parts[2].to_string(),
                    album: opt_str(parts[3]),
                    album_artists: opt_str(parts[4]),
                    track_number: parts[5].parse::<u32>().ok(),
                    disc_number: parts[6].parse::<u32>().ok(),
                    genre: opt_str(parts[7]),
                    date: opt_str(parts[8]),
                    length: Duration::from_millis(millis),
                });
            }
        }

        Ok(tracks)
    }

    // TODO: make scan incremental
    // TODO: automatically check file changes using notify and rescan
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
