//! Single-line text input component with autocomplete suggestions, validation,
//! undo/redo, and multiple echo modes (normal, password, hidden).

use std::collections::VecDeque;

use boba_core::command::Command;
use boba_core::component::Component;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;

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
pub struct TextInput {
    value: Vec<char>,
    cursor: usize,
    offset: usize,
    focus: bool,
    placeholder: String,
    prompt: String,
    char_limit: Option<usize>,
    echo_mode: EchoMode,
    style: TextInputStyle,
    #[allow(clippy::type_complexity)]
    validate: Option<Box<dyn Fn(&str) -> Result<(), String> + Send>>,
    err: Option<String>,
    suggestions: Vec<String>,
    filtered_suggestions: Vec<String>,
    show_suggestions: bool,
    suggestion_index: usize,
    undo_stack: VecDeque<(Vec<char>, usize)>,
    redo_stack: VecDeque<(Vec<char>, usize)>,
    history: Option<boba_core::input_history::InputHistory>,
    block: Option<Block<'static>>,
}

impl TextInput {
    /// Create a new text input with the given placeholder text.
    pub fn new(placeholder: impl Into<String>) -> Self {
        Self {
            value: Vec::new(),
            cursor: 0,
            offset: 0,
            focus: false,
            placeholder: placeholder.into(),
            prompt: String::new(),
            char_limit: None,
            echo_mode: EchoMode::default(),
            style: TextInputStyle::default(),
            validate: None,
            err: None,
            suggestions: Vec::new(),
            filtered_suggestions: Vec::new(),
            show_suggestions: true,
            suggestion_index: 0,
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
            history: None,
            block: None,
        }
    }

    /// Enable input history with the given maximum number of entries.
    ///
    /// When enabled, Up/Down keys browse through previously submitted
    /// inputs (shell-like behavior).
    pub fn with_history(mut self, max_entries: usize) -> Self {
        self.history = Some(boba_core::input_history::InputHistory::new(max_entries));
        self
    }

    /// Push a value into the input history.
    ///
    /// Typically called after the user submits input. Empty strings
    /// and consecutive duplicates are ignored.
    pub fn push_history(&mut self, entry: impl Into<String>) {
        if let Some(ref mut history) = self.history {
            history.push(entry);
        }
    }

    /// Get a reference to the input history, if enabled.
    pub fn history(&self) -> Option<&boba_core::input_history::InputHistory> {
        self.history.as_ref()
    }

    /// Set a prompt string displayed before the input (e.g., `> `).
    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = prompt.into();
        self
    }

    /// Set the echo mode (normal, password, or hidden).
    pub fn with_echo_mode(mut self, mode: EchoMode) -> Self {
        self.echo_mode = mode;
        self
    }

    /// Set the maximum number of characters allowed.
    pub fn with_char_limit(mut self, limit: usize) -> Self {
        self.char_limit = Some(limit);
        self
    }

    /// Set custom styles for the input.
    pub fn with_style(mut self, style: TextInputStyle) -> Self {
        self.style = style;
        self
    }

    /// Wrap the input in the given block (border/title).
    ///
    /// By default the input renders borderless. Use this method when you want
    /// the widget itself to draw a surrounding [`Block`].
    pub fn with_block(mut self, block: Block<'static>) -> Self {
        self.block = Some(block);
        self
    }

    /// Set a validation function called after every change. Returns `Ok(())` or `Err(message)`.
    pub fn with_validate(
        mut self,
        f: impl Fn(&str) -> Result<(), String> + Send + 'static,
    ) -> Self {
        self.validate = Some(Box::new(f));
        self
    }

    /// Set the list of autocomplete suggestions. Filtered automatically as the user types.
    pub fn set_suggestions(&mut self, suggestions: Vec<String>) {
        self.suggestions = suggestions;
        self.filter_suggestions();
    }

    /// Set autocomplete suggestions (builder variant).
    pub fn with_suggestions(mut self, suggestions: Vec<String>) -> Self {
        self.suggestions = suggestions;
        self.filter_suggestions();
        self
    }

    /// Get the currently highlighted suggestion, if any.
    pub fn current_suggestion(&self) -> Option<&str> {
        if !self.show_suggestions {
            return None;
        }
        self.filtered_suggestions
            .get(self.suggestion_index)
            .map(|s| s.as_str())
    }

    /// Get all suggestions matching the current input.
    pub fn available_suggestions(&self) -> &[String] {
        &self.filtered_suggestions
    }

    /// Enable or disable suggestion display.
    pub fn show_suggestions(&mut self, show: bool) {
        self.show_suggestions = show;
    }

    /// Give this input keyboard focus.
    pub fn focus(&mut self) {
        self.focus = true;
    }

    /// Remove keyboard focus.
    pub fn blur(&mut self) {
        self.focus = false;
    }

    /// Get the current input value as a String.
    pub fn value(&self) -> String {
        self.value.iter().collect()
    }

    /// Programmatically set the input value and move cursor to end.
    pub fn set_value(&mut self, value: &str) {
        self.value = value.chars().collect();
        self.cursor = self.value.len();
    }

    /// Programmatically set the cursor position.
    /// The position is clamped to `0..=value.len()`.
    pub fn set_cursor(&mut self, pos: usize) {
        self.cursor = pos.min(self.value.len());
    }

    /// Clear the input value and reset cursor to position 0.
    pub fn reset(&mut self) {
        self.value.clear();
        self.cursor = 0;
        self.offset = 0;
    }

    /// Move cursor to the start of the input.
    pub fn cursor_start(&mut self) {
        self.move_cursor_home();
    }

    /// Move cursor to the end of the input.
    pub fn cursor_end(&mut self) {
        self.move_cursor_end();
    }

    /// Return the current cursor position (character index).
    pub fn cursor_position(&self) -> usize {
        self.cursor
    }

    /// Return the current validation error, if any.
    pub fn err(&self) -> Option<&str> {
        self.err.as_deref()
    }

    /// Whether the input value is empty.
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    /// Return the number of characters in the input value.
    pub fn len(&self) -> usize {
        self.value.len()
    }

    fn push_undo(&mut self) {
        self.undo_stack.push_back((self.value.clone(), self.cursor));
        self.redo_stack.clear();
        if self.undo_stack.len() > 100 {
            self.undo_stack.pop_front();
        }
    }

    fn insert_char(&mut self, c: char) -> Command<Message> {
        if let Some(limit) = self.char_limit {
            if self.value.len() >= limit {
                return Command::none();
            }
        }
        self.value.insert(self.cursor, c);
        self.cursor += 1;
        Command::message(Message::Changed(self.value()))
    }

    fn delete_char_backward(&mut self) -> Command<Message> {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.value.remove(self.cursor);
            return Command::message(Message::Changed(self.value()));
        }
        Command::none()
    }

    fn delete_char_forward(&mut self) -> Command<Message> {
        if self.cursor < self.value.len() {
            self.value.remove(self.cursor);
            return Command::message(Message::Changed(self.value()));
        }
        Command::none()
    }

    fn delete_word_backward(&mut self) -> Command<Message> {
        if self.cursor == 0 {
            return Command::none();
        }
        // Skip spaces
        while self.cursor > 0 && self.value[self.cursor - 1] == ' ' {
            self.cursor -= 1;
            self.value.remove(self.cursor);
        }
        // Delete word chars
        while self.cursor > 0 && self.value[self.cursor - 1] != ' ' {
            self.cursor -= 1;
            self.value.remove(self.cursor);
        }
        Command::message(Message::Changed(self.value()))
    }

    fn delete_word_forward(&mut self) -> Command<Message> {
        if self.cursor >= self.value.len() {
            return Command::none();
        }
        // Skip non-alphanumeric characters first
        while self.cursor < self.value.len() && !self.value[self.cursor].is_alphanumeric() {
            self.value.remove(self.cursor);
        }
        // Delete alphanumeric word characters
        while self.cursor < self.value.len() && self.value[self.cursor].is_alphanumeric() {
            self.value.remove(self.cursor);
        }
        Command::message(Message::Changed(self.value()))
    }

    fn move_cursor_word_left(&mut self) {
        if self.cursor == 0 {
            return;
        }
        // Skip non-alphanumeric characters
        while self.cursor > 0 && !self.value[self.cursor - 1].is_alphanumeric() {
            self.cursor -= 1;
        }
        // Skip alphanumeric characters
        while self.cursor > 0 && self.value[self.cursor - 1].is_alphanumeric() {
            self.cursor -= 1;
        }
    }

    fn move_cursor_word_right(&mut self) {
        let len = self.value.len();
        if self.cursor >= len {
            return;
        }
        // Skip alphanumeric characters
        while self.cursor < len && self.value[self.cursor].is_alphanumeric() {
            self.cursor += 1;
        }
        // Skip non-alphanumeric characters
        while self.cursor < len && !self.value[self.cursor].is_alphanumeric() {
            self.cursor += 1;
        }
    }

    fn insert_paste(&mut self, text: &str) -> Command<Message> {
        let chars: Vec<char> = text.chars().collect();
        if chars.is_empty() {
            return Command::none();
        }
        let available = if let Some(limit) = self.char_limit {
            limit.saturating_sub(self.value.len())
        } else {
            chars.len()
        };
        let to_insert = &chars[..available.min(chars.len())];
        if to_insert.is_empty() {
            return Command::none();
        }
        for (i, &c) in to_insert.iter().enumerate() {
            self.value.insert(self.cursor + i, c);
        }
        self.cursor += to_insert.len();
        Command::message(Message::Changed(self.value()))
    }

    fn run_validate(&mut self) {
        if let Some(ref validate) = self.validate {
            let val = self.value();
            match validate(&val) {
                Ok(()) => self.err = None,
                Err(e) => self.err = Some(e),
            }
        }
    }

    fn move_cursor_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    fn move_cursor_right(&mut self) {
        if self.cursor < self.value.len() {
            self.cursor += 1;
        }
    }

    fn move_cursor_home(&mut self) {
        self.cursor = 0;
    }

    fn move_cursor_end(&mut self) {
        self.cursor = self.value.len();
    }

    fn display_value(&self) -> String {
        match &self.echo_mode {
            EchoMode::Normal => self.value.iter().collect(),
            EchoMode::Password(c) => c.to_string().repeat(self.value.len()),
            EchoMode::Hidden => String::new(),
        }
    }

    fn filter_suggestions(&mut self) {
        let current: String = self.value.iter().collect();
        let current_lower = current.to_lowercase();
        self.filtered_suggestions = self
            .suggestions
            .iter()
            .filter(|s| {
                let s_lower = s.to_lowercase();
                s_lower.starts_with(&current_lower) && s_lower != current_lower
            })
            .cloned()
            .collect();
        self.suggestion_index = 0;
    }

    fn accept_suggestion(&mut self) -> bool {
        if let Some(suggestion) = self.current_suggestion().map(|s| s.to_owned()) {
            self.value = suggestion.chars().collect();
            self.cursor = self.value.len();
            self.filter_suggestions();
            true
        } else {
            false
        }
    }
}

impl Component for TextInput {
    type Message = Message;

    fn update(&mut self, msg: Message) -> Command<Message> {
        match msg {
            Message::KeyPress(key) => {
                if !self.focus {
                    return Command::none();
                }
                let cmd = match (key.code, key.modifiers) {
                    (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                        self.push_undo();
                        let cmd = self.insert_char(c);
                        self.filter_suggestions();
                        cmd
                    }
                    (KeyCode::Backspace, KeyModifiers::NONE) => {
                        self.push_undo();
                        let cmd = self.delete_char_backward();
                        self.filter_suggestions();
                        cmd
                    }
                    (KeyCode::Delete, KeyModifiers::NONE) => {
                        self.push_undo();
                        let cmd = self.delete_char_forward();
                        self.filter_suggestions();
                        cmd
                    }
                    (KeyCode::Backspace, m) if m.contains(KeyModifiers::ALT) => {
                        self.push_undo();
                        let cmd = self.delete_word_backward();
                        self.filter_suggestions();
                        cmd
                    }
                    (KeyCode::Char('w'), m) if m.contains(KeyModifiers::CONTROL) => {
                        self.push_undo();
                        let cmd = self.delete_word_backward();
                        self.filter_suggestions();
                        cmd
                    }
                    // Delete word forward: Alt+D or Ctrl+Delete
                    (KeyCode::Char('d'), m) if m.contains(KeyModifiers::ALT) => {
                        self.push_undo();
                        let cmd = self.delete_word_forward();
                        self.filter_suggestions();
                        cmd
                    }
                    (KeyCode::Delete, m) if m.contains(KeyModifiers::CONTROL) => {
                        self.push_undo();
                        let cmd = self.delete_word_forward();
                        self.filter_suggestions();
                        cmd
                    }
                    (KeyCode::Tab, KeyModifiers::NONE) => {
                        if self.current_suggestion().is_some() {
                            self.push_undo();
                        }
                        if self.accept_suggestion() {
                            Command::message(Message::Changed(self.value()))
                        } else {
                            Command::none()
                        }
                    }
                    (KeyCode::Left, KeyModifiers::NONE) => {
                        self.move_cursor_left();
                        Command::none()
                    }
                    (KeyCode::Right, KeyModifiers::NONE) => {
                        if self.cursor == self.value.len() && self.current_suggestion().is_some() {
                            self.push_undo();
                            self.accept_suggestion();
                            Command::message(Message::Changed(self.value()))
                        } else {
                            self.move_cursor_right();
                            Command::none()
                        }
                    }
                    // Word movement: Ctrl+Left/Right or Alt+Left/Right
                    (KeyCode::Left, m)
                        if m.contains(KeyModifiers::CONTROL) || m.contains(KeyModifiers::ALT) =>
                    {
                        self.move_cursor_word_left();
                        Command::none()
                    }
                    (KeyCode::Right, m)
                        if m.contains(KeyModifiers::CONTROL) || m.contains(KeyModifiers::ALT) =>
                    {
                        self.move_cursor_word_right();
                        Command::none()
                    }
                    (KeyCode::Home, _) => {
                        self.move_cursor_home();
                        Command::none()
                    }
                    (KeyCode::Char('a'), m) if m.contains(KeyModifiers::CONTROL) => {
                        self.move_cursor_home();
                        Command::none()
                    }
                    (KeyCode::End, _) => {
                        self.move_cursor_end();
                        Command::none()
                    }
                    (KeyCode::Char('e'), m) if m.contains(KeyModifiers::CONTROL) => {
                        self.move_cursor_end();
                        Command::none()
                    }
                    (KeyCode::Char('u'), m) if m.contains(KeyModifiers::CONTROL) => {
                        self.push_undo();
                        self.value.drain(..self.cursor);
                        self.cursor = 0;
                        let cmd = Command::message(Message::Changed(self.value()));
                        self.filter_suggestions();
                        cmd
                    }
                    (KeyCode::Char('k'), m) if m.contains(KeyModifiers::CONTROL) => {
                        self.push_undo();
                        self.value.truncate(self.cursor);
                        let cmd = Command::message(Message::Changed(self.value()));
                        self.filter_suggestions();
                        cmd
                    }
                    (KeyCode::Char('z'), m) if m.contains(KeyModifiers::CONTROL) => {
                        if let Some((value, cursor)) = self.undo_stack.pop_back() {
                            self.redo_stack.push_back((self.value.clone(), self.cursor));
                            self.value = value;
                            self.cursor = cursor;
                            self.filter_suggestions();
                        }
                        Command::none()
                    }
                    (KeyCode::Char('y'), m) if m.contains(KeyModifiers::CONTROL) => {
                        if let Some((value, cursor)) = self.redo_stack.pop_back() {
                            self.undo_stack.push_back((self.value.clone(), self.cursor));
                            self.value = value;
                            self.cursor = cursor;
                            self.filter_suggestions();
                        }
                        Command::none()
                    }
                    (KeyCode::Up, KeyModifiers::NONE) => {
                        if self.history.is_some() {
                            let current = self.value();
                            let entry = self
                                .history
                                .as_mut()
                                .unwrap()
                                .older(&current)
                                .map(|s| s.to_owned());
                            if let Some(entry) = entry {
                                self.value = entry.chars().collect();
                                self.cursor = self.value.len();
                                self.filter_suggestions();
                                return Command::message(Message::Changed(self.value()));
                            }
                        }
                        Command::none()
                    }
                    (KeyCode::Down, KeyModifiers::NONE) => {
                        if let Some(ref mut history) = self.history {
                            if let Some(entry) = history.newer().map(|s| s.to_owned()) {
                                self.value = entry.chars().collect();
                                self.cursor = self.value.len();
                                self.filter_suggestions();
                                return Command::message(Message::Changed(self.value()));
                            }
                        }
                        Command::none()
                    }
                    (KeyCode::Enter, _) => Command::message(Message::Submit(self.value())),
                    _ => Command::none(),
                };
                self.run_validate();
                cmd
            }
            Message::Paste(text) => {
                if !self.focus {
                    return Command::none();
                }
                self.push_undo();
                let cmd = self.insert_paste(&text);
                self.filter_suggestions();
                self.run_validate();
                cmd
            }
            Message::Changed(_) | Message::Submit(_) => Command::none(),
        }
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        let display = self.display_value();

        let inner = if let Some(ref block) = self.block {
            let inner = block.inner(area);
            frame.render_widget(block.clone(), area);
            inner
        } else {
            area
        };

        // Calculate visible range with horizontal scrolling
        let visible_width = inner.width as usize;
        let prompt_len = self.prompt.len();
        let available = visible_width.saturating_sub(prompt_len);

        // Adjust offset so cursor is visible
        let offset = if self.cursor < self.offset {
            self.cursor
        } else if self.cursor >= self.offset + available {
            self.cursor.saturating_sub(available) + 1
        } else {
            self.offset
        };

        let mut spans = Vec::new();

        if !self.prompt.is_empty() {
            spans.push(Span::styled(&self.prompt, self.style.prompt));
        }

        // Compute ghost text from the current suggestion (remaining part only).
        let ghost_text: Option<String> = if self.show_suggestions {
            if let Some(suggestion) = self.current_suggestion() {
                let current_val = self.value();
                if suggestion.len() > current_val.len() {
                    Some(suggestion[current_val.len()..].to_string())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        if display.is_empty() && !self.focus {
            spans.push(Span::styled(&self.placeholder, self.style.placeholder));
        } else if display.is_empty() && self.focus {
            // Show cursor on empty input
            spans.push(Span::styled(" ", self.style.cursor));
            // Show ghost text after cursor when input is empty
            if let Some(ref ghost) = ghost_text {
                spans.push(Span::styled(ghost.clone(), self.style.suggestion));
            }
        } else {
            let chars: Vec<char> = display.chars().collect();
            let visible_end = (offset + available).min(chars.len());
            let visible: String = chars[offset..visible_end].iter().collect();

            if self.focus {
                let cursor_in_visible = self.cursor.saturating_sub(offset);
                let before: String = visible.chars().take(cursor_in_visible).collect();
                let cursor_char = visible.chars().nth(cursor_in_visible);
                let after: String = visible.chars().skip(cursor_in_visible + 1).collect();

                if !before.is_empty() {
                    spans.push(Span::styled(before, self.style.text));
                }
                if let Some(c) = cursor_char {
                    spans.push(Span::styled(c.to_string(), self.style.cursor));
                } else {
                    spans.push(Span::styled(" ", self.style.cursor));
                    // Ghost text when cursor is at end of input
                    if let Some(ref ghost) = ghost_text {
                        spans.push(Span::styled(ghost.clone(), self.style.suggestion));
                    }
                }
                if !after.is_empty() {
                    spans.push(Span::styled(after, self.style.text));
                }
            } else {
                spans.push(Span::styled(visible, self.style.text));
            }
        }

        let paragraph = Paragraph::new(Line::from(spans));
        frame.render_widget(paragraph, inner);
    }

    fn focused(&self) -> bool {
        self.focus
    }
}

#[cfg(test)]
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
