use crate::color::Color;

/// Text input widget component.
///
/// Attach to an entity together with `UiNode`.
/// `UiSystem` sets focus on click and updates the text by consuming the character buffer.
pub struct TextInput {
    pub text: String,
    /// UTF-8 byte index
    pub cursor: usize,
    pub focused: bool,
    pub placeholder: String,
    pub max_len: usize,
    /// Accumulated dt. Toggles cursor_visible every 0.5 seconds.
    pub cursor_blink: f32,
    pub cursor_visible: bool,
    /// String currently being composed by the IME. Rendered as a preview only, before commit.
    pub preedit: String,

    pub color_normal: Color,
    pub color_focused: Color,
    pub text_color: Color,
    pub font_size: f32,
}

impl TextInput {
    pub fn new(placeholder: impl Into<String>) -> Self {
        Self {
            text: String::new(),
            cursor: 0,
            focused: false,
            placeholder: placeholder.into(),
            max_len: 256,
            cursor_blink: 0.0,
            cursor_visible: true,
            preedit: String::new(),
            color_normal: Color::rgba(0.15, 0.15, 0.20, 1.0),
            color_focused: Color::rgba(0.20, 0.25, 0.35, 1.0),
            text_color: Color::rgba_u8(220, 220, 220, 255),
            font_size: 16.0,
        }
    }

    pub fn with_max_len(mut self, n: usize) -> Self {
        self.max_len = n;
        self
    }

    pub fn with_font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    pub fn current_color(&self) -> Color {
        if self.focused {
            self.color_focused
        } else {
            self.color_normal
        }
    }

    /// Deletes the character immediately before the cursor (UTF-8 safe).
    pub fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let mut char_start = self.cursor - 1;
        while !self.text.is_char_boundary(char_start) {
            char_start -= 1;
        }
        self.text.drain(char_start..self.cursor);
        self.cursor = char_start;
    }

    /// Moves the cursor one character boundary to the left (UTF-8 safe).
    pub fn move_left(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let mut i = self.cursor - 1;
        while !self.text.is_char_boundary(i) {
            i -= 1;
        }
        self.cursor = i;
    }

    /// Moves the cursor one character boundary to the right (UTF-8 safe).
    pub fn move_right(&mut self) {
        if self.cursor >= self.text.len() {
            return;
        }
        let mut i = self.cursor + 1;
        while i < self.text.len() && !self.text.is_char_boundary(i) {
            i += 1;
        }
        self.cursor = i;
    }

    /// Moves the cursor to the beginning of the text.
    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    /// Moves the cursor to the end of the text.
    pub fn move_end(&mut self) {
        self.cursor = self.text.len();
    }

    /// Deletes the character at the cursor position (forward delete, UTF-8 safe).
    pub fn delete_forward(&mut self) {
        if self.cursor >= self.text.len() {
            return;
        }
        let mut end = self.cursor + 1;
        while end < self.text.len() && !self.text.is_char_boundary(end) {
            end += 1;
        }
        self.text.drain(self.cursor..end);
    }

    /// Inserts a character at the cursor position.
    pub fn insert_char(&mut self, c: char) {
        if self.text.len() + c.len_utf8() <= self.max_len {
            self.text.insert(self.cursor, c);
            self.cursor += c.len_utf8();
        }
    }

    pub fn insert_str(&mut self, s: &str) {
        for ch in s.chars() {
            self.insert_char(ch);
        }
    }

    pub fn text_with_preedit(&self) -> String {
        if self.preedit.is_empty() {
            return self.text.clone();
        }
        let mut out = self.text.clone();
        out.insert_str(self.cursor, &self.preedit);
        out
    }

    /// Builds the string the renderer shows: placeholder when empty and unfocused,
    /// otherwise the text with the IME preedit and a caret inserted at the real
    /// cursor position — not pinned to the end. `cursor` is always on a char boundary,
    /// so the splits are UTF-8 safe.
    ///
    /// While focused the caret slot is *always reserved* (a space when the blink is
    /// off, `|` when on), so blinking does not shift the trailing text back and forth.
    pub fn display_with_caret(&self, focused: bool, blink_on: bool) -> String {
        if self.text.is_empty() && self.preedit.is_empty() && !focused {
            return self.placeholder.clone();
        }
        let (head, tail) = self.text.split_at(self.cursor);
        let caret = if !focused {
            ""
        } else if blink_on {
            "|"
        } else {
            " "
        };
        format!("{head}{}{caret}{tail}", self.preedit)
    }

    /// Remaining byte capacity before `max_len`. Zero means the field is full.
    pub fn remaining_capacity(&self) -> usize {
        self.max_len.saturating_sub(self.text.len())
    }

    /// Byte offset of the caret within the string from [`Self::display_with_caret`] —
    /// directly after the head text and the IME preedit. The renderer uses this to
    /// measure the caret's x for single-line horizontal scrolling.
    pub fn caret_display_offset(&self) -> usize {
        self.cursor + self.preedit.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_str_respects_utf8_max_len() {
        let mut input = TextInput::new("").with_max_len("한글".len());

        input.insert_str("한글!");

        assert_eq!(input.text, "한글");
    }

    #[test]
    fn cursor_movement_is_utf8_safe() {
        let mut input = TextInput::new("");
        input.insert_str("한a글"); // 3 + 1 + 3 bytes, cursor at end (7)
        assert_eq!(input.cursor, 7);

        input.move_left(); // before "글"
        assert_eq!(input.cursor, 4);
        input.move_left(); // before "a"
        assert_eq!(input.cursor, 3);
        input.move_home();
        assert_eq!(input.cursor, 0);

        input.move_right(); // past "한"
        assert_eq!(input.cursor, 3);
        input.delete_forward(); // removes "a"
        assert_eq!(input.text, "한글");
        assert_eq!(input.cursor, 3);

        input.move_end();
        assert_eq!(input.cursor, input.text.len());
        input.move_right(); // clamped at end
        assert_eq!(input.cursor, input.text.len());
    }

    #[test]
    fn display_with_caret_tracks_cursor() {
        let mut input = TextInput::new("type…");
        // Empty + unfocused → placeholder.
        assert_eq!(input.display_with_caret(false, false), "type…");

        input.focused = true;
        input.insert_str("한글"); // cursor at end (6 bytes)
        assert_eq!(input.display_with_caret(true, true), "한글|");
        // Blink off keeps a reserved space so the text does not shift.
        assert_eq!(input.display_with_caret(true, false), "한글 ");

        input.move_left(); // caret between 한 and 글
        assert_eq!(input.display_with_caret(true, true), "한|글");

        // Preedit composes at the caret, the bar sits after it.
        input.preedit = "ㄱ".to_string();
        assert_eq!(input.display_with_caret(true, true), "한ㄱ|글");
    }

    #[test]
    fn text_with_preedit_inserts_preview_at_cursor() {
        let mut input = TextInput::new("");
        input.insert_str("글");
        input.cursor = 0;
        input.preedit = "한".to_string();

        assert_eq!(input.text_with_preedit(), "한글");
        assert_eq!(input.text, "글");
    }

    #[test]
    fn remaining_capacity_tracks_max_len() {
        let mut input = TextInput::new("").with_max_len("한글".len()); // 6 bytes
        assert_eq!(input.remaining_capacity(), 6);
        input.insert_str("한"); // 3 bytes
        assert_eq!(input.remaining_capacity(), 3);
        input.insert_str("글"); // full
        assert_eq!(input.remaining_capacity(), 0);
        // Further input is rejected, capacity stays 0 (no underflow).
        input.insert_str("!");
        assert_eq!(input.remaining_capacity(), 0);
        assert_eq!(input.text, "한글");
    }

    #[test]
    fn caret_display_offset_accounts_for_preedit() {
        let mut input = TextInput::new("");
        input.insert_str("ab"); // cursor at 2
        assert_eq!(input.caret_display_offset(), 2);
        input.move_left(); // cursor at 1
        input.preedit = "ㄱ".to_string(); // 3 bytes inserted at cursor
                                          // display = "a" + "ㄱ" + caret + "b"; caret sits at byte 1 + 3 = 4.
        assert_eq!(input.caret_display_offset(), 1 + "ㄱ".len());
    }
}
