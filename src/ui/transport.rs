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
        let song_title = match current_track {
            Some(track) => track.title.as_str(),
            None => "No track playing",
        };

        let block = Block::default().borders(Borders::TOP);
        let inner = block.inner(area);
        block.render(area, buf);

        let paragraph = Paragraph::new(song_title);
        paragraph.render(inner, buf);
    }
}
