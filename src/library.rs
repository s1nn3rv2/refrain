use std::{
    collections::HashMap, fs::{self, File}, io::{self, BufWriter, Write}, path::{Path, PathBuf}, time::Duration,
};

use color_eyre::eyre::Context;
use lofty::{
    config::WriteOptions,
    file::{AudioFile, TaggedFileExt},
    tag::{Accessor as _, ItemKey, Tag, TagExt},
};

use crate::{
    config::Config,
    util::{self, is_audio_file},
};

#[derive(Clone)]
pub struct TrackTags {
    pub title: String,
    pub artists: String, // can have multiple, separated by a symbol (like ;),
    pub album: Option<String>,
    pub album_artists: Option<String>, // same story as artists
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    pub genre: Option<String>,
    pub date: Option<String>, // stored as string due to variety of different formats people use
}

impl TrackTags {
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

    // Why separation between formatted and unformatted and why not just make artists public? I
    // don't wanna cause confusion, since artists are stored internally as separated with ;.
    // Separate explicit function for formatted and unformatted artists make it more clear!

    /// Unformatted artists, returns "Artist1;Artist2;Artist3"
    pub fn unformatted_artists(&self) -> &str {
        &self.artists
    }
    /// Unformatted album artists, returns "Artist1;Artist2;Artist3"
    pub fn unformatted_album_artists(&self) -> &str {
        self.album_artists
            .as_deref()
            .unwrap_or_default()
    }

    pub fn individual_artists(&self) -> Vec<&str> {
        self.artists
            .split(';')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect()
    }

    pub fn album(&self) -> &str {
        self.album.as_deref().unwrap_or("")
    }

    pub fn genre(&self) -> &str {
        self.genre.as_deref().unwrap_or("")
    }

    pub fn date(&self) -> &str {
        self.date.as_deref().unwrap_or("")
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
            self.album(),
            self.genre(),
            self.date()
        );
    }
}

#[derive(Clone)]
pub struct Track {
    pub path: PathBuf,

    pub tags: TrackTags,
    pub length: Duration,

    /// Used by incremental scan to know if the file changed since last scan
    pub mtime: u128,
}

impl Track {
    pub fn from_path(path: PathBuf) -> Self {
        let mtime = util::get_mtime(&path);
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
                // Sometimes, tracks have multiple artists parsed as multiple Artist tags, like:
                // Artist = 'Artist1'
                // Artist = 'Artist2'
                // Instead of the usual
                // Artist = 'Artist1;Artist2'
                // So we reconcile those to a single string joined with ;
                let parsed_artists: Vec<&str> = tag
                    .get_strings(ItemKey::TrackArtist)
                    .collect();
                artists = if !parsed_artists.is_empty() {
                    parsed_artists.join(";")
                } else {
                    artists
                };
                // same situation as above!
                let parsed_album_artists: Vec<&str> = tag
                    .get_strings(ItemKey::AlbumArtist)
                    .chain(tag.get_strings(ItemKey::AlbumArtists))
                    .collect();
                album_artists =
                    (!parsed_album_artists.is_empty()).then(|| parsed_album_artists.join(";"));
                if let Some(al) = tag.album().as_deref() {
                    album = Some(al.to_string());
                }

                track_number = tag.track();
                // i will not follow your "disk" spelling!!! disc is clearly superior
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
            tags: TrackTags {
                title,
                artists,
                album,
                album_artists,
                track_number,
                disc_number,
                genre,
                date,
            },
            length,
            mtime,
        }
    }

    pub fn save_tags(&mut self, tags: TrackTags) -> color_eyre::Result<()> {
        let mut tagged_file =
            lofty::read_from_path(&self.path).wrap_err("Failed to read audio file")?;

        let tag = match tagged_file.primary_tag_mut() {
            Some(t) => t,
            None => {
                if let Some(first) = tagged_file.first_tag_mut() {
                    first
                } else {
                    let tag_type = tagged_file.primary_tag_type();
                    tagged_file.insert_tag(Tag::new(tag_type));
                    tagged_file
                        .primary_tag_mut()
                        .unwrap()
                }
            },
        };

        tag.set_title(tags.title.clone());
        tag.set_artist(tags.artists.clone());

        if let Some(ref al) = tags.album {
            tag.set_album(al.clone());
        } else {
            tag.remove_album()
        }

        if let Some(ref g) = tags.genre {
            tag.set_genre(g.clone());
        } else {
            tag.remove_genre();
        }

        if let Some(t_num) = tags.track_number {
            tag.set_track(t_num);
        } else {
            tag.remove_track();
        }

        if let Some(d_num) = tags.disc_number {
            tag.set_disk(d_num);
        } else {
            tag.remove_disk();
        }

        if let Some(ref aa) = tags.album_artists {
            tag.insert_text(ItemKey::AlbumArtist, aa.clone());
        } else {
            tag.remove_key(ItemKey::AlbumArtist);
        }

        if let Some(ref date) = tags.date {
            tag.insert_text(ItemKey::ReleaseDate, date.clone());
        } else {
            tag.remove_key(ItemKey::ReleaseDate);
        }

        tag.save_to_path(&self.path, WriteOptions::default())
            .wrap_err("Failed to write tags to audio file")?;

        self.tags = tags;
        self.mtime = util::get_mtime(&self.path); // file was rewritten! make sure to update mtime
        Ok(())
    }
}

/// Shows what changed during a scan
#[derive(Default)]
pub struct Scan {
    pub added: Vec<PathBuf>,
    pub updated: Vec<PathBuf>,
    pub removed: Vec<PathBuf>,
}

impl Scan {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.updated.is_empty() && self.removed.is_empty()
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
        util::cache_dir().join("library.tsv")
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
            // TODO: make this done in a nicer way, this is so awful it actually hurts
            writeln!(
                writer,
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                t.path.display(),
                t.tags.title,
                t.tags.artists,
                t.tags
                    .album
                    .as_deref()
                    .unwrap_or(""),
                t.tags
                    .album_artists
                    .as_deref()
                    .unwrap_or(""),
                t.tags
                    .track_number
                    .map(|n| n.to_string())
                    .unwrap_or_default(),
                t.tags
                    .disc_number
                    .map(|n| n.to_string())
                    .unwrap_or_default(),
                t.tags
                    .genre
                    .as_deref()
                    .unwrap_or(""),
                t.tags
                    .date
                    .as_deref()
                    .unwrap_or(""),
                t.length.as_millis(),
                t.mtime,
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
            if parts.len() == 11 {
                let millis: u64 = parts[9].parse().unwrap_or(0);
                tracks.push(Track {
                    path: PathBuf::from(parts[0]),
                    tags: TrackTags {
                        title: parts[1].to_string(),
                        artists: parts[2].to_string(),
                        album: opt_str(parts[3]),
                        album_artists: opt_str(parts[4]),
                        track_number: parts[5].parse::<u32>().ok(),
                        disc_number: parts[6].parse::<u32>().ok(),
                        genre: opt_str(parts[7]),
                        date: opt_str(parts[8]),
                    },
                    length: Duration::from_millis(millis),
                    mtime: parts[10].parse().unwrap_or(0),
                });
            }
        }

        Ok(tracks)
    }

    // TODO: automatically check file changes using notify and rescan
    /// Scans tracks in music directory incrementally
    pub fn scan(&mut self) -> color_eyre::Result<Scan> {
        let music_path = &Config::get().music_dir;
        let files = visit_dirs(music_path).wrap_err("Failed to scan music directory")?;

        // hashmap for quicker lookup
        // old is stuff that existed on last scan
        let mut old: HashMap<PathBuf, Track> = std::mem::take(&mut self.tracks).into_iter().map(|t| (t.path.clone(), t)).collect();

        let mut scan = Scan::default();
        let mut tracks = Vec::with_capacity(files.len());

        for (path, mtime) in files {
            match old.remove(&path) {
                Some(track) if track.mtime == mtime => tracks.push(track),
                Some(_) => {
                    scan.updated.push(path.clone());
                    tracks.push(Track::from_path(path));
                },
                None => {
                    scan.added.push(path.clone());
                    tracks.push(Track::from_path(path));
                },
            }
        }

        scan.removed = old.into_keys().collect();

        self.tracks = tracks;

        if !scan.is_empty() {
            self.save_cache()?;
        }

        Ok(scan)
    }
}

fn visit_dirs(dir: &Path) -> io::Result<Vec<(PathBuf, u128)>> {
    let mut files = Vec::new();
    collect_files(dir, &mut files)?;
    Ok(files)
}

fn collect_files(dir: &Path, files: &mut Vec<(PathBuf, u128)>) -> io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                collect_files(&path, files)?;
            } else if is_audio_file(&path) {
                let mtime = util::get_mtime(&path);
                files.push((path, mtime));
            }
        }
    }
    Ok(())
}
