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
    config::{Column, ColumnSetting},
    library::Track,
    task::THUMB_SIZE,
    ui::{library::SortDirection, marquee::Marquee},
    util::DurationExt,
};

pub const ROW_MARGIN: u16 = 1;

pub fn track_table_header(
    columns: &[ColumnSetting],
    sort_key: Column,
    sort_direction: SortDirection,
) -> Row<'static> {
    let arrow = match sort_direction {
        SortDirection::Ascending => "▲",
        SortDirection::Descending => "▼",
    };

    let cells = columns.iter().map(|col_setting| {
        let col = col_setting.column();
        let label = col_setting.label();
        let text = if col == sort_key {
            format!("{}{}", label, arrow)
        } else {
            label
        };

        let line = Line::from(text).alignment(col_setting.alignment());
        Cell::from(line)
    });

    Row::new(cells)
}

pub fn track_to_row(
    track: &Track,
    columns: &[ColumnSetting],
    col_widths: &[usize],
) -> Row<'static> {
    let cells = columns
        .iter()
        .zip(col_widths)
        .map(|(col_setting, &width)| {
            let align = col_setting.alignment();
            let text = match col_setting.column() {
                Column::Cover => String::new(),
                Column::Artist => Marquee::scroll(&track.tags.formatted_artists(), width),
                Column::Title => Marquee::scroll(track.tags.title.as_str(), width),
                Column::Album => Marquee::scroll(track.tags.album(), width),
                Column::Genre => Marquee::scroll(track.tags.genre(), width),
                Column::Date => Marquee::scroll(track.tags.date(), width),
                Column::Length => track.length.format_time(),
            };

            let line = Line::from(text).alignment(align);
            Cell::from(Text::from(vec![Line::from(""), line]))
        });

    Row::new(cells)
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
    cover_x: u16,
    inner: Rect,
    thumbnails: &mut LruCache<std::path::PathBuf, Option<(Size, Protocol)>>,
    visible: &mut Vec<(std::path::PathBuf, Size)>,
    buf: &mut Buffer,
) where
    I: IntoIterator<Item = &'a Path>,
{
    let first_row_y = if has_header { inner.y + 1 } else { inner.y };

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
            x: cover_x,
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
