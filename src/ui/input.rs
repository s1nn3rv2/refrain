use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Position, Rect},
    style::{Color, Style},
    widgets::{Block, Padding, Paragraph},
};

pub enum InputAction {
    None,
    Changed,   // modified
    Submitted, // pressed Enter
    Escaped,   // pressed Esc
}

pub struct TextInput {
    pub value: String,
    pub character_index: usize,
    pub is_focused: bool,
    pub placeholder: String,
    pub title: String,
}

impl TextInput {
    pub fn new(title: impl Into<String>, placeholder: impl Into<String>) -> Self {
        Self {
            value: String::new(),
            character_index: 0,
            is_focused: false,
            placeholder: placeholder.into(),
            title: title.into(),
        }
    }

    pub fn focus(&mut self) {
        self.is_focused = true;
        self.character_index = self.value.chars().count(); // place cursor at the end
    }

    pub fn unfocus(&mut self) {
        self.is_focused = false;
    }

    fn move_cursor_left(&mut self) {
        let cursor_moved_left = self
            .character_index
            .saturating_sub(1);
        self.character_index = self.clamp_cursor(cursor_moved_left);
    }

    fn move_cursor_right(&mut self) {
        let cursor_moved_right = self
            .character_index
            .saturating_add(1);
        self.character_index = self.clamp_cursor(cursor_moved_right);
    }

    /// Returns the byte index based on the character position.
    ///
    /// Since each character in a string can contain multiple bytes, it's necessary to calculate
    /// the byte index based on the index of the character.
    fn byte_index(&self) -> usize {
        self.value
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.character_index)
            .unwrap_or(self.value.len())
    }

    fn enter_char(&mut self, new_char: char) {
        let index = self.byte_index();
        self.value.insert(index, new_char);
        self.move_cursor_right();
    }

    /// Returns true if query text changed
    fn delete_char(&mut self) -> bool {
        if self.character_index != 0 {
            self.move_cursor_left();
            let byte_idx = self.byte_index();
            self.value.remove(byte_idx); // docs dont use remove because it works on bytes, not chars, but if we use byte_index, we can avoid that
            true
        } else {
            false
        }
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.value.chars().count())
    }

    const fn reset_cursor(&mut self) {
        self.character_index = 0;
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> InputAction {
        if !self.is_focused {
            return InputAction::None;
        }

        match key.code {
            KeyCode::Esc => {
                self.is_focused = false;
                InputAction::Escaped
            },
            KeyCode::Enter => {
                self.is_focused = false;
                InputAction::Submitted
            },
            KeyCode::Left => {
                self.move_cursor_left();
                InputAction::None
            },
            KeyCode::Right => {
                self.move_cursor_right();
                InputAction::None
            },
            KeyCode::Backspace => {
                if self.delete_char() {
                    InputAction::Changed
                } else {
                    InputAction::None
                }
            },
            KeyCode::Char(c) => {
                self.enter_char(c);
                InputAction::Changed
            },
            _ => InputAction::None,
        }
    }

    pub fn render(&self, frame: &mut ratatui::Frame, area: Rect) {
        let border_color = if self.is_focused {
            Color::Cyan
        } else {
            Color::DarkGray
        };

        let block = Block::default()
            .border_style(Style::default().fg(border_color))
            .padding(Padding::horizontal(1));
        let inner = block.inner(area);

        frame.render_widget(block, area);

        let display_text = if self.value.is_empty() && !self.is_focused {
            self.placeholder.as_str()
        } else {
            self.value.as_str()
        };

        frame.render_widget(Paragraph::new(display_text), inner);

        // blinking cursor
        if self.is_focused {
            let cursor_x = inner.x + self.character_index as u16;
            let cursor_y = inner.y;
            frame.set_cursor_position(Position::new(cursor_x, cursor_y));
        }
    }
}
