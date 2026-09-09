use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols::{self, border},
    widgets::{Block, Paragraph, Tabs, Widget},
};

pub enum SidebarAction {
    /// Sets a filter like `genre:EDM` or `None` to clear the tag
    // Done this way so user-query will still be kept in the search while we can still clear the
    // applied tag
    ApplyFilter {
        key: &'static str,     // "genre", "album", etc.
        value: Option<String>, // Some("EDM") or None for example for All
    },
}

pub enum SidebarTab {
    Genres,
    Albums,
    Artists,
}

pub struct SidebarWidget {
    selected_tab: usize,
}

impl SidebarWidget {
    const TABS: [&str; 3] = ["Genres", "Albums", "Artists"];

    pub fn new() -> Self {
        Self { selected_tab: 0 }
    }

    fn next(&mut self) {
        self.selected_tab = (self.selected_tab + 1) % Self::TABS.len();
    }

    fn prev(&mut self) {
        self.selected_tab = (self.selected_tab + Self::TABS.len() - 1) % Self::TABS.len();
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let block = Block::bordered()
            .border_set(border::THICK)
            .title(" Sidebar ");

        let inner = block.inner(area);
        block.render(area, buf);

        let [top, main] =
            Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(inner);

        // sucks that ratatui has no centering in native tab widget, but nothing we can do
        let tab_areas: [Rect; 3] = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(1),
        ])
        .areas(top);

        for (i, (area, title)) in tab_areas
            .into_iter()
            .zip(Self::TABS)
            .enumerate()
        {
            let style = if i == self.selected_tab {
                // highlighted style
                Style::default()
                    .fg(Color::White)
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Cyan)
            };

            Paragraph::new(title)
                .alignment(Alignment::Center)
                .style(style)
                .render(area, buf);
        }
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent) -> Option<SidebarAction> {
        match key_event.code {
            KeyCode::Char('[') => {
                self.prev();
                None
            },
            KeyCode::Char(']') => {
                self.next();
                None
            },
            _ => None,
        }
    }
}
