use audio_waveform::{Measure, WaveformOptions};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Color,
    symbols::Marker,
    widgets::{
        Widget,
        canvas::{Canvas, Line},
    },
};

use crate::waveform::WaveformData;

pub struct WaveformWidget {
    data: WaveformData,
    progress: f64, // 0.0 ..= 1.0
}

impl WaveformWidget {
    pub fn new(data: WaveformData, progress: f64) -> Self {
        Self {
            data,
            progress: progress.clamp(0.0, 1.0),
        }
    }
}

impl Widget for WaveformWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let num_points = (area.width as usize) * 2;

        let resample_opts = WaveformOptions::new(num_points)
            .measure(Measure::Peak)
            .normalize(false);

        // downsample points to exact columns
        let columns = audio_waveform::generate_from_u8_samples(&self.data, &resample_opts);
        let played_boundary = (self.progress * num_points as f64).round() as usize;

        Canvas::default()
            .marker(Marker::Braille) // braille seems the highest res
            .x_bounds([0.0, num_points as f64])
            .y_bounds([-1.0, 1.0])
            .paint(|ctx| {
                for (x, &val) in columns.iter().enumerate() {
                    let amp = (val as f64 / 255.0).clamp(0.0, 1.0);
                    let color = if x < played_boundary {
                        Color::Cyan
                    } else {
                        Color::DarkGray
                    };

                    ctx.draw(&Line::new(x as f64, -amp, x as f64, amp, color));
                }
            })
            .render(area, buf);
    }
}
