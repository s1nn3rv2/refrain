use std::time::Duration;

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget},
};

use crate::{audio::AudioPlayer, library::Track, util::DurationExt};

#[derive(Default)]
pub struct TransportState {}

impl TransportState {
    pub fn render(&self, player: &AudioPlayer, area: Rect, buf: &mut Buffer) {
        let current_track = player.current_track();

        let (title_display, time_display) = if let Some(track) = current_track {
            let song = format!("{} - {}", track.formatted_artists(), track.title);
            let time = format!(
                "{} / {}",
                player.elapsed().format_time(),
                track.length.format_time()
            );

            (song, time)
        } else {
            ("No track playing".to_string(), "--:-- / --:--".to_string())
        };

        let block = Block::default().borders(Borders::TOP);
        let inner = block.inner(area);
        block.render(area, buf);

        let [title_area, time_area] =
            Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(inner);

        Paragraph::new(title_display).render(title_area, buf);
        Paragraph::new(time_display).render(time_area, buf);
    }
}
