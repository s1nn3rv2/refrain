use std::path::Path;

use lru::LruCache;
use ratatui::{
    buffer::Buffer,
    layout::{Rect, Size},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Cell, Row, Widget},
};
use ratatui_image::{Image, protocol::Protocol};
use unicode_truncate::UnicodeTruncateStr;

use crate::{library::Track, task::THUMB_SIZE, util::DurationExt};

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

pub fn track_to_row(track: &Track) -> Row<'_> {
    let duration_secs = track.length.as_secs();
    let duration_display = format!("{:02}:{:02}", duration_secs / 60, duration_secs % 60);

    let centered_artist = Text::from(vec![
        Line::from(""),
        Line::from(track.tags.formatted_artists()),
    ]);

    let centered_title = Text::from(vec![Line::from(""), Line::from(track.tags.title.as_str())]);

    let centered_album = Text::from(vec![
        Line::from(""),
        Line::from(
            track
                .tags
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

pub fn track_to_compact_row<'a>(track: &'a Track, width: usize) -> Row<'a> {
    let time = track.length.format_time();
    let album_name = track
        .tags
        .album
        .as_deref()
        .unwrap_or("Single");

    let line_title = Line::from(Span::styled(
        track.tags.title.as_str(),
        Style::default().add_modifier(Modifier::BOLD),
    ));

    let line_artist = Line::from(Span::styled(
        track.tags.formatted_artists(),
        Style::default().fg(Color::Gray),
    ));

    let max_album = width.saturating_sub(time.len() + 1);
    let (album_display, album_width) = album_name.unicode_truncate(max_album);

    let pad = width.saturating_sub(album_width + time.len());
    let line_album = Line::from(vec![
        Span::styled(album_display, Style::default().fg(Color::DarkGray)),
        Span::raw(" ".repeat(pad)),
        Span::styled(time, Style::default().fg(Color::DarkGray)),
    ]);

    let card_text = Text::from(vec![line_title, line_artist, line_album]);

    Row::new([Cell::from(""), Cell::from(card_text)])
        .height(THUMB_SIZE.height)
        .bottom_margin(ROW_MARGIN)
}

pub fn render_visible_thumbnails<'a, I>(
    tracks: I,
    offset: usize,
    has_header: bool,
    inner: Rect,
    thumbnails: &mut LruCache<std::path::PathBuf, Option<(Size, Protocol)>>,
    visible: &mut Vec<(std::path::PathBuf, Size)>,
    buf: &mut Buffer,
) where
    I: IntoIterator<Item = &'a Path>,
{
    let first_row_y = if has_header { inner.y + 1 } else { inner.y };
    let first_row_x = inner.x;

    for (screen_row, path) in tracks
        .into_iter()
        .skip(offset)
        .enumerate()
    {
        let y = first_row_y + screen_row as u16 * (THUMB_SIZE.height + ROW_MARGIN);
        if y + THUMB_SIZE.height > inner.y + inner.height {
            break;
        }

        let rect = Rect {
            x: first_row_x,
            y,
            width: THUMB_SIZE.width,
            height: THUMB_SIZE.height,
        };

        render_thumbnail(path, rect, thumbnails, visible, buf);
    }
}

pub fn render_thumbnail(
    path: &Path,
    rect: Rect,
    thumbnails: &mut LruCache<std::path::PathBuf, Option<(Size, Protocol)>>,
    visible: &mut Vec<(std::path::PathBuf, Size)>,
    buf: &mut Buffer,
) {
    match thumbnails.get(path) {
        Some(Some((size, protocol))) if *size == THUMB_SIZE => {
            Image::new(protocol).render(rect, buf);
        },
        Some(Some((_, protocol))) => {
            Image::new(protocol)
                .allow_clipping(true)
                .render(rect, buf);
            visible.push((path.to_path_buf(), THUMB_SIZE));
        },
        Some(None) => {},
        None => visible.push((path.to_path_buf(), THUMB_SIZE)),
    }
}
