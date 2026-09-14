use std::{env::home_dir, path::PathBuf, sync::OnceLock};

use color_eyre::eyre::Context;
use serde::{Deserialize, Serialize};

static CONFIG: OnceLock<Config> = OnceLock::new();

// TODO: Support ~ symbol in config

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct Config {
    pub music_dir: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            music_dir: dirs::audio_dir()
                .or_else(|| home_dir().map(|h| h.join("Music")))
                .unwrap_or_else(|| PathBuf::from("Music")),
        }
    }
}

impl Config {
    pub fn init(config: Config) {
        CONFIG
            .set(config)
            .expect("Config should only be initialized once");
    }

    pub fn get() -> &'static Config {
        CONFIG
            .get()
            .expect("Config must be initialized before accessing")
    }

    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| {
            p.join("refrain")
                .join("config.toml")
        })
    }

    pub fn load() -> color_eyre::Result<Self> {
        let Some(path) = Self::config_path() else {
            return Ok(Self::default());
        };

        if !path.exists() {
            // TODO: write template config for user?
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&path)
            .wrap_err_with(|| format!("Failed to read config file at {:?}", path))?;

        let config: Config = toml::from_str(&content)
            .wrap_err(format!("Failed to parse config file at {:?}", path))?;

        Ok(config)
    }
}
