use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Position, Rect},
    widgets::{Block, Borders, Paragraph, Widget},
};
use ratatui_image::{Image, protocol::Protocol};

use crate::{
    audio::AudioPlayer, task::COVER_SIZE, ui::waveform::WaveformWidget, util::DurationExt,
    waveform::WaveformData,
};

pub enum TransportAction {
    Seek(f64), // progress from 0..1.0
}

#[derive(Default)]
pub struct TransportState {
    waveform_area: Rect,
}

impl TransportState {
    pub fn render(
        &mut self,
        player: &AudioPlayer,
        waveform: Option<&WaveformData>,
        cover: Option<&Protocol>,
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

        let [cover_area, details_area] =
            Layout::horizontal([Constraint::Length(COVER_SIZE.width), Constraint::Fill(1)])
                .spacing(2)
                .areas(inner);

        if let Some(protocol) = cover {
            Image::new(protocol).render(cover_area, buf);
        }

        let [title_area, waveform_area, time_area] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .areas(details_area);

        self.waveform_area = waveform_area;

        Paragraph::new(title_display).render(title_area, buf);

        match waveform {
            Some(wf) => WaveformWidget::new(*wf, progress).render(waveform_area, buf),
            None => WaveformWidget::empty().render(waveform_area, buf),
        }

        Paragraph::new(time_display).render(time_area, buf);
    }

    pub fn handle_mouse_event(&self, event: MouseEvent) -> Option<TransportAction> {
        match event.kind {
            MouseEventKind::Down(MouseButton::Left) | MouseEventKind::Drag(MouseButton::Left) => {
                let click_pos = Position::new(event.column, event.row);

                // check if click was inside the waveform area
                if self
                    .waveform_area
                    .contains(click_pos)
                {
                    let offset = (event.column - self.waveform_area.x) as f64;
                    let progress = (offset / self.waveform_area.width as f64).clamp(0.0, 1.0);
                    return Some(TransportAction::Seek(progress));
                }
            },
            _ => {},
        }
        None
    }
}
