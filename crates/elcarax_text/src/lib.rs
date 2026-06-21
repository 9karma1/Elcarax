//! Text editing boundary for Elcarax.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextBuffer {
    text: String,
    caret_byte_index: usize,
}

impl TextBuffer {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            caret_byte_index: 0,
        }
    }

    pub fn insert_str(&mut self, input: &str) {
        self.text.insert_str(self.caret_byte_index, input);
        self.caret_byte_index += input.len();
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn caret_byte_index(&self) -> usize {
        self.caret_byte_index
    }
}
