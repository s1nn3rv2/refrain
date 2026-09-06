mod library;
mod ui;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::Stylize,
    symbols::border,
    text::Line,
    widgets::{Block, Widget},
};

use crate::{
    library::LibraryState,
    ui::{library::LibraryWidgetState, transport::TransportState},
};

#[derive(Default)]
pub struct App {
    transport: TransportState,
    library: LibraryState,
    library_widget: LibraryWidgetState,
    quit: bool,
}

impl App {
    fn new() -> Self {
        Self {
            quit: false,
            library: LibraryState::new(),
            library_widget: LibraryWidgetState::default(),
            transport: TransportState::default(),
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> color_eyre::Result<()> {
        while !self.quit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }

        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        let [main_area, transport_area] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(3)]).areas(frame.area());

        let title = Line::from(" Test ".bold());
        let block = Block::bordered()
            .title(title.centered())
            .border_set(border::THICK);

        frame.render_widget(block, main_area);

        self.library_widget
            .render(&self.library, main_area, frame.buffer_mut());
        frame.render_widget(&self.transport, transport_area);
    }

    fn handle_events(&mut self) -> color_eyre::Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            },
            _ => {},
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            _ => self
                .library_widget
                .handle_key_event(key_event),
        }
    }

    fn exit(&mut self) {
        self.quit = true;
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {}
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    ratatui::run(|terminal| App::new().run(terminal))
}
