use ratatui::{
    text::{Line, Text},
    widgets::{Cell, Row},
};

use crate::{library::Track, task::THUMB_SIZE};

pub const ROW_MARGIN: u16 = 1;

pub fn track_table_header() -> Row<'static> {
    Row::new([
        Cell::from(""),
        Cell::from("Artists"),
        Cell::from("Title"),
        Cell::from("Album"),
        Cell::from(Line::from("Length").right_aligned()), // cell has no right_aligned lol
    ])
}

pub fn track_to_row(track: &Track) -> Row {
    let duration_secs = track.length.as_secs();
    let duration_display = format!("{:02}:{:02}", duration_secs / 60, duration_secs % 60);

    let centered_artist = Text::from(vec![Line::from(""), Line::from(track.formatted_artists())]);

    let centered_title = Text::from(vec![Line::from(""), Line::from(track.title.as_str())]);

    let centered_album = Text::from(vec![
        Line::from(""),
        Line::from(
            track
                .album
                .as_deref()
                .unwrap_or(""),
        ),
    ]);

    let centered_duration = Text::from(vec![
        Line::from(""),
        Line::from(duration_display).right_aligned(),
    ]);

    Row::new([
        Cell::from(""), // for cover art
        Cell::from(centered_artist),
        Cell::from(centered_title),
        Cell::from(centered_album),
        Cell::from(centered_duration),
    ])
    .height(THUMB_SIZE.height)
    .bottom_margin(ROW_MARGIN)
}
