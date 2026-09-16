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

use crate::{
    library::Track,
    task::THUMB_SIZE,
    ui::{
        library::{SortDirection, SortKey},
        marquee::Marquee,
    },
    util::DurationExt,
};

pub const ROW_MARGIN: u16 = 1;

pub fn track_table_header(sort_key: SortKey, sort_direction: SortDirection) -> Row<'static> {
    let arrow = match sort_direction {
        SortDirection::Ascending => "▲",
        SortDirection::Descending => "▼",
    };

    let artist_header = format!(
        "Artists{}",
        if sort_key == SortKey::Artist {
            arrow
        } else {
            ""
        }
    );
    let title_header = format!(
        "Title{}",
        if sort_key == SortKey::Title {
            arrow
        } else {
            ""
        }
    );
    let album_header = format!(
        "Album{}",
        if sort_key == SortKey::Album {
            arrow
        } else {
            ""
        }
    );
    let length_header = format!(
        "Length{}",
        if sort_key == SortKey::Length {
            arrow
        } else {
            ""
        }
    );

    Row::new([
        Cell::from(""),
        Cell::from(artist_header),
        Cell::from(title_header),
        Cell::from(album_header),
        Cell::from(Line::from(length_header).right_aligned()), // cell has no right_aligned lol
    ])
}

pub fn track_to_row(
    track: &Track,
    artist_width: usize,
    title_width: usize,
    album_width: usize,
) -> Row<'_> {
    let duration_secs = track.length.as_secs();
    let duration_display = format!("{:02}:{:02}", duration_secs / 60, duration_secs % 60);

    let artist = Marquee::scroll(&track.tags.formatted_artists(), artist_width);
    let title = Marquee::scroll(&track.tags.title, title_width);
    let album = Marquee::scroll(track.tags.album(), album_width);

    let centered_artist = Text::from(vec![Line::from(""), Line::from(artist)]);
    let centered_title = Text::from(vec![Line::from(""), Line::from(title)]);
    let centered_album = Text::from(vec![Line::from(""), Line::from(album)]);
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
    let album_name = track.tags.album();

    let line_title = Line::from(Span::styled(
        Marquee::scroll(track.tags.title.as_str(), width),
        Style::default().add_modifier(Modifier::BOLD),
    ));

    let line_artist = Line::from(Span::styled(
        Marquee::scroll(&track.tags.formatted_artists(), width),
        Style::default().fg(Color::Gray),
    ));

    let max_album = width.saturating_sub(time.len() + 1);

    let album_display = Marquee::scroll(album_name, max_album);
    let album_width = Line::from(album_display.as_str()).width();

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
