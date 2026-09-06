use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::library::Track;

#[derive(Default)]
pub struct TransportState {}

impl TransportState {
    pub fn render(&self, current_track: Option<&Track>, area: Rect, buf: &mut Buffer) {
        let song_playing_display = match current_track {
            Some(track) => format!("{} - {}", track.formatted_artists(), track.title),
            None => "No track playing".to_string(),
        };

        let block = Block::default().borders(Borders::TOP);
        let inner = block.inner(area);
        block.render(area, buf);

        let paragraph = Paragraph::new(song_playing_display);
        paragraph.render(inner, buf);
    }
}
