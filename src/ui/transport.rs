use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::{
    audio::AudioPlayer, ui::waveform::WaveformWidget, util::DurationExt, waveform::WaveformData,
};

#[derive(Default)]
pub struct TransportState {}

impl TransportState {
    pub fn render(
        &self,
        player: &AudioPlayer,
        waveform: Option<&WaveformData>,
        area: Rect,
        buf: &mut Buffer,
    ) {
        let current_track = player.current_track();

        let (title_display, time_display, progress) = if let Some(track) = current_track {
            let song = format!("{} - {}", track.formatted_artists(), track.title);
            let elapsed = player.elapsed();
            let time = format!("{} / {}", elapsed.format_time(), track.length.format_time());
            let progress = if track.length.as_secs_f64() > 0.0 {
                elapsed.as_secs_f64() / track.length.as_secs_f64()
            } else {
                0.0
            };

            (song, time, progress)
        } else {
            (
                "No track playing".to_string(),
                "--:-- / --:--".to_string(),
                0.0,
            )
        };

        let block = Block::default().borders(Borders::TOP);
        let inner = block.inner(area);
        block.render(area, buf);

        let [title_area, waveform_area, time_area] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .areas(inner);

        Paragraph::new(title_display).render(title_area, buf);

        if let Some(wf) = waveform {
            WaveformWidget::new(*wf, progress).render(waveform_area, buf);
        }

        Paragraph::new(time_display).render(time_area, buf);
    }
}
