use std::{
    env::home_dir,
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

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

/// taken from fnv crate code
/// didn't add it as a dependency, as it is a pretty simple function
/// Why use this instead of DefaultHasher? DefaultHasher is not stable,
/// so cache could be invalidated during rust changes, using this prevents that.
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

    home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cache/refrain")
        .join(dir)
        .join(format!("{hash:016x}.{ext}"))
}
