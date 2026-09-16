use std::time::{SystemTime, UNIX_EPOCH};

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{Paragraph, Widget},
};
use unicode_truncate::UnicodeTruncateStr;

pub struct Marquee {
    text: String,
    style: Style,
    separator: String,
}

impl Marquee {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: Style::default(),
            separator: "   x   ".to_string(),
        }
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn scroll(text: &str, max_width: usize) -> String {
        Self::scroll_with_separator(text, max_width, "   x   ")
    }

    pub fn scroll_with_separator(text: &str, max_width: usize, separator: &str) -> String {
        // if text already fits in max_width, dont scroll
        let text_width = Line::from(text).width();
        if text_width <= max_width || max_width == 0 {
            return text.to_string();
        }

        let stream = format!("{text}{separator}");
        let chars: Vec<char> = stream.chars().collect();

        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let offset = ((now_ms / 250) as usize) % chars.len(); // this should be multiple of TICK
        // in main.rs, not less though

        let sample: String = chars
            .iter()
            .cycle()
            .skip(offset)
            .take(max_width * 2)
            .collect();

        let (display, _) = sample.unicode_truncate(max_width);
        display.to_string()
    }
}

impl Widget for Marquee {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let scrolled =
            Self::scroll_with_separator(&self.text, area.width as usize, &self.separator);
        Paragraph::new(scrolled)
            .style(self.style)
            .render(area, buf);
    }
}
