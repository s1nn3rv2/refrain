mod app;
mod audio;
mod config;
mod cover;
mod library;
mod mpris;
mod queue;
mod state;
mod task;
mod ui;
mod util;
mod waveform;

use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use ratatui_image::picker::Picker;

use crate::{app::App, config::Config};

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let config = Config::load()?;
    Config::init(config);

    let picker = Picker::from_query_stdio()?;

    // enable mouse capture
    crossterm::execute!(std::io::stdout(), EnableMouseCapture)?;

    let res = ratatui::run(|terminal| App::new(picker).run(terminal));

    // disable mouse capture on exit
    let _ = crossterm::execute!(std::io::stdout(), DisableMouseCapture);

    res
}
