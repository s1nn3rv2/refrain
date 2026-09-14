use crate::{ui::input::TextInput, util};

pub struct SearchBar {
    pub input: TextInput,
}

impl SearchBar {
    pub fn new() -> Self {
        Self {
            input: TextInput::new("Press '/' to search..."),
        }
    }

    pub fn set_tag(&mut self, key: &str, value: Option<&str>) {
        let tag_prefix = format!("{key}:");

        let mut terms: Vec<String> = util::tokenize(&self.input.value)
            .into_iter()
            .filter(|token| !token.starts_with(&tag_prefix))
            .collect();

        if let Some(val) = value {
            // if has spaces (more than 1 word), surround it in quotes
            let tag = if val.contains(' ') {
                format!("{tag_prefix}\"{val}\"")
            } else {
                format!("{tag_prefix}{val}")
            };
            terms.insert(0, tag);
        }

        self.input.value = terms.join(" ");
        self.input.character_index = self.input.value.chars().count();
    }
}
