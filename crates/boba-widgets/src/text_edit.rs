//! Shared single-line text editing state.
//!
//! `TextEditState` provides character buffer management, cursor movement,
//! word-boundary navigation, kill operations, and undo/redo for single-line
//! text editing. Used internally by [`TextInput`](crate::text_input::TextInput)
//! and [`Search`](crate::search::Search).

use std::collections::VecDeque;

/// Single-line text editing state with undo/redo support.
pub struct TextEditState {
    chars: Vec<char>,
    cursor: usize,
    undo_stack: VecDeque<(Vec<char>, usize)>,
    redo_stack: VecDeque<(Vec<char>, usize)>,
}

impl TextEditState {
    /// Create a new empty editing state.
    pub fn new() -> Self {
        Self {
            chars: Vec::new(),
            cursor: 0,
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
        }
    }

    /// Get the current value as a String.
    pub fn value(&self) -> String {
        self.chars.iter().collect()
    }

    /// Set the value and move cursor to end.
    pub fn set_value(&mut self, s: &str) {
        self.chars = s.chars().collect();
        self.cursor = self.chars.len();
    }

    /// Get the character buffer.
    pub fn chars(&self) -> &[char] {
        &self.chars
    }

    /// Current cursor position (char index, 0-based).
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Set cursor position, clamped to 0..=len.
    pub fn set_cursor(&mut self, pos: usize) {
        self.cursor = pos.min(self.chars.len());
    }

    /// Number of characters.
    pub fn len(&self) -> usize {
        self.chars.len()
    }

    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }

    /// Save current state to undo stack.
    pub fn push_undo(&mut self) {
        self.undo_stack.push_back((self.chars.clone(), self.cursor));
        self.redo_stack.clear();
        if self.undo_stack.len() > 100 {
            self.undo_stack.pop_front();
        }
    }

    /// Insert a character at cursor, advance cursor.
    /// Returns true if the character was inserted.
    pub fn insert_char(&mut self, c: char) -> bool {
        self.chars.insert(self.cursor, c);
        self.cursor += 1;
        true
    }

    /// Insert a string at cursor, advance cursor.
    /// If `max_len` is Some, limits total length. Returns number of chars inserted.
    pub fn insert_str(&mut self, s: &str, max_len: Option<usize>) -> usize {
        let chars: Vec<char> = s.chars().collect();
        let available = if let Some(limit) = max_len {
            limit.saturating_sub(self.chars.len())
        } else {
            chars.len()
        };
        let to_insert = &chars[..available.min(chars.len())];
        for (i, &c) in to_insert.iter().enumerate() {
            self.chars.insert(self.cursor + i, c);
        }
        self.cursor += to_insert.len();
        to_insert.len()
    }

    /// Delete character before cursor (backspace).
    /// Returns true if a character was deleted.
    pub fn delete_back(&mut self) -> bool {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.chars.remove(self.cursor);
            true
        } else {
            false
        }
    }

    /// Delete character at cursor (delete key).
    /// Returns true if a character was deleted.
    pub fn delete_forward(&mut self) -> bool {
        if self.cursor < self.chars.len() {
            self.chars.remove(self.cursor);
            true
        } else {
            false
        }
    }

    /// Delete word backward (Alt+Backspace / Ctrl+W).
    /// Returns true if anything was deleted.
    pub fn delete_word_back(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        let start_cursor = self.cursor;
        let start_len = self.chars.len();
        // Skip spaces
        while self.cursor > 0 && self.chars[self.cursor - 1] == ' ' {
            self.cursor -= 1;
            self.chars.remove(self.cursor);
        }
        // Delete word chars
        while self.cursor > 0 && self.chars[self.cursor - 1] != ' ' {
            self.cursor -= 1;
            self.chars.remove(self.cursor);
        }
        self.cursor != start_cursor || self.chars.len() != start_len
    }

    /// Delete word forward (Alt+D).
    /// Returns true if anything was deleted.
    pub fn delete_word_forward(&mut self) -> bool {
        if self.cursor >= self.chars.len() {
            return false;
        }
        let start_len = self.chars.len();
        // Skip non-alphanumeric
        while self.cursor < self.chars.len() && !self.chars[self.cursor].is_alphanumeric() {
            self.chars.remove(self.cursor);
        }
        // Delete alphanumeric
        while self.cursor < self.chars.len() && self.chars[self.cursor].is_alphanumeric() {
            self.chars.remove(self.cursor);
        }
        self.chars.len() != start_len
    }

    /// Move cursor left one character.
    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    /// Move cursor right one character.
    pub fn move_right(&mut self) {
        if self.cursor < self.chars.len() {
            self.cursor += 1;
        }
    }

    /// Move cursor to start.
    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    /// Move cursor to end.
    pub fn move_end(&mut self) {
        self.cursor = self.chars.len();
    }

    /// Move cursor to previous word boundary.
    pub fn word_left(&mut self) {
        if self.cursor == 0 {
            return;
        }
        while self.cursor > 0 && !self.chars[self.cursor - 1].is_alphanumeric() {
            self.cursor -= 1;
        }
        while self.cursor > 0 && self.chars[self.cursor - 1].is_alphanumeric() {
            self.cursor -= 1;
        }
    }

    /// Move cursor to next word boundary.
    pub fn word_right(&mut self) {
        let len = self.chars.len();
        if self.cursor >= len {
            return;
        }
        while self.cursor < len && self.chars[self.cursor].is_alphanumeric() {
            self.cursor += 1;
        }
        while self.cursor < len && !self.chars[self.cursor].is_alphanumeric() {
            self.cursor += 1;
        }
    }

    /// Kill from cursor to start of line (Ctrl+U).
    /// Returns true if anything was killed.
    pub fn kill_to_start(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        self.chars.drain(..self.cursor);
        self.cursor = 0;
        true
    }

    /// Kill from cursor to end of line (Ctrl+K).
    /// Returns true if anything was killed.
    pub fn kill_to_end(&mut self) -> bool {
        if self.cursor >= self.chars.len() {
            return false;
        }
        self.chars.truncate(self.cursor);
        true
    }

    /// Undo last change.
    pub fn undo(&mut self) -> bool {
        if let Some((value, cursor)) = self.undo_stack.pop_back() {
            self.redo_stack.push_back((self.chars.clone(), self.cursor));
            self.chars = value;
            self.cursor = cursor;
            true
        } else {
            false
        }
    }

    /// Redo last undone change.
    pub fn redo(&mut self) -> bool {
        if let Some((value, cursor)) = self.redo_stack.pop_back() {
            self.undo_stack.push_back((self.chars.clone(), self.cursor));
            self.chars = value;
            self.cursor = cursor;
            true
        } else {
            false
        }
    }

    /// Clear the buffer and reset cursor.
    pub fn reset(&mut self) {
        self.chars.clear();
        self.cursor = 0;
    }
}

impl Default for TextEditState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_state_is_empty() {
        let state = TextEditState::new();
        assert!(state.is_empty());
        assert_eq!(state.len(), 0);
        assert_eq!(state.cursor(), 0);
        assert_eq!(state.value(), "");
    }

    #[test]
    fn insert_char_advances_cursor() {
        let mut state = TextEditState::new();
        assert!(state.insert_char('h'));
        assert!(state.insert_char('i'));
        assert_eq!(state.value(), "hi");
        assert_eq!(state.cursor(), 2);
    }

    #[test]
    fn insert_char_at_middle() {
        let mut state = TextEditState::new();
        state.set_value("ac");
        state.set_cursor(1);
        state.insert_char('b');
        assert_eq!(state.value(), "abc");
        assert_eq!(state.cursor(), 2);
    }

    #[test]
    fn delete_back() {
        let mut state = TextEditState::new();
        state.set_value("ab");
        assert!(state.delete_back());
        assert_eq!(state.value(), "a");
        assert_eq!(state.cursor(), 1);
    }

    #[test]
    fn delete_back_at_start_is_noop() {
        let mut state = TextEditState::new();
        state.set_value("a");
        state.set_cursor(0);
        assert!(!state.delete_back());
        assert_eq!(state.value(), "a");
    }

    #[test]
    fn delete_forward() {
        let mut state = TextEditState::new();
        state.set_value("ab");
        state.set_cursor(0);
        assert!(state.delete_forward());
        assert_eq!(state.value(), "b");
        assert_eq!(state.cursor(), 0);
    }

    #[test]
    fn delete_forward_at_end_is_noop() {
        let mut state = TextEditState::new();
        state.set_value("a");
        assert!(!state.delete_forward());
        assert_eq!(state.value(), "a");
    }

    #[test]
    fn move_left_and_right() {
        let mut state = TextEditState::new();
        state.set_value("abc");
        assert_eq!(state.cursor(), 3);

        state.move_left();
        assert_eq!(state.cursor(), 2);

        state.move_left();
        assert_eq!(state.cursor(), 1);

        state.move_right();
        assert_eq!(state.cursor(), 2);
    }

    #[test]
    fn move_left_at_zero_stays() {
        let mut state = TextEditState::new();
        state.move_left();
        assert_eq!(state.cursor(), 0);
    }

    #[test]
    fn move_right_at_end_stays() {
        let mut state = TextEditState::new();
        state.set_value("a");
        state.move_right();
        assert_eq!(state.cursor(), 1);
    }

    #[test]
    fn move_home_and_end() {
        let mut state = TextEditState::new();
        state.set_value("hello");
        assert_eq!(state.cursor(), 5);

        state.move_home();
        assert_eq!(state.cursor(), 0);

        state.move_end();
        assert_eq!(state.cursor(), 5);
    }

    #[test]
    fn word_left() {
        let mut state = TextEditState::new();
        state.set_value("hello world foo");
        // cursor at 15 (end)

        state.word_left();
        assert_eq!(state.cursor(), 12); // start of "foo"

        state.word_left();
        assert_eq!(state.cursor(), 6); // start of "world"

        state.word_left();
        assert_eq!(state.cursor(), 0); // start of "hello"

        state.word_left();
        assert_eq!(state.cursor(), 0); // already at start
    }

    #[test]
    fn word_right() {
        let mut state = TextEditState::new();
        state.set_value("hello world foo");
        state.set_cursor(0);

        state.word_right();
        assert_eq!(state.cursor(), 6); // after "hello "

        state.word_right();
        assert_eq!(state.cursor(), 12); // after "world "

        state.word_right();
        assert_eq!(state.cursor(), 15); // end

        state.word_right();
        assert_eq!(state.cursor(), 15); // already at end
    }

    #[test]
    fn delete_word_back() {
        let mut state = TextEditState::new();
        state.set_value("hello world");
        // cursor at end (11)
        assert!(state.delete_word_back());
        assert_eq!(state.value(), "hello ");

        assert!(state.delete_word_back());
        assert_eq!(state.value(), "");
    }

    #[test]
    fn delete_word_back_at_start_is_noop() {
        let mut state = TextEditState::new();
        state.set_value("hello");
        state.set_cursor(0);
        assert!(!state.delete_word_back());
        assert_eq!(state.value(), "hello");
    }

    #[test]
    fn delete_word_forward() {
        let mut state = TextEditState::new();
        state.set_value("hello world");
        state.set_cursor(0);

        assert!(state.delete_word_forward());
        assert_eq!(state.value(), " world");
        assert_eq!(state.cursor(), 0);

        assert!(state.delete_word_forward());
        assert_eq!(state.value(), "");
    }

    #[test]
    fn delete_word_forward_at_end_is_noop() {
        let mut state = TextEditState::new();
        state.set_value("hello");
        assert!(!state.delete_word_forward());
        assert_eq!(state.value(), "hello");
    }

    #[test]
    fn kill_to_start() {
        let mut state = TextEditState::new();
        state.set_value("hello world");
        state.set_cursor(5);
        assert!(state.kill_to_start());
        assert_eq!(state.value(), " world");
        assert_eq!(state.cursor(), 0);
    }

    #[test]
    fn kill_to_start_at_beginning_is_noop() {
        let mut state = TextEditState::new();
        state.set_value("hello");
        state.set_cursor(0);
        assert!(!state.kill_to_start());
        assert_eq!(state.value(), "hello");
    }

    #[test]
    fn kill_to_end() {
        let mut state = TextEditState::new();
        state.set_value("hello world");
        state.set_cursor(5);
        assert!(state.kill_to_end());
        assert_eq!(state.value(), "hello");
        assert_eq!(state.cursor(), 5);
    }

    #[test]
    fn kill_to_end_at_end_is_noop() {
        let mut state = TextEditState::new();
        state.set_value("hello");
        assert!(!state.kill_to_end());
        assert_eq!(state.value(), "hello");
    }

    #[test]
    fn undo_basic() {
        let mut state = TextEditState::new();
        state.push_undo();
        state.insert_char('a');
        state.push_undo();
        state.insert_char('b');
        state.push_undo();
        state.insert_char('c');
        assert_eq!(state.value(), "abc");

        assert!(state.undo());
        assert_eq!(state.value(), "ab");
        assert_eq!(state.cursor(), 2);

        assert!(state.undo());
        assert_eq!(state.value(), "a");
        assert_eq!(state.cursor(), 1);

        assert!(state.undo());
        assert_eq!(state.value(), "");
        assert_eq!(state.cursor(), 0);

        // No more undo
        assert!(!state.undo());
        assert_eq!(state.value(), "");
    }

    #[test]
    fn undo_then_redo() {
        let mut state = TextEditState::new();
        state.push_undo();
        state.insert_char('x');
        state.push_undo();
        state.insert_char('y');
        assert_eq!(state.value(), "xy");

        assert!(state.undo());
        assert_eq!(state.value(), "x");

        assert!(state.redo());
        assert_eq!(state.value(), "xy");
        assert_eq!(state.cursor(), 2);
    }

    #[test]
    fn new_edit_clears_redo_stack() {
        let mut state = TextEditState::new();
        state.push_undo();
        state.insert_char('a');
        state.push_undo();
        state.insert_char('b');
        assert_eq!(state.value(), "ab");

        // Undo 'b'
        state.undo();
        assert_eq!(state.value(), "a");

        // New edit clears redo
        state.push_undo();
        state.insert_char('z');
        assert_eq!(state.value(), "az");

        // Redo should be no-op
        assert!(!state.redo());
        assert_eq!(state.value(), "az");
    }

    #[test]
    fn undo_on_empty_stack_is_noop() {
        let mut state = TextEditState::new();
        assert!(!state.undo());
        assert_eq!(state.value(), "");
        assert_eq!(state.cursor(), 0);
    }

    #[test]
    fn redo_on_empty_stack_is_noop() {
        let mut state = TextEditState::new();
        assert!(!state.redo());
    }

    #[test]
    fn insert_str_basic() {
        let mut state = TextEditState::new();
        let n = state.insert_str("hello", None);
        assert_eq!(n, 5);
        assert_eq!(state.value(), "hello");
        assert_eq!(state.cursor(), 5);
    }

    #[test]
    fn insert_str_at_middle() {
        let mut state = TextEditState::new();
        state.set_value("hd");
        state.set_cursor(1);
        let n = state.insert_str("ello worl", None);
        assert_eq!(n, 9);
        assert_eq!(state.value(), "hello world");
        assert_eq!(state.cursor(), 10);
    }

    #[test]
    fn insert_str_with_max_len() {
        let mut state = TextEditState::new();
        state.set_value("ab");
        let n = state.insert_str("cdefgh", Some(5));
        assert_eq!(n, 3);
        assert_eq!(state.value(), "abcde");
        assert_eq!(state.len(), 5);
    }

    #[test]
    fn insert_str_at_limit_inserts_nothing() {
        let mut state = TextEditState::new();
        state.set_value("abcde");
        let n = state.insert_str("x", Some(5));
        assert_eq!(n, 0);
        assert_eq!(state.value(), "abcde");
    }

    #[test]
    fn set_value_moves_cursor_to_end() {
        let mut state = TextEditState::new();
        state.set_value("hello");
        assert_eq!(state.cursor(), 5);
        assert_eq!(state.value(), "hello");
    }

    #[test]
    fn set_cursor_clamps() {
        let mut state = TextEditState::new();
        state.set_value("hi");
        state.set_cursor(100);
        assert_eq!(state.cursor(), 2);
    }

    #[test]
    fn reset_clears_everything() {
        let mut state = TextEditState::new();
        state.set_value("hello");
        state.reset();
        assert!(state.is_empty());
        assert_eq!(state.cursor(), 0);
        assert_eq!(state.value(), "");
    }

    #[test]
    fn default_is_empty() {
        let state = TextEditState::default();
        assert!(state.is_empty());
        assert_eq!(state.cursor(), 0);
    }

    #[test]
    fn chars_returns_buffer() {
        let mut state = TextEditState::new();
        state.set_value("abc");
        assert_eq!(state.chars(), &['a', 'b', 'c']);
    }
}
