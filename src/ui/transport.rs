use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Block, Borders, Paragraph, Widget},
};

#[derive(Default)]
pub struct TransportState {}

impl TransportState {}

impl Widget for &TransportState {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let song_title = "test song";

        let block = Block::default().borders(Borders::TOP);
        let inner = block.inner(area);
        block.render(area, buf);

        let paragraph = Paragraph::new(song_title);
        paragraph.render(inner, buf);
    }
}
