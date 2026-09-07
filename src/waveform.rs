use std::{
    env::home_dir,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use audio_waveform::{ChannelMode, Measure, WaveformOptions};

use crate::util::fnv1a;

pub const WAVEFORM_POINTS: usize = 512;
// we use this instead of Vec<u8> as we know exactly how many points we have in each waveform
// For now we keep it as u8 as its probably enough precision, although it means we need to convert
// f32 to u8 from audio_waveform crate, could change it in future if needed
pub type WaveformData = [u8; WAVEFORM_POINTS];

/// Return cache path of a track, keyed by path and modified time
/// Example path: ~/.cache/refrain/waveforms/<hash>.bin
pub fn cache_path(track_path: &Path) -> PathBuf {
    // get modified time, returns 0 if cannot get
    let mtime = fs::metadata(track_path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    let key = format!("{}:{mtime}", track_path.display());
    let hash = fnv1a(key.as_bytes());

    let file_name = format!("{hash:016x}.bin");
    home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cache/refrain/waveforms")
        .join(file_name)
}

/// Decode file to peaks, or read from disk cache
pub fn load_or_compute(track_path: &Path) -> Option<WaveformData> {
    let file = cache_path(track_path);

    // check for disk cache
    if let Ok(mut f) = File::open(&file) {
        let mut data = [0u8; WAVEFORM_POINTS];
        if f.read_exact(&mut data).is_ok() {
            // hit cache! return that data
            return Some(data);
        }
    }

    // cache miss, generate with audio-waveform
    let options = WaveformOptions::new(WAVEFORM_POINTS)
        .measure(Measure::Peak)
        .channels(ChannelMode::Mix)
        .normalize(true);

    let (points, _duration) = audio_waveform::generate_u8(track_path, &options).ok()?;

    let data: WaveformData = points.try_into().ok()?;

    // ... check if dir exists, if not create
    if let Some(parent) = file.parent() {
        let _ = fs::create_dir_all(parent);
    }
    // and then create file
    if let Ok(mut f) = File::create(&file) {
        let _ = f.write_all(&data);
    }

    Some(data)
}
