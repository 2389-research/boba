// ABOUTME: Deprecated single-line text input component, now a thin wrapper around TextArea.
// ABOUTME: Preserved for backward compatibility; use TextArea with .with_single_line(true) instead.

//! Single-line text input component with autocomplete suggestions, validation,
//! undo/redo, and multiple echo modes (normal, password, hidden).
//!
//! This module is deprecated. Use [`crate::text_area::TextArea`] with
//! `.with_single_line(true)` for all new code.

use boba_core::command::Command;
use boba_core::component::Component;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::Block;
use ratatui::Frame;

use crate::text_area::{self, TextArea, TextAreaStyle};

/// Controls how input text is displayed.
#[derive(Debug, Clone, Default)]
pub enum EchoMode {
    /// Display characters as typed.
    #[default]
    Normal,
    /// Display each character as the given mask character.
    Password(char),
    /// Display nothing.
    Hidden,
}

/// Style configuration for the text input.
#[derive(Debug, Clone)]
pub struct TextInputStyle {
    /// Style applied to the prompt string.
    pub prompt: Style,
    /// Style applied to the input text.
    pub text: Style,
    /// Style applied to the placeholder text.
    pub placeholder: Style,
    /// Style applied to the cursor character.
    pub cursor: Style,
    /// Style applied to autocomplete suggestion ghost text.
    pub suggestion: Style,
}

impl Default for TextInputStyle {
    fn default() -> Self {
        Self {
            prompt: Style::default().fg(Color::Cyan),
            text: Style::default(),
            placeholder: Style::default().fg(Color::DarkGray),
            cursor: Style::default().add_modifier(Modifier::REVERSED),
            suggestion: Style::default().fg(Color::DarkGray),
        }
    }
}

/// Messages for the text input component.
#[derive(Debug, Clone)]
pub enum Message {
    /// A keyboard event to process.
    KeyPress(KeyEvent),
    /// Paste text at cursor position.
    Paste(String),
    /// Emitted when the input value changes.
    Changed(String),
    /// Emitted when Enter is pressed.
    Submit(String),
}

/// A single-line text input component.
///
/// This is a thin wrapper around [`TextArea`] configured in single-line mode.
/// All new code should use `TextArea` directly with `.with_single_line(true)`.
///
/// # Example
///
/// ```ignore
/// let mut input = TextInput::new("Type here...")
///     .with_prompt("> ")
///     .with_char_limit(80)
///     .with_suggestions(vec!["apple".into(), "banana".into()]);
///
/// input.focus();
///
/// // In your parent component's update method, forward messages:
/// // let cmd = input.update(msg);
///
/// // In your parent component's view method, delegate rendering:
/// // input.view(frame, area);
/// ```
#[deprecated(
    since = "0.2.0",
    note = "Use TextArea with .with_single_line(true) instead"
)]
pub struct TextInput {
    inner: TextArea,
}

#[allow(deprecated)]
impl TextInput {
    /// Create a new text input with the given placeholder text.
    pub fn new(placeholder: impl Into<String>) -> Self {
        let inner = TextArea::new()
            .with_single_line(true)
            .with_line_numbers(false)
            .with_placeholder(placeholder);
        Self { inner }
    }

    /// Enable input history with the given maximum number of entries.
    ///
    /// When enabled, Up/Down keys browse through previously submitted
    /// inputs (shell-like behavior).
    pub fn with_history(mut self, max_entries: usize) -> Self {
        self.inner = self.inner.with_history(max_entries);
        self
    }

    /// Push a value into the input history.
    ///
    /// Typically called after the user submits input. Empty strings
    /// and consecutive duplicates are ignored.
    pub fn push_history(&mut self, entry: impl Into<String>) {
        self.inner.push_history(entry);
    }

    /// Get a reference to the input history, if enabled.
    pub fn history(&self) -> Option<&boba_core::input_history::InputHistory> {
        self.inner.history()
    }

    /// Set a prompt string displayed before the input (e.g., `> `).
    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.inner = self.inner.with_prompt(prompt);
        self
    }

    /// Set the echo mode (normal, password, or hidden).
    pub fn with_echo_mode(mut self, mode: EchoMode) -> Self {
        self.inner = self.inner.with_echo_mode(convert_echo_mode(mode));
        self
    }

    /// Set the maximum number of characters allowed.
    pub fn with_char_limit(mut self, limit: usize) -> Self {
        self.inner = self.inner.with_char_limit(limit);
        self
    }

    /// Set custom styles for the input.
    pub fn with_style(mut self, style: TextInputStyle) -> Self {
        self.inner = self.inner.with_style(convert_style(style));
        self
    }

    /// Wrap the input in the given block (border/title).
    ///
    /// By default the input renders borderless. Use this method when you want
    /// the widget itself to draw a surrounding [`Block`].
    pub fn with_block(mut self, block: Block<'static>) -> Self {
        self.inner = self.inner.with_block(block);
        self
    }

    /// Set a validation function called after every change. Returns `Ok(())` or `Err(message)`.
    pub fn with_validate(
        mut self,
        f: impl Fn(&str) -> Result<(), String> + Send + 'static,
    ) -> Self {
        self.inner = self.inner.with_validate(f);
        self
    }

    /// Set the list of autocomplete suggestions. Filtered automatically as the user types.
    pub fn set_suggestions(&mut self, suggestions: Vec<String>) {
        self.inner.set_suggestions(suggestions);
    }

    /// Set autocomplete suggestions (builder variant).
    pub fn with_suggestions(mut self, suggestions: Vec<String>) -> Self {
        self.inner = self.inner.with_suggestions(suggestions);
        self
    }

    /// Get the currently highlighted suggestion, if any.
    pub fn current_suggestion(&self) -> Option<&str> {
        self.inner.current_suggestion()
    }

    /// Get all suggestions matching the current input.
    pub fn available_suggestions(&self) -> &[String] {
        self.inner.available_suggestions()
    }

    /// Enable or disable suggestion display.
    pub fn show_suggestions(&mut self, show: bool) {
        self.inner.show_suggestions(show);
    }

    /// Give this input keyboard focus.
    pub fn focus(&mut self) {
        self.inner.focus();
    }

    /// Remove keyboard focus.
    pub fn blur(&mut self) {
        self.inner.blur();
    }

    /// Get the current input value as a String.
    pub fn value(&self) -> String {
        self.inner.value()
    }

    /// Programmatically set the input value and move cursor to end.
    pub fn set_value(&mut self, value: &str) {
        self.inner.set_value(value);
        // TextInput historically moves cursor to end after set_value.
        self.inner.set_cursor(value.len());
    }

    /// Programmatically set the cursor position.
    /// The position is clamped to `0..=value.len()`.
    pub fn set_cursor(&mut self, pos: usize) {
        self.inner.set_cursor(pos);
    }

    /// Clear the input value and reset cursor to position 0.
    pub fn reset(&mut self) {
        self.inner.reset();
    }

    /// Move cursor to the start of the input.
    pub fn cursor_start(&mut self) {
        self.inner.set_cursor(0);
    }

    /// Move cursor to the end of the input.
    pub fn cursor_end(&mut self) {
        let len = self.inner.value().len();
        self.inner.set_cursor(len);
    }

    /// Return the current cursor position (character index).
    pub fn cursor_position(&self) -> usize {
        self.inner.cursor_position()
    }

    /// Return the current validation error, if any.
    pub fn err(&self) -> Option<&str> {
        self.inner.err()
    }

    /// Whether the input value is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Return the number of characters in the input value.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Return the display value based on the current echo mode.
    #[cfg(test)]
    fn display_value(&self) -> String {
        match self.inner.echo_mode() {
            text_area::EchoMode::Normal => self.inner.value(),
            text_area::EchoMode::Password(c) => c.to_string().repeat(self.inner.len()),
            text_area::EchoMode::Hidden => String::new(),
        }
    }

    /// Translate a text_input key event, remapping readline bindings that
    /// TextArea handles differently (Ctrl+A, Ctrl+E, Alt+B, Alt+F).
    fn translate_key(key: KeyEvent) -> KeyEvent {
        match (key.code, key.modifiers) {
            // TextInput: Ctrl+A = move to start of line (readline Home)
            // TextArea: Ctrl+A = select all
            // Remap to Home key.
            (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
                KeyEvent::new(KeyCode::Home, KeyModifiers::NONE)
            }
            // TextInput: Ctrl+E = move to end of line (readline End)
            // TextArea has no Ctrl+E handler.
            // Remap to End key.
            (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
                KeyEvent::new(KeyCode::End, KeyModifiers::NONE)
            }
            // TextInput: Alt+B = word left (readline)
            // TextArea has no Alt+B handler; it uses Alt+Left.
            (KeyCode::Char('b'), KeyModifiers::ALT) => {
                KeyEvent::new(KeyCode::Left, KeyModifiers::ALT)
            }
            // TextInput: Alt+F = word right (readline)
            // TextArea has no Alt+F handler; it uses Alt+Right.
            (KeyCode::Char('f'), KeyModifiers::ALT) => {
                KeyEvent::new(KeyCode::Right, KeyModifiers::ALT)
            }
            _ => key,
        }
    }
}

#[allow(deprecated)]
impl Component for TextInput {
    type Message = Message;

    fn update(&mut self, msg: Message) -> Command<Message> {
        let inner_msg = match msg {
            Message::KeyPress(key) => {
                let translated = Self::translate_key(key);
                text_area::Message::KeyPress(translated)
            }
            Message::Paste(s) => text_area::Message::Paste(s),
            // Changed and Submit are output-only messages; no-op if received.
            Message::Changed(_) | Message::Submit(_) => return Command::none(),
        };

        let cmd = self.inner.update(inner_msg);
        // Map the returned Command from text_area::Message to text_input::Message.
        cmd.map(|ta_msg| match ta_msg {
            text_area::Message::Changed(s) => Message::Changed(s),
            text_area::Message::Submit(s) => Message::Submit(s),
            text_area::Message::KeyPress(k) => Message::KeyPress(k),
            text_area::Message::Paste(s) => Message::Paste(s),
            // TextInput never had Copy/Cut; drop them.
            text_area::Message::Copy(_) | text_area::Message::Cut(_) => {
                Message::Changed(String::new())
            }
        })
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        self.inner.view(frame, area);
    }

    fn focused(&self) -> bool {
        self.inner.focused()
    }
}

/// Convert a text_input EchoMode to a text_area EchoMode.
fn convert_echo_mode(mode: EchoMode) -> text_area::EchoMode {
    match mode {
        EchoMode::Normal => text_area::EchoMode::Normal,
        EchoMode::Password(c) => text_area::EchoMode::Password(c),
        EchoMode::Hidden => text_area::EchoMode::Hidden,
    }
}

/// Convert a TextInputStyle to a TextAreaStyle.
fn convert_style(style: TextInputStyle) -> TextAreaStyle {
    TextAreaStyle {
        text: style.text,
        cursor: style.cursor,
        line_number: Style::default().fg(Color::DarkGray),
        selection: Style::default(),
        prompt: style.prompt,
        placeholder: style.placeholder,
        suggestion: style.suggestion,
    }
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use super::*;
    use boba_core::component::Component;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn key_ctrl(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn key_alt(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::ALT,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn new_text_input_is_empty() {
        let input = TextInput::new("placeholder");
        assert_eq!(input.value(), "");
    }

    #[test]
    fn typing_characters() {
        let mut input = TextInput::new("");
        input.focus();
        input.update(Message::KeyPress(key(KeyCode::Char('h'))));
        input.update(Message::KeyPress(key(KeyCode::Char('i'))));
        assert_eq!(input.value(), "hi");
    }

    #[test]
    fn backspace_deletes_char() {
        let mut input = TextInput::new("");
        input.focus();
        input.update(Message::KeyPress(key(KeyCode::Char('a'))));
        input.update(Message::KeyPress(key(KeyCode::Char('b'))));
        input.update(Message::KeyPress(key(KeyCode::Backspace)));
        assert_eq!(input.value(), "a");
    }

    #[test]
    fn cursor_movement() {
        let mut input = TextInput::new("");
        input.focus();
        input.update(Message::KeyPress(key(KeyCode::Char('a'))));
        input.update(Message::KeyPress(key(KeyCode::Char('b'))));
        input.update(Message::KeyPress(key(KeyCode::Char('c'))));
        // Cursor at end (3), move left
        input.update(Message::KeyPress(key(KeyCode::Left)));
        input.update(Message::KeyPress(key(KeyCode::Left)));
        // Now insert between a and b
        input.update(Message::KeyPress(key(KeyCode::Char('x'))));
        assert_eq!(input.value(), "axbc");
    }

    #[test]
    fn home_end_keys() {
        let mut input = TextInput::new("");
        input.focus();
        input.set_value("hello");
        input.update(Message::KeyPress(key(KeyCode::Home)));
        input.update(Message::KeyPress(key(KeyCode::Char('!'))));
        assert_eq!(input.value(), "!hello");

        input.update(Message::KeyPress(key(KeyCode::End)));
        input.update(Message::KeyPress(key(KeyCode::Char('!'))));
        assert_eq!(input.value(), "!hello!");
    }

    #[test]
    fn ctrl_u_clears_to_start() {
        let mut input = TextInput::new("");
        input.focus();
        input.set_value("hello world");
        // Cursor is at end (11), ctrl+u clears everything before cursor
        input.update(Message::KeyPress(key_ctrl(KeyCode::Char('u'))));
        assert_eq!(input.value(), "");
    }

    #[test]
    fn ctrl_k_clears_to_end() {
        let mut input = TextInput::new("");
        input.focus();
        input.set_value("hello world");
        // Move cursor to position 5
        input.update(Message::KeyPress(key(KeyCode::Home)));
        for _ in 0..5 {
            input.update(Message::KeyPress(key(KeyCode::Right)));
        }
        input.update(Message::KeyPress(key_ctrl(KeyCode::Char('k'))));
        assert_eq!(input.value(), "hello");
    }

    #[test]
    fn char_limit() {
        let mut input = TextInput::new("").with_char_limit(3);
        input.focus();
        input.update(Message::KeyPress(key(KeyCode::Char('a'))));
        input.update(Message::KeyPress(key(KeyCode::Char('b'))));
        input.update(Message::KeyPress(key(KeyCode::Char('c'))));
        input.update(Message::KeyPress(key(KeyCode::Char('d'))));
        assert_eq!(input.value(), "abc");
    }

    #[test]
    fn unfocused_ignores_keys() {
        let mut input = TextInput::new("");
        // Don't focus
        input.update(Message::KeyPress(key(KeyCode::Char('a'))));
        assert_eq!(input.value(), "");
    }

    #[test]
    fn password_mode() {
        let mut input = TextInput::new("").with_echo_mode(EchoMode::Password('*'));
        input.focus();
        input.update(Message::KeyPress(key(KeyCode::Char('s'))));
        input.update(Message::KeyPress(key(KeyCode::Char('e'))));
        input.update(Message::KeyPress(key(KeyCode::Char('c'))));
        assert_eq!(input.value(), "sec");
        assert_eq!(input.display_value(), "***");
    }

    #[test]
    fn reset_clears_value() {
        let mut input = TextInput::new("");
        input.set_value("hello");
        input.reset();
        assert_eq!(input.value(), "");
    }

    #[test]
    fn enter_produces_submit() {
        let mut input = TextInput::new("");
        input.focus();
        input.set_value("hello");
        // We test by sending Enter, then checking model state.
        // The submit value is returned as a Command::message(Message::Submit(...))
        // but since we can't inspect Command internals from outside the crate,
        // we just verify the behavior works via the message flow.
        let _cmd = input.update(Message::KeyPress(key(KeyCode::Enter)));
        // Value should still be "hello" (enter doesn't clear)
        assert_eq!(input.value(), "hello");
    }

    #[test]
    fn ctrl_left_moves_to_previous_word_boundary() {
        let mut input = TextInput::new("");
        input.focus();
        input.set_value("hello world foo");
        // Cursor is at position 15 (end)
        assert_eq!(input.cursor_position(), 15);

        // Ctrl+Left should jump to start of "foo" (position 12)
        input.update(Message::KeyPress(key_ctrl(KeyCode::Left)));
        assert_eq!(input.cursor_position(), 12);

        // Ctrl+Left should jump to start of "world" (position 6)
        input.update(Message::KeyPress(key_ctrl(KeyCode::Left)));
        assert_eq!(input.cursor_position(), 6);

        // Ctrl+Left should jump to start of "hello" (position 0)
        input.update(Message::KeyPress(key_ctrl(KeyCode::Left)));
        assert_eq!(input.cursor_position(), 0);

        // Already at start, should stay at 0
        input.update(Message::KeyPress(key_ctrl(KeyCode::Left)));
        assert_eq!(input.cursor_position(), 0);
    }

    #[test]
    fn ctrl_right_moves_to_next_word_boundary() {
        let mut input = TextInput::new("");
        input.focus();
        input.set_value("hello world foo");
        input.cursor_start();
        assert_eq!(input.cursor_position(), 0);

        // Ctrl+Right should jump past "hello" and whitespace to "world" (position 6)
        input.update(Message::KeyPress(key_ctrl(KeyCode::Right)));
        assert_eq!(input.cursor_position(), 6);

        // Ctrl+Right should jump past "world" and whitespace to "foo" (position 12)
        input.update(Message::KeyPress(key_ctrl(KeyCode::Right)));
        assert_eq!(input.cursor_position(), 12);

        // Ctrl+Right should jump to end (position 15)
        input.update(Message::KeyPress(key_ctrl(KeyCode::Right)));
        assert_eq!(input.cursor_position(), 15);
    }

    #[test]
    fn alt_left_right_word_movement() {
        let mut input = TextInput::new("");
        input.focus();
        input.set_value("one two");
        assert_eq!(input.cursor_position(), 7);

        // Alt+Left should move to start of "two"
        input.update(Message::KeyPress(key_alt(KeyCode::Left)));
        assert_eq!(input.cursor_position(), 4);

        // Alt+Right should move past "two" to end
        input.update(Message::KeyPress(key_alt(KeyCode::Right)));
        assert_eq!(input.cursor_position(), 7);
    }

    #[test]
    fn alt_b_f_readline_word_movement() {
        let mut input = TextInput::new("");
        input.focus();
        input.set_value("one two");
        assert_eq!(input.cursor_position(), 7);

        // Alt+B should move to start of "two"
        input.update(Message::KeyPress(key_alt(KeyCode::Char('b'))));
        assert_eq!(input.cursor_position(), 4);

        // Alt+F should move past "two" to end
        input.update(Message::KeyPress(key_alt(KeyCode::Char('f'))));
        assert_eq!(input.cursor_position(), 7);
    }

    #[test]
    fn delete_word_forward_alt_d() {
        let mut input = TextInput::new("");
        input.focus();
        input.set_value("hello world");
        input.cursor_start();
        // Alt+D deletes "hello"
        input.update(Message::KeyPress(key_alt(KeyCode::Char('d'))));
        assert_eq!(input.value(), " world");
        assert_eq!(input.cursor_position(), 0);

        // Alt+D deletes " " (non-alnum) then "world"
        input.update(Message::KeyPress(key_alt(KeyCode::Char('d'))));
        assert_eq!(input.value(), "");
    }

    #[test]
    fn delete_word_forward_ctrl_delete() {
        let mut input = TextInput::new("");
        input.focus();
        input.set_value("foo bar");
        input.cursor_start();
        input.update(Message::KeyPress(key_ctrl(KeyCode::Delete)));
        assert_eq!(input.value(), " bar");
    }

    #[test]
    fn paste_inserts_at_cursor() {
        let mut input = TextInput::new("");
        input.focus();
        input.set_value("hd");
        // Move cursor between h and d
        input.update(Message::KeyPress(key(KeyCode::Home)));
        input.update(Message::KeyPress(key(KeyCode::Right)));
        // Paste "ello worl"
        input.update(Message::Paste("ello worl".into()));
        assert_eq!(input.value(), "hello world");
        assert_eq!(input.cursor_position(), 10);
    }

    #[test]
    fn paste_respects_char_limit() {
        let mut input = TextInput::new("").with_char_limit(5);
        input.focus();
        input.set_value("ab");
        // Paste "cdefgh" but only 3 chars should fit (limit 5, already 2)
        input.update(Message::Paste("cdefgh".into()));
        assert_eq!(input.value(), "abcde");
        assert_eq!(input.len(), 5);
    }

    #[test]
    fn paste_when_unfocused_is_ignored() {
        let mut input = TextInput::new("");
        // Do not focus
        input.update(Message::Paste("hello".into()));
        assert_eq!(input.value(), "");
    }

    #[test]
    fn validation_sets_error() {
        let mut input = TextInput::new("").with_validate(|v| {
            if v.is_empty() {
                Err("required".into())
            } else {
                Ok(())
            }
        });
        input.focus();
        // Initially no error (validation runs on update, not on construction)
        assert!(input.err().is_none());

        // Type a character, value is "a" -> valid
        input.update(Message::KeyPress(key(KeyCode::Char('a'))));
        assert!(input.err().is_none());

        // Delete it, value is "" -> invalid
        input.update(Message::KeyPress(key(KeyCode::Backspace)));
        assert_eq!(input.err(), Some("required"));
    }

    #[test]
    fn validation_clears_error_when_valid() {
        let mut input = TextInput::new("").with_validate(|v| {
            if v.len() < 3 {
                Err("too short".into())
            } else {
                Ok(())
            }
        });
        input.focus();
        input.update(Message::KeyPress(key(KeyCode::Char('a'))));
        assert_eq!(input.err(), Some("too short"));

        input.update(Message::KeyPress(key(KeyCode::Char('b'))));
        assert_eq!(input.err(), Some("too short"));

        input.update(Message::KeyPress(key(KeyCode::Char('c'))));
        assert!(input.err().is_none());
    }

    #[test]
    fn validation_runs_on_paste() {
        let mut input = TextInput::new("").with_validate(|v| {
            if v.contains(' ') {
                Err("no spaces".into())
            } else {
                Ok(())
            }
        });
        input.focus();
        input.update(Message::Paste("hello world".into()));
        assert_eq!(input.err(), Some("no spaces"));
    }

    #[test]
    fn cursor_position_tracking() {
        let mut input = TextInput::new("");
        input.focus();
        assert_eq!(input.cursor_position(), 0);

        input.update(Message::KeyPress(key(KeyCode::Char('a'))));
        assert_eq!(input.cursor_position(), 1);

        input.update(Message::KeyPress(key(KeyCode::Char('b'))));
        assert_eq!(input.cursor_position(), 2);

        input.update(Message::KeyPress(key(KeyCode::Char('c'))));
        assert_eq!(input.cursor_position(), 3);

        input.update(Message::KeyPress(key(KeyCode::Left)));
        assert_eq!(input.cursor_position(), 2);

        input.cursor_start();
        assert_eq!(input.cursor_position(), 0);

        input.cursor_end();
        assert_eq!(input.cursor_position(), 3);
    }

    #[test]
    fn is_empty_and_len() {
        let mut input = TextInput::new("");
        assert!(input.is_empty());
        assert_eq!(input.len(), 0);

        input.set_value("hello");
        assert!(!input.is_empty());
        assert_eq!(input.len(), 5);

        input.reset();
        assert!(input.is_empty());
        assert_eq!(input.len(), 0);
    }

    #[test]
    fn undo_basic() {
        let mut input = TextInput::new("");
        input.focus();
        input.update(Message::KeyPress(key(KeyCode::Char('a'))));
        input.update(Message::KeyPress(key(KeyCode::Char('b'))));
        input.update(Message::KeyPress(key(KeyCode::Char('c'))));
        assert_eq!(input.value(), "abc");

        // Undo 'c'
        input.update(Message::KeyPress(key_ctrl(KeyCode::Char('z'))));
        assert_eq!(input.value(), "ab");
        assert_eq!(input.cursor_position(), 2);

        // Undo 'b'
        input.update(Message::KeyPress(key_ctrl(KeyCode::Char('z'))));
        assert_eq!(input.value(), "a");
        assert_eq!(input.cursor_position(), 1);

        // Undo 'a'
        input.update(Message::KeyPress(key_ctrl(KeyCode::Char('z'))));
        assert_eq!(input.value(), "");
        assert_eq!(input.cursor_position(), 0);
    }

    #[test]
    fn undo_then_redo() {
        let mut input = TextInput::new("");
        input.focus();
        input.update(Message::KeyPress(key(KeyCode::Char('x'))));
        input.update(Message::KeyPress(key(KeyCode::Char('y'))));
        assert_eq!(input.value(), "xy");

        // Undo 'y'
        input.update(Message::KeyPress(key_ctrl(KeyCode::Char('z'))));
        assert_eq!(input.value(), "x");
        assert_eq!(input.cursor_position(), 1);

        // Redo 'y'
        input.update(Message::KeyPress(key_ctrl(KeyCode::Char('y'))));
        assert_eq!(input.value(), "xy");
        assert_eq!(input.cursor_position(), 2);
    }

    #[test]
    fn new_edit_clears_redo_stack() {
        let mut input = TextInput::new("");
        input.focus();
        input.update(Message::KeyPress(key(KeyCode::Char('a'))));
        input.update(Message::KeyPress(key(KeyCode::Char('b'))));
        assert_eq!(input.value(), "ab");

        // Undo 'b'
        input.update(Message::KeyPress(key_ctrl(KeyCode::Char('z'))));
        assert_eq!(input.value(), "a");

        // Type a new character instead of redo
        input.update(Message::KeyPress(key(KeyCode::Char('z'))));
        assert_eq!(input.value(), "az");

        // Redo should now be a no-op because redo stack was cleared
        input.update(Message::KeyPress(key_ctrl(KeyCode::Char('y'))));
        assert_eq!(input.value(), "az");
    }

    #[test]
    fn undo_on_empty_stack_is_noop() {
        let mut input = TextInput::new("");
        input.focus();
        // No edits have been made; undo should do nothing
        input.update(Message::KeyPress(key_ctrl(KeyCode::Char('z'))));
        assert_eq!(input.value(), "");
        assert_eq!(input.cursor_position(), 0);

        // Type something, then undo all, then undo again
        input.update(Message::KeyPress(key(KeyCode::Char('q'))));
        input.update(Message::KeyPress(key_ctrl(KeyCode::Char('z'))));
        assert_eq!(input.value(), "");

        // One more undo on empty stack
        input.update(Message::KeyPress(key_ctrl(KeyCode::Char('z'))));
        assert_eq!(input.value(), "");
        assert_eq!(input.cursor_position(), 0);
    }

    #[test]
    fn history_browse_up_down() {
        let mut input = TextInput::new("").with_history(100);
        input.focus();
        input.push_history("first");
        input.push_history("second");

        // Type something as draft
        input.update(Message::KeyPress(key(KeyCode::Char('d'))));
        assert_eq!(input.value(), "d");

        // Up → most recent history entry
        input.update(Message::KeyPress(key(KeyCode::Up)));
        assert_eq!(input.value(), "second");

        // Up → older entry
        input.update(Message::KeyPress(key(KeyCode::Up)));
        assert_eq!(input.value(), "first");

        // Up at oldest → stays
        input.update(Message::KeyPress(key(KeyCode::Up)));
        assert_eq!(input.value(), "first");

        // Down → newer entry
        input.update(Message::KeyPress(key(KeyCode::Down)));
        assert_eq!(input.value(), "second");

        // Down → back to draft
        input.update(Message::KeyPress(key(KeyCode::Down)));
        assert_eq!(input.value(), "d");

        // Down past draft → no change
        input.update(Message::KeyPress(key(KeyCode::Down)));
        assert_eq!(input.value(), "d");
    }

    #[test]
    fn history_without_history_enabled() {
        let mut input = TextInput::new("");
        input.focus();
        input.set_value("hello");
        // Up/Down should be no-ops when history is not enabled
        input.update(Message::KeyPress(key(KeyCode::Up)));
        assert_eq!(input.value(), "hello");
        input.update(Message::KeyPress(key(KeyCode::Down)));
        assert_eq!(input.value(), "hello");
    }

    #[test]
    fn history_cursor_moves_to_end() {
        let mut input = TextInput::new("").with_history(100);
        input.focus();
        input.push_history("long entry");

        input.update(Message::KeyPress(key(KeyCode::Up)));
        assert_eq!(input.value(), "long entry");
        assert_eq!(input.cursor_position(), 10); // cursor at end
    }
}
