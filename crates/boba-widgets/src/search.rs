//! Search overlay component with match navigation.
//!
//! Provides an inline search bar that sits at the bottom of a content area,
//! tracks matches by index, and lets the user navigate between them with
//! `Ctrl+N` / `Ctrl+P`.  The parent is responsible for providing searchable content
//! and reporting match indices — this widget handles the UI and navigation
//! state.
//!
//! # Example
//!
//! ```ignore
//! use boba_widgets::search::Search;
//!
//! let search = Search::new();
//! // In update(), when search query changes:
//! //   search.set_matches(find_matches(&content, search.query()));
//! // In view(), render search bar at bottom of content area:
//! //   search.view(frame, search_bar_area);
//! ```

use boba_core::command::Command;
use boba_core::component::Component;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Messages emitted by the search component.
#[derive(Debug, Clone)]
pub enum Message {
    /// A key press event forwarded to the search bar.
    KeyPress(KeyEvent),
    /// The search query changed (new query string).
    QueryChanged(String),
    /// User requested jump to a match index.
    JumpTo(usize),
    /// Search was activated (entered search mode).
    Activated,
    /// Search was dismissed (Esc).
    Dismissed,
}

/// Style configuration for the search bar.
#[derive(Debug, Clone)]
pub struct SearchStyle {
    /// Style for the "/" prompt character.
    pub prompt: Style,
    /// Style for the query text.
    pub text: Style,
    /// Style for the cursor.
    pub cursor: Style,
    /// Style for the match counter (e.g. "3/15").
    pub counter: Style,
    /// Style for "No matches" text.
    pub no_matches: Style,
    /// Background style for the search bar.
    pub background: Style,
}

impl Default for SearchStyle {
    fn default() -> Self {
        Self {
            prompt: Style::default().fg(Color::Yellow),
            text: Style::default(),
            cursor: Style::default().add_modifier(Modifier::REVERSED),
            counter: Style::default().fg(Color::DarkGray),
            no_matches: Style::default().fg(Color::Red),
            background: Style::default(),
        }
    }
}

/// Inline search bar with match navigation.
pub struct Search {
    active: bool,
    query: String,
    cursor_pos: usize,
    matches: Vec<usize>,
    current_match: usize,
    style: SearchStyle,
    prompt_char: char,
}

impl Default for Search {
    fn default() -> Self {
        Self::new()
    }
}

impl Search {
    /// Create a new search component (initially inactive).
    pub fn new() -> Self {
        Self {
            active: false,
            query: String::new(),
            cursor_pos: 0,
            matches: Vec::new(),
            current_match: 0,
            style: SearchStyle::default(),
            prompt_char: '/',
        }
    }

    /// Set the style configuration.
    pub fn with_style(mut self, style: SearchStyle) -> Self {
        self.style = style;
        self
    }

    /// Set the prompt character (default: `/`).
    pub fn with_prompt(mut self, ch: char) -> Self {
        self.prompt_char = ch;
        self
    }

    /// Whether the search bar is currently active/visible.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Activate the search bar programmatically.
    pub fn activate(&mut self) {
        self.active = true;
        self.query.clear();
        self.cursor_pos = 0;
        self.matches.clear();
        self.current_match = 0;
    }

    /// Deactivate the search bar.
    pub fn deactivate(&mut self) {
        self.active = false;
        self.query.clear();
        self.matches.clear();
        self.current_match = 0;
    }

    /// Get the current search query.
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Set the match indices (call this whenever the query or content changes).
    pub fn set_matches(&mut self, matches: Vec<usize>) {
        self.matches = matches;
        if self.current_match >= self.matches.len() {
            self.current_match = 0;
        }
    }

    /// Get the current match index (into the matches vec).
    pub fn current_match_index(&self) -> usize {
        self.current_match
    }

    /// Get the value at the current match (the content index), if any.
    pub fn current_match_value(&self) -> Option<usize> {
        self.matches.get(self.current_match).copied()
    }

    /// Get the total number of matches.
    pub fn match_count(&self) -> usize {
        self.matches.len()
    }

    /// Get all match indices.
    pub fn matches(&self) -> &[usize] {
        &self.matches
    }

    /// Navigate to the next match (wraps around).
    fn next_match(&mut self) {
        if !self.matches.is_empty() {
            self.current_match = (self.current_match + 1) % self.matches.len();
        }
    }

    /// Navigate to the previous match (wraps around).
    fn prev_match(&mut self) {
        if !self.matches.is_empty() {
            if self.current_match == 0 {
                self.current_match = self.matches.len() - 1;
            } else {
                self.current_match -= 1;
            }
        }
    }

    /// Convert a char index to a byte offset in the query string.
    fn byte_offset(s: &str, char_idx: usize) -> usize {
        s.char_indices()
            .nth(char_idx)
            .map(|(i, _)| i)
            .unwrap_or(s.len())
    }

    /// Number of characters in the query.
    fn char_len(&self) -> usize {
        self.query.chars().count()
    }
}

impl Component for Search {
    type Message = Message;

    fn update(&mut self, msg: Message) -> Command<Message> {
        match msg {
            Message::KeyPress(key) => {
                if !self.active {
                    return Command::none();
                }

                match (key.code, key.modifiers) {
                    (KeyCode::Esc, _) => {
                        self.deactivate();
                        Command::message(Message::Dismissed)
                    }
                    (KeyCode::Enter, _) => {
                        if let Some(idx) = self.current_match_value() {
                            Command::message(Message::JumpTo(idx))
                        } else {
                            Command::none()
                        }
                    }
                    (KeyCode::Char('n'), KeyModifiers::CONTROL) if !self.matches.is_empty() => {
                        self.next_match();
                        if let Some(idx) = self.current_match_value() {
                            Command::message(Message::JumpTo(idx))
                        } else {
                            Command::none()
                        }
                    }
                    (KeyCode::Char('p'), KeyModifiers::CONTROL) if !self.matches.is_empty() => {
                        self.prev_match();
                        if let Some(idx) = self.current_match_value() {
                            Command::message(Message::JumpTo(idx))
                        } else {
                            Command::none()
                        }
                    }
                    (KeyCode::Backspace, _) => {
                        if self.query.is_empty() {
                            self.deactivate();
                            Command::message(Message::Dismissed)
                        } else {
                            if self.cursor_pos > 0 {
                                self.cursor_pos -= 1;
                                let byte_pos = Self::byte_offset(&self.query, self.cursor_pos);
                                self.query.remove(byte_pos);
                            }
                            Command::message(Message::QueryChanged(self.query.clone()))
                        }
                    }
                    (KeyCode::Left, _) => {
                        if self.cursor_pos > 0 {
                            self.cursor_pos -= 1;
                        }
                        Command::none()
                    }
                    (KeyCode::Right, _) => {
                        if self.cursor_pos < self.char_len() {
                            self.cursor_pos += 1;
                        }
                        Command::none()
                    }
                    (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                        let byte_pos = Self::byte_offset(&self.query, self.cursor_pos);
                        self.query.insert(byte_pos, c);
                        self.cursor_pos += 1;
                        Command::message(Message::QueryChanged(self.query.clone()))
                    }
                    _ => Command::none(),
                }
            }
            Message::Activated => {
                self.activate();
                Command::none()
            }
            Message::Dismissed | Message::QueryChanged(_) | Message::JumpTo(_) => Command::none(),
        }
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        if !self.active || area.height == 0 {
            return;
        }

        let mut spans = Vec::new();

        // Prompt
        spans.push(Span::styled(
            format!("{} ", self.prompt_char),
            self.style.prompt,
        ));

        // Query text with cursor
        if self.query.is_empty() {
            spans.push(Span::styled(" ", self.style.cursor));
        } else {
            let byte_pos = Self::byte_offset(&self.query, self.cursor_pos);
            let char_count = self.query.chars().count();
            let before = &self.query[..byte_pos];
            if !before.is_empty() {
                spans.push(Span::styled(before.to_string(), self.style.text));
            }
            if self.cursor_pos < char_count {
                let next_byte = Self::byte_offset(&self.query, self.cursor_pos + 1);
                let cursor_char = &self.query[byte_pos..next_byte];
                spans.push(Span::styled(cursor_char.to_string(), self.style.cursor));
                let after = &self.query[next_byte..];
                if !after.is_empty() {
                    spans.push(Span::styled(after.to_string(), self.style.text));
                }
            } else {
                spans.push(Span::styled(" ", self.style.cursor));
            }
        }

        // Match counter
        spans.push(Span::raw("  "));
        if self.query.is_empty() {
            // Don't show counter for empty query
        } else if self.matches.is_empty() {
            spans.push(Span::styled("No matches", self.style.no_matches));
        } else {
            spans.push(Span::styled(
                format!("{}/{}", self.current_match + 1, self.matches.len()),
                self.style.counter,
            ));
        }

        let paragraph = Paragraph::new(Line::from(spans)).style(self.style.background);
        frame.render_widget(paragraph, area);
    }

    fn focused(&self) -> bool {
        self.active
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEventKind, KeyEventState};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn ctrl_key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn initially_inactive() {
        let search = Search::new();
        assert!(!search.is_active());
        assert!(search.query().is_empty());
    }

    #[test]
    fn activate_and_deactivate() {
        let mut search = Search::new();
        search.activate();
        assert!(search.is_active());
        search.deactivate();
        assert!(!search.is_active());
    }

    #[test]
    fn typing_updates_query() {
        let mut search = Search::new();
        search.activate();
        let cmd = search.update(Message::KeyPress(key(KeyCode::Char('h'))));
        assert_eq!(search.query(), "h");
        assert!(matches!(cmd.into_message(), Some(Message::QueryChanged(_))));

        search.update(Message::KeyPress(key(KeyCode::Char('i'))));
        assert_eq!(search.query(), "hi");
    }

    #[test]
    fn esc_dismisses() {
        let mut search = Search::new();
        search.activate();
        search.update(Message::KeyPress(key(KeyCode::Char('a'))));
        let cmd = search.update(Message::KeyPress(key(KeyCode::Esc)));
        assert!(!search.is_active());
        assert!(matches!(cmd.into_message(), Some(Message::Dismissed)));
    }

    #[test]
    fn backspace_on_empty_dismisses() {
        let mut search = Search::new();
        search.activate();
        let cmd = search.update(Message::KeyPress(key(KeyCode::Backspace)));
        assert!(!search.is_active());
        assert!(matches!(cmd.into_message(), Some(Message::Dismissed)));
    }

    #[test]
    fn backspace_removes_char() {
        let mut search = Search::new();
        search.activate();
        search.update(Message::KeyPress(key(KeyCode::Char('a'))));
        search.update(Message::KeyPress(key(KeyCode::Char('b'))));
        assert_eq!(search.query(), "ab");
        search.update(Message::KeyPress(key(KeyCode::Backspace)));
        assert_eq!(search.query(), "a");
        assert!(search.is_active());
    }

    #[test]
    fn match_navigation() {
        let mut search = Search::new();
        search.activate();
        search.update(Message::KeyPress(key(KeyCode::Char('x'))));
        search.set_matches(vec![0, 5, 10]);
        assert_eq!(search.current_match_index(), 0);
        assert_eq!(search.current_match_value(), Some(0));

        // Ctrl+N → next
        search.update(Message::KeyPress(ctrl_key(KeyCode::Char('n'))));
        assert_eq!(search.current_match_index(), 1);
        assert_eq!(search.current_match_value(), Some(5));

        // Ctrl+N → next
        search.update(Message::KeyPress(ctrl_key(KeyCode::Char('n'))));
        assert_eq!(search.current_match_index(), 2);

        // Ctrl+N → wraps to 0
        search.update(Message::KeyPress(ctrl_key(KeyCode::Char('n'))));
        assert_eq!(search.current_match_index(), 0);

        // Ctrl+P → wraps to last
        search.update(Message::KeyPress(ctrl_key(KeyCode::Char('p'))));
        assert_eq!(search.current_match_index(), 2);
    }

    #[test]
    fn enter_jumps_to_current() {
        let mut search = Search::new();
        search.activate();
        search.update(Message::KeyPress(key(KeyCode::Char('q'))));
        search.set_matches(vec![42]);
        let cmd = search.update(Message::KeyPress(key(KeyCode::Enter)));
        match cmd.into_message() {
            Some(Message::JumpTo(42)) => {}
            other => panic!(
                "Expected JumpTo(42), got {:?}",
                other.map(|m| format!("{:?}", m))
            ),
        }
    }

    #[test]
    fn no_matches_enter_is_noop() {
        let mut search = Search::new();
        search.activate();
        search.update(Message::KeyPress(key(KeyCode::Char('q'))));
        // No matches set
        let cmd = search.update(Message::KeyPress(key(KeyCode::Enter)));
        assert!(cmd.is_none());
    }

    #[test]
    fn set_matches_clamps_current() {
        let mut search = Search::new();
        search.activate();
        search.update(Message::KeyPress(key(KeyCode::Char('a'))));
        search.set_matches(vec![1, 2, 3]);
        search.next_match();
        search.next_match(); // current = 2
        assert_eq!(search.current_match_index(), 2);

        // Now set fewer matches — current should clamp
        search.set_matches(vec![1]);
        assert_eq!(search.current_match_index(), 0);
    }

    #[test]
    fn multibyte_chars_do_not_panic() {
        let mut search = Search::new();
        search.activate();
        // Type multi-byte chars: "café"
        search.update(Message::KeyPress(key(KeyCode::Char('c'))));
        search.update(Message::KeyPress(key(KeyCode::Char('a'))));
        search.update(Message::KeyPress(key(KeyCode::Char('f'))));
        search.update(Message::KeyPress(key(KeyCode::Char('é'))));
        assert_eq!(search.query(), "café");
        assert_eq!(search.cursor_pos, 4); // 4 chars

        // Move left, then right
        search.update(Message::KeyPress(key(KeyCode::Left)));
        assert_eq!(search.cursor_pos, 3);
        search.update(Message::KeyPress(key(KeyCode::Right)));
        assert_eq!(search.cursor_pos, 4);

        // Backspace the 'é'
        search.update(Message::KeyPress(key(KeyCode::Backspace)));
        assert_eq!(search.query(), "caf");
    }

    #[test]
    fn inactive_ignores_keys() {
        let mut search = Search::new();
        // Don't activate — keys should be ignored
        let cmd = search.update(Message::KeyPress(key(KeyCode::Char('a'))));
        assert!(cmd.is_none());
        assert!(search.query().is_empty());
    }
}
