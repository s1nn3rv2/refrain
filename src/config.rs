use std::{env::home_dir, path::PathBuf, sync::OnceLock};

use color_eyre::eyre::Context;
use ratatui::layout::{Alignment, Constraint};
use serde::{Deserialize, Serialize};

use crate::task::THUMB_SIZE;

// TODO: implement hot-reload
//
static CONFIG: OnceLock<Config> = OnceLock::new();

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ColumnAlignment {
    Left,
    Center,
    Right,
}

impl From<ColumnAlignment> for Alignment {
    fn from(align: ColumnAlignment) -> Self {
        match align {
            ColumnAlignment::Left => Alignment::Left,
            ColumnAlignment::Center => Alignment::Center,
            ColumnAlignment::Right => Alignment::Right,
        }
    }
}

#[derive(PartialEq, Eq, Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum Column {
    Cover,
    Artist,
    Title,
    Album,
    Genre,
    Date,
    Length,
}

impl Column {
    pub fn default_columns() -> Vec<ColumnSetting> {
        vec![
            ColumnSetting::Simple(Column::Cover),
            ColumnSetting::Simple(Column::Artist),
            ColumnSetting::Simple(Column::Title),
            ColumnSetting::Simple(Column::Album),
            ColumnSetting::Simple(Column::Length),
        ]
    }

    pub fn default_constraint(&self) -> Constraint {
        match self {
            Self::Cover => Constraint::Length(THUMB_SIZE.width),
            Self::Artist => Constraint::Percentage(20),
            Self::Title => Constraint::Fill(1),
            Self::Album => Constraint::Percentage(20),
            Self::Genre => Constraint::Percentage(15),
            Self::Date => Constraint::Length(10), // xxxx-xx-xx
            Self::Length => Constraint::Percentage(8),
        }
    }

    pub fn default_title(&self) -> &str {
        match self {
            Self::Cover => "",
            Self::Artist => "Artists",
            Self::Title => "Title",
            Self::Album => "Album",
            Self::Genre => "Genre",
            Self::Date => "Date",
            Self::Length => "Length",
        }
    }

    pub fn default_alignment(&self) -> ColumnAlignment {
        match self {
            Self::Length => ColumnAlignment::Right,
            _ => ColumnAlignment::Left,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum ColumnSetting {
    Simple(Column), // uses predefined setting for each column
    Detailed {
        // allows user to change each column width
        name: Column,
        label: Option<String>,
        width: Option<String>,
        alignment: Option<ColumnAlignment>,
    },
}

impl ColumnSetting {
    pub fn column(&self) -> Column {
        match self {
            Self::Simple(col) => *col,
            Self::Detailed { name, .. } => *name,
        }
    }

    pub fn label(&self) -> String {
        match self {
            Self::Simple(col) => col.default_title().to_string(),
            Self::Detailed { name, label, .. } => label
                .clone()
                .unwrap_or_else(|| name.default_title().to_string()),
        }
    }

    pub fn alignment(&self) -> Alignment {
        match self {
            Self::Simple(col) => col.default_alignment().into(),
            Self::Detailed {
                name, alignment, ..
            } => alignment
                .unwrap_or_else(|| name.default_alignment())
                .into(),
        }
    }

    pub fn constraint(&self) -> Constraint {
        match self {
            Self::Simple(col) => col.default_constraint(),
            Self::Detailed { name, width, .. } => match width.as_deref() {
                Some("fill") => Constraint::Fill(1),
                Some(w) if w.ends_with('%') => {
                    let pct = w
                        .trim_end_matches('%')
                        .parse()
                        .unwrap_or(20);
                    Constraint::Percentage(pct)
                },
                Some(w) => {
                    let len = w.parse().unwrap_or(8);
                    Constraint::Length(len)
                },
                None => name.default_constraint(),
            },
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(default)]
pub struct Config {
    pub music_dir: PathBuf,
    pub columns: Vec<ColumnSetting>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            music_dir: dirs::audio_dir()
                .or_else(|| home_dir().map(|h| h.join("Music")))
                .unwrap_or_else(|| PathBuf::from("Music")),
            columns: Column::default_columns(),
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

        let mut config: Config = toml::from_str(&content)
            .wrap_err(format!("Failed to parse config file at {:?}", path))?;

        // could use shellexpand in future if needed, for now this suffices
        if let Ok(suffix) = config.music_dir.strip_prefix("~") {
            config.music_dir = dirs::home_dir()
                .map(|h| h.join(suffix))
                .unwrap_or(config.music_dir);
        }

        Ok(config)
    }
}
