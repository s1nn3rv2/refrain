use std::{
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use lofty::file::FileType;

pub trait DurationExt {
    fn format_time(&self) -> String;
}

impl DurationExt for std::time::Duration {
    fn format_time(&self) -> String {
        let total_secs = self.as_secs();
        let hours = total_secs / 3600;
        let mins = (total_secs % 3600) / 60;
        let secs = total_secs % 60;

        if hours > 0 {
            format!("{hours:02}:{mins:02}:{secs:02}")
        } else {
            format!("{mins:02}:{secs:02}")
        }
    }
}

// taken from fnv crate code
// didn't add it as a dependency, as it is a pretty simple function
// Why use this instead of DefaultHasher? DefaultHasher is not stable,
// so cache could be invalidated during rust changes, using this prevents that.
pub fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
    }
    hash
}

/// Returns modified time of a file at path, returns 0 if cannot get
pub fn get_mtime(file_path: &Path) -> u128 {
    fs::metadata(file_path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

/// Returns `~/.cache/refrain`
pub fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| {
            dirs::home_dir()
                .map(|h| h.join(".cache"))
                .unwrap_or_else(|| PathBuf::from(".cache"))
        })
        .join("refrain")
}

/// Utility to return cache path
/// `~/.cache/refrain/<dir>/<hash>.<ext>`
/// Keyed by path and modified time
pub fn cache_path(track_path: &Path, dir: &str, ext: &str) -> PathBuf {
    let mtime = get_mtime(track_path);

    // We add mtime as a key so when a track is modified (cover art update, tags), cache is
    // invalidated and it no longer uses old saved information
    //
    // Now, this also means that old cache records are not being removed, but it doesn't really
    // matter in terms of space at all. Perhaps for cover it is a bigger difference, but it's still
    // like max 200KB, so it wont be an issue either.
    let key = format!("{}:{mtime}", track_path.display());
    let hash = fnv1a(key.as_bytes());

    cache_dir()
        .join(dir)
        .join(format!("{hash:016x}.{ext}"))
}

/// Splits a query on whitespace, except inside double quotes, so a tag value can hold spaces. For
/// example: `artist:"best artist ever" edm` -> ["artist:best artist ever", "edm"]
// TODO: we do not handle unfinished quotes yet, but I don't know how to properly deal with
// those yet
pub fn tokenize(query: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quoted = false; // keep track of quotes

    for c in query.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                current.push('"'); // keep the quote
            },
            c if c.is_whitespace() && !quoted => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            },
            // otherwise, we append the character to current
            c => current.push(c),
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

// Lofty supports a ton of formats, but we are limited to what rodio will actually play. By default
// it's those, can be expanded via feature flags if people will report need for more formats tho
pub fn is_audio_file(path: &Path) -> bool {
    matches!(
        FileType::from_path(path),
        Some(
            FileType::Mpeg
                | FileType::Flac
                | FileType::Wav
                | FileType::Vorbis
                | FileType::Mp4
                | FileType::Aac
        )
    )
}
