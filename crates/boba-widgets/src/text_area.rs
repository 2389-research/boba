//! Multi-line text editor component with line numbers, text selection,
//! undo/redo, word case operations, clipboard integration, and soft wrapping.

use std::collections::VecDeque;

use boba_core::command::Command;
use boba_core::component::Component;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Wrap};
use ratatui::Frame;

/// Controls which key combination triggers a submit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SubmitBinding {
    /// Enter key submits.
    Enter,
    /// Shift+Enter submits.
    ShiftEnter,
    /// Ctrl+Enter submits.
    CtrlEnter,
    /// No submit key; all Enter variants insert newlines.
    #[default]
    None,
}

/// Controls how input text is displayed.
///
/// Only meaningful in single-line mode.
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

/// Messages for the text area component.
#[derive(Debug, Clone)]
pub enum Message {
    /// A keyboard event.
    KeyPress(KeyEvent),
    /// Emitted when content changes.
    Changed(String),
    /// Insert text at cursor position.
    Paste(String),
    /// Emitted with selected text on Ctrl+C.
    Copy(String),
    /// Emitted with selected text on Ctrl+X.
    Cut(String),
    /// Emitted when the user triggers the submit binding.
    Submit(String),
}

type UndoEntry = (Vec<Vec<char>>, (usize, usize));

/// A multi-line text editor with line numbers.
///
/// # Example
///
/// ```ignore
/// let mut editor = TextArea::new()
///     .with_content("Hello\nWorld")
///     .with_line_numbers(true)
///     .with_soft_wrap(true);
///
/// editor.focus();
///
/// // In your parent component's update method, forward messages:
/// // let cmd = editor.update(msg);
///
/// // In your parent component's view method, delegate rendering:
/// // editor.view(frame, area);
/// ```
pub struct TextArea {
    lines: Vec<Vec<char>>,
    cursor_row: usize,
    cursor_col: usize,
    scroll_offset: usize,
    focus: bool,
    show_line_numbers: bool,
    char_limit: Option<usize>,
    style: TextAreaStyle,
    selection_start: Option<(usize, usize)>,
    undo_stack: VecDeque<UndoEntry>,
    redo_stack: VecDeque<UndoEntry>,
    soft_wrap: bool,
    line_prompt: Option<String>,
    history: Option<boba_core::input_history::InputHistory>,
    block: Option<Block<'static>>,
    single_line: bool,
    submit_binding: SubmitBinding,
    /// Horizontal scroll offset for single-line mode.
    h_offset: usize,
    placeholder: String,
    echo_mode: EchoMode,
    suggestions: Vec<String>,
    filtered_suggestions: Vec<String>,
    show_suggestions: bool,
    suggestion_index: usize,
    validate: Option<Box<dyn Fn(&str) -> Result<(), String> + Send>>,
    err: Option<String>,
}

/// Style configuration for the text area.
#[derive(Debug, Clone)]
pub struct TextAreaStyle {
    /// Style applied to regular text content.
    pub text: Style,
    /// Style applied to the cursor character.
    pub cursor: Style,
    /// Style applied to line number gutters.
    pub line_number: Style,
    /// Style applied to selected (highlighted) text.
    pub selection: Style,
    /// Style applied to the per-line prompt string.
    pub prompt: Style,
    /// Style applied to placeholder text shown when empty and unfocused.
    pub placeholder: Style,
    /// Style applied to ghost text from autocomplete suggestions.
    pub suggestion: Style,
}

impl Default for TextAreaStyle {
    fn default() -> Self {
        Self {
            text: Style::default(),
            cursor: Style::default().add_modifier(Modifier::REVERSED),
            line_number: Style::default().fg(Color::DarkGray),
            selection: Style::default().bg(Color::DarkGray),
            prompt: Style::default().fg(Color::Cyan),
            placeholder: Style::default().fg(Color::DarkGray),
            suggestion: Style::default().fg(Color::DarkGray),
        }
    }
}

impl TextArea {
    /// Create an empty text area.
    pub fn new() -> Self {
        Self {
            lines: vec![Vec::new()],
            cursor_row: 0,
            cursor_col: 0,
            scroll_offset: 0,
            focus: false,
            show_line_numbers: true,
            char_limit: None,
            style: TextAreaStyle::default(),
            selection_start: None,
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
            soft_wrap: false,
            line_prompt: None,
            history: None,
            block: None,
            single_line: false,
            submit_binding: SubmitBinding::None,
            h_offset: 0,
            placeholder: String::new(),
            echo_mode: EchoMode::Normal,
            suggestions: Vec::new(),
            filtered_suggestions: Vec::new(),
            show_suggestions: true,
            suggestion_index: 0,
            validate: None,
            err: None,
        }
    }

    /// Initialize with the given text content.
    pub fn with_content(mut self, content: &str) -> Self {
        self.lines = content.lines().map(|l| l.chars().collect()).collect();
        if self.lines.is_empty() {
            self.lines.push(Vec::new());
        }
        // Preserve trailing newline: str::lines() drops it
        if content.ends_with('\n') {
            self.lines.push(Vec::new());
        }
        self
    }

    /// Show or hide line numbers (default: true).
    pub fn with_line_numbers(mut self, show: bool) -> Self {
        self.show_line_numbers = show;
        self
    }

    /// Set custom styles.
    pub fn with_style(mut self, style: TextAreaStyle) -> Self {
        self.style = style;
        self
    }

    /// Set maximum character count.
    pub fn with_char_limit(mut self, limit: usize) -> Self {
        self.char_limit = Some(limit);
        self
    }

    /// Enable soft wrapping at the visible width.
    pub fn with_soft_wrap(mut self, wrap: bool) -> Self {
        self.soft_wrap = wrap;
        self
    }

    /// Set a per-line prompt string rendered before each line.
    pub fn with_line_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.line_prompt = Some(prompt.into());
        self
    }

    /// Set the prompt string displayed before each line of content.
    ///
    /// This is an alias for `with_line_prompt` that provides a shorter name.
    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.line_prompt = Some(prompt.into());
        self
    }

    /// Set placeholder text displayed when the editor is empty and unfocused.
    pub fn with_placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = text.into();
        self
    }

    /// Enable input history with the given maximum number of entries.
    ///
    /// When enabled and the buffer is a single line, Up/Down keys browse
    /// through previously submitted inputs (shell-like behavior). When
    /// the buffer has multiple lines, Up/Down move the cursor normally.
    pub fn with_history(mut self, max_entries: usize) -> Self {
        self.history = Some(boba_core::input_history::InputHistory::new(max_entries));
        self
    }

    /// Set a block (border/title) to wrap the text area.
    pub fn with_block(mut self, block: Block<'static>) -> Self {
        self.block = Some(block);
        self
    }

    /// Enable single-line mode.
    ///
    /// In single-line mode, Enter submits (unless overridden), pasted text
    /// has newlines stripped, and Up/Down always browse history.
    ///
    /// When enabling single-line mode, the submit binding is automatically
    /// set to `Enter` if it was `None`. Use `with_submit()` to override.
    pub fn with_single_line(mut self, single_line: bool) -> Self {
        self.single_line = single_line;
        if single_line && self.submit_binding == SubmitBinding::None {
            self.submit_binding = SubmitBinding::Enter;
        }
        self
    }

    /// Set the key binding that triggers a submit.
    ///
    /// Works in both single-line and multi-line modes. When a submit
    /// binding is active, the non-submit Enter variants insert newlines.
    pub fn with_submit(mut self, binding: SubmitBinding) -> Self {
        self.submit_binding = binding;
        self
    }

    /// Set the echo mode for displaying input text.
    ///
    /// Only meaningful in single-line mode. In `Password` mode, each character
    /// is replaced with the given mask character. In `Hidden` mode, nothing is
    /// rendered.
    pub fn with_echo_mode(mut self, mode: EchoMode) -> Self {
        self.echo_mode = mode;
        self
    }

    /// Return the current echo mode.
    pub fn echo_mode(&self) -> &EchoMode {
        &self.echo_mode
    }

    /// Set the list of autocomplete suggestions.
    ///
    /// Suggestions are only active in single-line mode. After setting,
    /// the list is immediately filtered against the current input value.
    pub fn set_suggestions(&mut self, suggestions: Vec<String>) {
        self.suggestions = suggestions;
        self.filter_suggestions();
    }

    /// Builder method to set autocomplete suggestions.
    ///
    /// Suggestions are only active in single-line mode.
    pub fn with_suggestions(mut self, suggestions: Vec<String>) -> Self {
        self.suggestions = suggestions;
        self.filter_suggestions();
        self
    }

    /// Return the currently highlighted suggestion, if any.
    ///
    /// Returns `None` when not in single-line mode or when suggestions
    /// are hidden.
    pub fn current_suggestion(&self) -> Option<&str> {
        if !self.single_line || !self.show_suggestions {
            return None;
        }
        self.filtered_suggestions
            .get(self.suggestion_index)
            .map(|s| s.as_str())
    }

    /// Return the filtered suggestions that match the current input.
    pub fn available_suggestions(&self) -> &[String] {
        &self.filtered_suggestions
    }

    /// Show or hide autocomplete suggestions.
    pub fn show_suggestions(&mut self, show: bool) {
        self.show_suggestions = show;
    }

    /// Attach a validation function that runs after every content change.
    ///
    /// The validator receives the current value and should return `Ok(())`
    /// when valid or `Err(message)` with a human-readable error string.
    pub fn with_validate(
        mut self,
        f: impl Fn(&str) -> Result<(), String> + Send + 'static,
    ) -> Self {
        self.validate = Some(Box::new(f));
        self
    }

    /// Return the current validation error, if any.
    pub fn err(&self) -> Option<&str> {
        self.err.as_deref()
    }

    /// Filter the suggestion list against the current input value.
    ///
    /// Keeps only suggestions that start with the current value
    /// (case-insensitive) and are not an exact match. Resets the
    /// suggestion index to 0.
    fn filter_suggestions(&mut self) {
        let val = self.value().to_lowercase();
        self.filtered_suggestions = self
            .suggestions
            .iter()
            .filter(|s| {
                let sl = s.to_lowercase();
                sl.starts_with(&val) && sl != val
            })
            .cloned()
            .collect();
        self.suggestion_index = 0;
    }

    /// Run the validation function against the current value, updating `err`.
    fn run_validate(&mut self) {
        if let Some(ref f) = self.validate {
            self.err = f(&self.value()).err();
        }
    }

    /// Accept the current suggestion, replacing the input content.
    ///
    /// Returns true if a suggestion was accepted.
    fn accept_suggestion(&mut self) -> bool {
        if let Some(suggestion) = self.current_suggestion().map(|s| s.to_owned()) {
            self.lines = vec![suggestion.chars().collect()];
            self.cursor_col = self.lines[0].len();
            self.cursor_row = 0;
            self.filter_suggestions();
            true
        } else {
            false
        }
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

    /// Give this editor keyboard focus.
    pub fn focus(&mut self) {
        self.focus = true;
    }

    /// Remove keyboard focus.
    pub fn blur(&mut self) {
        self.focus = false;
    }

    /// Get the full content as a newline-separated string.
    pub fn value(&self) -> String {
        self.lines
            .iter()
            .map(|l| l.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Programmatically set content, resetting the cursor to 0,0.
    pub fn set_value(&mut self, content: &str) {
        self.lines = content.lines().map(|l| l.chars().collect()).collect();
        if self.lines.is_empty() {
            self.lines.push(Vec::new());
        }
        // Preserve trailing newline: str::lines() drops it
        if content.ends_with('\n') {
            self.lines.push(Vec::new());
        }
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.scroll_offset = 0;
        self.selection_start = None;
    }

    /// Insert a string at the current cursor position, handling newlines.
    pub fn insert_string(&mut self, s: &str) {
        for c in s.chars() {
            if let Some(limit) = self.char_limit {
                // Count newlines toward the limit to prevent unbounded line creation.
                if self.total_chars() + self.lines.len().saturating_sub(1) >= limit {
                    return;
                }
            }
            if c == '\n' {
                let rest = self.lines[self.cursor_row].split_off(self.cursor_col);
                self.cursor_row += 1;
                self.cursor_col = 0;
                self.lines.insert(self.cursor_row, rest);
            } else {
                self.lines[self.cursor_row].insert(self.cursor_col, c);
                self.cursor_col += 1;
            }
        }
    }

    /// Insert a single character at the current cursor position.
    pub fn insert_rune(&mut self, c: char) {
        if let Some(limit) = self.char_limit {
            if self.total_chars() >= limit {
                return;
            }
        }
        self.lines[self.cursor_row].insert(self.cursor_col, c);
        self.cursor_col += 1;
    }

    /// Return the total number of lines.
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Move the cursor down one line.
    pub fn cursor_down(&mut self) {
        if self.cursor_row < self.lines.len() - 1 {
            self.cursor_row += 1;
            self.clamp_cursor_col();
        }
    }

    /// Move the cursor up one line.
    pub fn cursor_up(&mut self) {
        if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.clamp_cursor_col();
        }
    }

    /// Return the current cursor row.
    pub fn cursor_row(&self) -> usize {
        self.cursor_row
    }

    /// Return the current cursor column.
    pub fn cursor_col(&self) -> usize {
        self.cursor_col
    }

    /// Return the current horizontal scroll offset (single-line mode).
    pub fn h_offset(&self) -> usize {
        self.h_offset
    }

    /// Update the horizontal offset so the cursor stays visible within
    /// the given available width. Called internally during `view()` and
    /// exposed for testing.
    pub fn update_h_offset(&mut self, available_width: usize) {
        if available_width == 0 {
            return;
        }
        if self.cursor_col < self.h_offset {
            self.h_offset = self.cursor_col;
        } else if self.cursor_col >= self.h_offset + available_width {
            self.h_offset = self.cursor_col.saturating_sub(available_width) + 1;
        }
    }

    /// Return whether there is an active selection.
    pub fn has_selection(&self) -> bool {
        if let Some((sr, sc)) = self.selection_start {
            sr != self.cursor_row || sc != self.cursor_col
        } else {
            false
        }
    }

    /// Return the normalized selection range so start <= end.
    /// Compares row first, then col.
    pub fn selection_range(&self) -> Option<((usize, usize), (usize, usize))> {
        let (sr, sc) = self.selection_start?;
        let (er, ec) = (self.cursor_row, self.cursor_col);
        if sr == er && sc == ec {
            return None;
        }
        if (sr, sc) <= (er, ec) {
            Some(((sr, sc), (er, ec)))
        } else {
            Some(((er, ec), (sr, sc)))
        }
    }

    /// Return the selected text, or None if no selection.
    pub fn selected_text(&self) -> Option<String> {
        let ((sr, sc), (er, ec)) = self.selection_range()?;
        if sr == er {
            // Single-line selection
            let text: String = self.lines[sr][sc..ec].iter().collect();
            Some(text)
        } else {
            let mut result = String::new();
            // First line: from sc to end
            let first: String = self.lines[sr][sc..].iter().collect();
            result.push_str(&first);
            // Middle lines
            for row in (sr + 1)..er {
                result.push('\n');
                let line: String = self.lines[row].iter().collect();
                result.push_str(&line);
            }
            // Last line: from start to ec
            result.push('\n');
            let last: String = self.lines[er][..ec].iter().collect();
            result.push_str(&last);
            Some(result)
        }
    }

    /// Select all text in the text area.
    pub fn select_all(&mut self) {
        self.selection_start = Some((0, 0));
        let last_row = self.lines.len() - 1;
        let last_col = self.lines[last_row].len();
        self.cursor_row = last_row;
        self.cursor_col = last_col;
    }

    /// Delete selected text. Returns true if there was a selection to delete.
    pub fn delete_selection(&mut self) -> bool {
        let range = match self.selection_range() {
            Some(r) => r,
            None => {
                self.selection_start = None;
                return false;
            }
        };
        let ((sr, sc), (er, ec)) = range;
        if sr == er {
            // Single-line deletion
            self.lines[sr].drain(sc..ec);
        } else {
            // Keep the part before selection on the first line
            // and the part after selection on the last line, join them.
            let after: Vec<char> = self.lines[er][ec..].to_vec();
            // Remove lines from sr+1 through er
            self.lines.drain((sr + 1)..=er);
            // Truncate the first line at sc and append the tail
            self.lines[sr].truncate(sc);
            self.lines[sr].extend(after);
        }
        self.cursor_row = sr;
        self.cursor_col = sc;
        self.selection_start = None;
        true
    }

    /// Begin or continue a selection. If no selection is active, record the
    /// current cursor position as the selection start.
    fn ensure_selection_started(&mut self) {
        if self.selection_start.is_none() {
            self.selection_start = Some((self.cursor_row, self.cursor_col));
        }
    }

    /// Clear the selection without deleting text.
    fn clear_selection(&mut self) {
        self.selection_start = None;
    }

    fn current_line_len(&self) -> usize {
        self.lines[self.cursor_row].len()
    }

    fn clamp_cursor_col(&mut self) {
        let len = self.current_line_len();
        if self.cursor_col > len {
            self.cursor_col = len;
        }
    }

    /// Return the total character count across all lines (not counting newlines).
    fn total_chars(&self) -> usize {
        self.lines.iter().map(|l| l.len()).sum()
    }

    /// Find the previous word boundary from the current cursor position on the
    /// current line. A word boundary is a transition between whitespace and
    /// non-whitespace characters, scanning backwards.
    fn prev_word_boundary(&self) -> usize {
        let line = &self.lines[self.cursor_row];
        if self.cursor_col == 0 {
            return 0;
        }
        let mut col = self.cursor_col;
        // Skip whitespace backwards
        while col > 0 && line[col - 1].is_whitespace() {
            col -= 1;
        }
        // Skip non-whitespace backwards
        while col > 0 && !line[col - 1].is_whitespace() {
            col -= 1;
        }
        col
    }

    /// Find the next word boundary from the current cursor position on the
    /// current line.
    fn next_word_boundary(&self) -> usize {
        let line = &self.lines[self.cursor_row];
        let len = line.len();
        if self.cursor_col >= len {
            return len;
        }
        let mut col = self.cursor_col;
        // Skip non-whitespace forward
        while col < len && !line[col].is_whitespace() {
            col += 1;
        }
        // Skip whitespace forward
        while col < len && line[col].is_whitespace() {
            col += 1;
        }
        col
    }

    /// Find the end of the current word from the cursor position.
    /// A word ends at the next whitespace or end of line.
    fn word_end_boundary(&self) -> usize {
        let line = &self.lines[self.cursor_row];
        let len = line.len();
        if self.cursor_col >= len {
            return len;
        }
        let mut col = self.cursor_col;
        while col < len && !line[col].is_whitespace() {
            col += 1;
        }
        col
    }

    /// Kill from cursor to end of line. If cursor is at end of line, join with
    /// the next line.
    fn kill_to_end_of_line(&mut self) {
        if self.cursor_col < self.current_line_len() {
            self.lines[self.cursor_row].truncate(self.cursor_col);
        } else if self.cursor_row < self.lines.len() - 1 {
            let next = self.lines.remove(self.cursor_row + 1);
            self.lines[self.cursor_row].extend(next);
        }
    }

    /// Kill from start of line to cursor.
    fn kill_to_start_of_line(&mut self) {
        let remaining = self.lines[self.cursor_row].split_off(self.cursor_col);
        self.lines[self.cursor_row] = remaining;
        self.cursor_col = 0;
    }

    /// Delete the word before the cursor.
    fn delete_word_backward(&mut self) {
        let boundary = self.prev_word_boundary();
        if boundary < self.cursor_col {
            self.lines[self.cursor_row].drain(boundary..self.cursor_col);
            self.cursor_col = boundary;
        } else if self.cursor_col == 0 && self.cursor_row > 0 {
            // At the start of a line, join with the previous line
            let current = self.lines.remove(self.cursor_row);
            self.cursor_row -= 1;
            self.cursor_col = self.lines[self.cursor_row].len();
            self.lines[self.cursor_row].extend(current);
        }
    }

    /// Delete the word after the cursor.
    fn delete_word_forward(&mut self) {
        let boundary = self.next_word_boundary();
        if boundary > self.cursor_col {
            self.lines[self.cursor_row].drain(self.cursor_col..boundary);
        } else if self.cursor_col >= self.current_line_len()
            && self.cursor_row < self.lines.len() - 1
        {
            // At the end of a line, join with the next line
            let next = self.lines.remove(self.cursor_row + 1);
            self.lines[self.cursor_row].extend(next);
        }
    }

    /// Push the current state onto the undo stack, clearing the redo stack.
    /// Caps the undo stack at 100 entries.
    fn push_undo(&mut self) {
        self.undo_stack
            .push_back((self.lines.clone(), (self.cursor_row, self.cursor_col)));
        self.redo_stack.clear();
        if self.undo_stack.len() > 100 {
            self.undo_stack.pop_front();
        }
    }

    /// Uppercase the word at cursor (from cursor_col to next word boundary).
    fn uppercase_word(&mut self) {
        let end = self.word_end_boundary();
        if end <= self.cursor_col {
            return;
        }
        for i in self.cursor_col..end {
            let upper: Vec<char> = self.lines[self.cursor_row][i].to_uppercase().collect();
            if upper.len() == 1 {
                self.lines[self.cursor_row][i] = upper[0];
            }
        }
        self.cursor_col = end;
    }

    /// Lowercase the word at cursor (from cursor_col to next word boundary).
    fn lowercase_word(&mut self) {
        let end = self.word_end_boundary();
        if end <= self.cursor_col {
            return;
        }
        for i in self.cursor_col..end {
            let lower: Vec<char> = self.lines[self.cursor_row][i].to_lowercase().collect();
            if lower.len() == 1 {
                self.lines[self.cursor_row][i] = lower[0];
            }
        }
        self.cursor_col = end;
    }

    /// Capitalize the word at cursor: uppercase the first char, lowercase the rest.
    fn capitalize_word(&mut self) {
        let end = self.word_end_boundary();
        if end <= self.cursor_col {
            return;
        }
        // Uppercase the first character
        let upper: Vec<char> = self.lines[self.cursor_row][self.cursor_col]
            .to_uppercase()
            .collect();
        if upper.len() == 1 {
            self.lines[self.cursor_row][self.cursor_col] = upper[0];
        }
        // Lowercase the rest
        for i in (self.cursor_col + 1)..end {
            let lower: Vec<char> = self.lines[self.cursor_row][i].to_lowercase().collect();
            if lower.len() == 1 {
                self.lines[self.cursor_row][i] = lower[0];
            }
        }
        self.cursor_col = end;
    }

    /// Check whether a position (row, col) falls within the current selection.
    /// Returns true if the position is >= start and < end of the normalized
    /// selection range.
    fn is_selected(&self, row: usize, col: usize) -> bool {
        if let Some(((sr, sc), (er, ec))) = self.selection_range() {
            if row < sr || row > er {
                return false;
            }
            if row == sr && row == er {
                return col >= sc && col < ec;
            }
            if row == sr {
                return col >= sc;
            }
            if row == er {
                return col < ec;
            }
            // Middle row — fully selected
            true
        } else {
            false
        }
    }

    /// Render the text area in single-line mode with horizontal scrolling
    /// and overflow indicators.
    fn view_single_line(&self, frame: &mut Frame, inner: Rect) {
        let prompt_width = self.line_prompt.as_ref().map(|p| p.len()).unwrap_or(0);
        let total_width = inner.width as usize;
        let available = total_width.saturating_sub(prompt_width);

        if available == 0 {
            return;
        }

        // Hidden echo mode: render prompt only (no content, no cursor).
        if matches!(self.echo_mode, EchoMode::Hidden) {
            let mut spans = Vec::new();
            if let Some(ref prompt) = self.line_prompt {
                spans.push(Span::styled(prompt.clone(), self.style.prompt));
            }
            let paragraph = Paragraph::new(Line::from(spans));
            frame.render_widget(paragraph, inner);
            return;
        }

        // Compute horizontal offset so cursor stays visible (same logic
        // as update_h_offset but without mutating self).
        let h_off = if self.cursor_col < self.h_offset {
            self.cursor_col
        } else if self.cursor_col >= self.h_offset + available {
            self.cursor_col.saturating_sub(available) + 1
        } else {
            self.h_offset
        };

        // For Password echo mode, replace each character with the mask.
        let masked;
        let line_chars = match self.echo_mode {
            EchoMode::Password(mask) => {
                masked = vec![mask; self.lines[0].len()];
                &masked
            }
            _ => &self.lines[0],
        };
        let line_len = line_chars.len();

        // Determine if overflow indicators are needed
        let has_left_overflow = h_off > 0;
        let has_right_overflow = h_off + available < line_len;

        // Slice the visible window from line content
        let visible_end = (h_off + available).min(line_len);
        let mut visible: Vec<char> = line_chars[h_off..visible_end].to_vec();

        // Pad with spaces if content is shorter than available width
        while visible.len() < available {
            visible.push(' ');
        }

        // Replace edge characters with overflow indicators
        if has_left_overflow && !visible.is_empty() {
            visible[0] = '\u{2026}'; // …
        }
        if has_right_overflow && !visible.is_empty() {
            let last = visible.len() - 1;
            visible[last] = '\u{2026}'; // …
        }

        let mut spans = Vec::new();

        if let Some(ref prompt) = self.line_prompt {
            spans.push(Span::styled(prompt.clone(), self.style.prompt));
        }

        // Show placeholder text when empty and unfocused
        let is_empty = line_len == 0;
        if is_empty && !self.focus && !self.placeholder.is_empty() {
            spans.push(Span::styled(self.placeholder.clone(), self.style.placeholder));
            let paragraph = Paragraph::new(Line::from(spans));
            frame.render_widget(paragraph, inner);
            return;
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

        // Render the visible slice with cursor highlighting
        if self.focus {
            let cursor_in_visible = self.cursor_col.saturating_sub(h_off);
            let before: String = visible[..cursor_in_visible].iter().collect();
            let cursor_char = visible.get(cursor_in_visible);
            let after_start = cursor_in_visible + 1;
            let after: String = if after_start < visible.len() {
                visible[after_start..].iter().collect()
            } else {
                String::new()
            };

            if !before.is_empty() {
                spans.push(Span::styled(before, self.style.text));
            }
            if let Some(&c) = cursor_char {
                // If the character is a padding space at the end of content,
                // render it as a cursor block.
                spans.push(Span::styled(c.to_string(), self.style.cursor));
            } else {
                spans.push(Span::styled(" ", self.style.cursor));
            }
            if !after.is_empty() {
                spans.push(Span::styled(after, self.style.text));
            }
            // Show ghost text after the cursor when cursor is at end of content.
            if let Some(ref ghost) = ghost_text {
                if self.cursor_col >= line_len {
                    spans.push(Span::styled(ghost.clone(), self.style.suggestion));
                }
            }
        } else {
            let text: String = visible.iter().collect();
            spans.push(Span::styled(text, self.style.text));
        }

        let paragraph = Paragraph::new(Line::from(spans));
        frame.render_widget(paragraph, inner);
    }
}

impl Default for TextArea {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for TextArea {
    type Message = Message;

    fn update(&mut self, msg: Message) -> Command<Message> {
        match msg {
            Message::Paste(text) if self.focus => {
                // In single-line mode, strip newlines from pasted text.
                let text = if self.single_line {
                    text.replace('\n', "").replace('\r', "")
                } else {
                    text
                };
                self.push_undo();
                self.delete_selection();
                if let Some(limit) = self.char_limit {
                    let available = limit.saturating_sub(self.total_chars());
                    let chars: Vec<char> = text.chars().collect();
                    let to_insert: String = chars.into_iter().take(available).collect();
                    self.insert_string(&to_insert);
                } else {
                    self.insert_string(&text);
                }
                if self.single_line {
                    self.filter_suggestions();
                }
                self.run_validate();
                Command::message(Message::Changed(self.value()))
            }
            Message::KeyPress(key) if self.focus => {
                let shift = key.modifiers.contains(KeyModifiers::SHIFT);
                match (key.code, key.modifiers) {
                    // Ctrl+Z: undo
                    (KeyCode::Char('z'), KeyModifiers::CONTROL) => {
                        if let Some((lines, (row, col))) = self.undo_stack.pop_back() {
                            self.redo_stack.push_back((
                                self.lines.clone(),
                                (self.cursor_row, self.cursor_col),
                            ));
                            self.lines = lines;
                            self.cursor_row = row;
                            self.cursor_col = col;
                            self.selection_start = None;
                            if self.single_line {
                                self.filter_suggestions();
                            }
                            self.run_validate();
                            Command::message(Message::Changed(self.value()))
                        } else {
                            Command::none()
                        }
                    }
                    // Ctrl+Y: redo
                    (KeyCode::Char('y'), KeyModifiers::CONTROL) => {
                        if let Some((lines, (row, col))) = self.redo_stack.pop_back() {
                            self.undo_stack.push_back((
                                self.lines.clone(),
                                (self.cursor_row, self.cursor_col),
                            ));
                            self.lines = lines;
                            self.cursor_row = row;
                            self.cursor_col = col;
                            self.selection_start = None;
                            if self.single_line {
                                self.filter_suggestions();
                            }
                            self.run_validate();
                            Command::message(Message::Changed(self.value()))
                        } else {
                            Command::none()
                        }
                    }
                    // Ctrl+C: copy selection
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                        if let Some(text) = self.selected_text() {
                            Command::message(Message::Copy(text))
                        } else {
                            Command::none()
                        }
                    }
                    // Ctrl+X: cut selection
                    (KeyCode::Char('x'), KeyModifiers::CONTROL) => {
                        if let Some(text) = self.selected_text() {
                            self.push_undo();
                            self.delete_selection();
                            if self.single_line {
                                self.filter_suggestions();
                            }
                            self.run_validate();
                            Command::batch([
                                Command::message(Message::Cut(text)),
                                Command::message(Message::Changed(self.value())),
                            ])
                        } else {
                            Command::none()
                        }
                    }
                    // Ctrl+A: select all
                    (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
                        self.select_all();
                        Command::none()
                    }
                    // Ctrl+K: kill to end of line
                    (KeyCode::Char('k'), KeyModifiers::CONTROL) => {
                        self.push_undo();
                        self.clear_selection();
                        self.kill_to_end_of_line();
                        if self.single_line {
                            self.filter_suggestions();
                        }
                        self.run_validate();
                        Command::message(Message::Changed(self.value()))
                    }
                    // Ctrl+U: kill to start of line
                    (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                        self.push_undo();
                        self.clear_selection();
                        self.kill_to_start_of_line();
                        if self.single_line {
                            self.filter_suggestions();
                        }
                        self.run_validate();
                        Command::message(Message::Changed(self.value()))
                    }
                    // Ctrl+W / Alt+Backspace: delete word backward
                    (KeyCode::Char('w'), KeyModifiers::CONTROL) => {
                        self.push_undo();
                        self.clear_selection();
                        self.delete_word_backward();
                        if self.single_line {
                            self.filter_suggestions();
                        }
                        self.run_validate();
                        Command::message(Message::Changed(self.value()))
                    }
                    (KeyCode::Backspace, KeyModifiers::ALT) => {
                        self.push_undo();
                        self.clear_selection();
                        self.delete_word_backward();
                        if self.single_line {
                            self.filter_suggestions();
                        }
                        self.run_validate();
                        Command::message(Message::Changed(self.value()))
                    }
                    // Alt+D: delete word forward
                    (KeyCode::Char('d'), KeyModifiers::ALT) => {
                        self.push_undo();
                        self.clear_selection();
                        self.delete_word_forward();
                        if self.single_line {
                            self.filter_suggestions();
                        }
                        self.run_validate();
                        Command::message(Message::Changed(self.value()))
                    }
                    // Alt+U: uppercase word at cursor
                    (KeyCode::Char('u'), KeyModifiers::ALT) => {
                        self.push_undo();
                        self.clear_selection();
                        self.uppercase_word();
                        if self.single_line {
                            self.filter_suggestions();
                        }
                        self.run_validate();
                        Command::message(Message::Changed(self.value()))
                    }
                    // Alt+L: lowercase word at cursor
                    (KeyCode::Char('l'), KeyModifiers::ALT) => {
                        self.push_undo();
                        self.clear_selection();
                        self.lowercase_word();
                        if self.single_line {
                            self.filter_suggestions();
                        }
                        self.run_validate();
                        Command::message(Message::Changed(self.value()))
                    }
                    // Alt+C: capitalize word at cursor
                    (KeyCode::Char('c'), KeyModifiers::ALT) => {
                        self.push_undo();
                        self.clear_selection();
                        self.capitalize_word();
                        if self.single_line {
                            self.filter_suggestions();
                        }
                        self.run_validate();
                        Command::message(Message::Changed(self.value()))
                    }
                    // Ctrl+Delete: delete word forward
                    (KeyCode::Delete, KeyModifiers::CONTROL) => {
                        self.push_undo();
                        self.clear_selection();
                        self.delete_word_forward();
                        if self.single_line {
                            self.filter_suggestions();
                        }
                        self.run_validate();
                        Command::message(Message::Changed(self.value()))
                    }
                    // Ctrl+Left / Alt+Left: move to previous word boundary
                    (KeyCode::Left, KeyModifiers::CONTROL) | (KeyCode::Left, KeyModifiers::ALT) => {
                        self.clear_selection();
                        self.cursor_col = self.prev_word_boundary();
                        Command::none()
                    }
                    // Ctrl+Right / Alt+Right: move to next word boundary
                    (KeyCode::Right, KeyModifiers::CONTROL)
                    | (KeyCode::Right, KeyModifiers::ALT) => {
                        self.clear_selection();
                        self.cursor_col = self.next_word_boundary();
                        Command::none()
                    }
                    // Shift+Left: extend selection left
                    (KeyCode::Left, _) if shift => {
                        self.ensure_selection_started();
                        if self.cursor_col > 0 {
                            self.cursor_col -= 1;
                        } else if self.cursor_row > 0 {
                            self.cursor_row -= 1;
                            self.cursor_col = self.current_line_len();
                        }
                        Command::none()
                    }
                    // Shift+Right: extend selection right
                    (KeyCode::Right, _) if shift => {
                        self.ensure_selection_started();
                        if self.cursor_col < self.current_line_len() {
                            self.cursor_col += 1;
                        } else if self.cursor_row < self.lines.len() - 1 {
                            self.cursor_row += 1;
                            self.cursor_col = 0;
                        }
                        Command::none()
                    }
                    // Shift+Up: extend selection up
                    (KeyCode::Up, _) if shift => {
                        self.ensure_selection_started();
                        if self.cursor_row > 0 {
                            self.cursor_row -= 1;
                            self.clamp_cursor_col();
                        }
                        Command::none()
                    }
                    // Shift+Down: extend selection down
                    (KeyCode::Down, _) if shift => {
                        self.ensure_selection_started();
                        if self.cursor_row < self.lines.len() - 1 {
                            self.cursor_row += 1;
                            self.clamp_cursor_col();
                        }
                        Command::none()
                    }
                    // Shift+Home: select to start of line
                    (KeyCode::Home, _) if shift => {
                        self.ensure_selection_started();
                        self.cursor_col = 0;
                        Command::none()
                    }
                    // Shift+End: select to end of line
                    (KeyCode::End, _) if shift => {
                        self.ensure_selection_started();
                        self.cursor_col = self.current_line_len();
                        Command::none()
                    }
                    // Tab: accept suggestion in single-line mode, otherwise insert 4 spaces
                    (KeyCode::Tab, _) => {
                        if self.single_line {
                            if self.accept_suggestion() {
                                self.run_validate();
                                Command::message(Message::Changed(self.value()))
                            } else {
                                Command::none()
                            }
                        } else {
                            self.push_undo();
                            self.delete_selection();
                            for _ in 0..4 {
                                if let Some(limit) = self.char_limit {
                                    if self.total_chars() >= limit {
                                        break;
                                    }
                                }
                                self.lines[self.cursor_row].insert(self.cursor_col, ' ');
                                self.cursor_col += 1;
                            }
                            self.run_validate();
                            Command::message(Message::Changed(self.value()))
                        }
                    }
                    (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                        self.push_undo();
                        self.delete_selection();
                        if let Some(limit) = self.char_limit {
                            if self.total_chars() >= limit {
                                return Command::none();
                            }
                        }
                        self.lines[self.cursor_row].insert(self.cursor_col, c);
                        self.cursor_col += 1;
                        if self.single_line {
                            self.filter_suggestions();
                        }
                        self.run_validate();
                        Command::message(Message::Changed(self.value()))
                    }
                    (KeyCode::Enter, m) => {
                        let is_submit = match self.submit_binding {
                            SubmitBinding::Enter => {
                                !m.contains(KeyModifiers::SHIFT)
                                    && !m.contains(KeyModifiers::CONTROL)
                            }
                            SubmitBinding::ShiftEnter => m.contains(KeyModifiers::SHIFT),
                            SubmitBinding::CtrlEnter => m.contains(KeyModifiers::CONTROL),
                            SubmitBinding::None => false,
                        };
                        if is_submit {
                            return Command::message(Message::Submit(self.value()));
                        }
                        if self.single_line {
                            return Command::none();
                        }
                        self.push_undo();
                        self.delete_selection();
                        let rest = self.lines[self.cursor_row].split_off(self.cursor_col);
                        self.cursor_row += 1;
                        self.cursor_col = 0;
                        self.lines.insert(self.cursor_row, rest);
                        self.run_validate();
                        Command::message(Message::Changed(self.value()))
                    }
                    (KeyCode::Backspace, _) => {
                        if self.has_selection() {
                            self.push_undo();
                            self.delete_selection();
                        } else {
                            self.clear_selection();
                            if self.cursor_col > 0 {
                                self.push_undo();
                                self.cursor_col -= 1;
                                self.lines[self.cursor_row].remove(self.cursor_col);
                            } else if self.cursor_row > 0 {
                                self.push_undo();
                                let current = self.lines.remove(self.cursor_row);
                                self.cursor_row -= 1;
                                self.cursor_col = self.lines[self.cursor_row].len();
                                self.lines[self.cursor_row].extend(current);
                            } else {
                                return Command::none();
                            }
                        }
                        if self.single_line {
                            self.filter_suggestions();
                        }
                        self.run_validate();
                        Command::message(Message::Changed(self.value()))
                    }
                    (KeyCode::Delete, _) => {
                        if self.has_selection() {
                            self.push_undo();
                            self.delete_selection();
                        } else {
                            self.clear_selection();
                            if self.cursor_col < self.current_line_len() {
                                self.push_undo();
                                self.lines[self.cursor_row].remove(self.cursor_col);
                            } else if self.cursor_row < self.lines.len() - 1 {
                                self.push_undo();
                                let next = self.lines.remove(self.cursor_row + 1);
                                self.lines[self.cursor_row].extend(next);
                            } else {
                                return Command::none();
                            }
                        }
                        if self.single_line {
                            self.filter_suggestions();
                        }
                        self.run_validate();
                        Command::message(Message::Changed(self.value()))
                    }
                    (KeyCode::Left, _) => {
                        self.clear_selection();
                        if self.cursor_col > 0 {
                            self.cursor_col -= 1;
                        } else if self.cursor_row > 0 {
                            self.cursor_row -= 1;
                            self.cursor_col = self.current_line_len();
                        }
                        Command::none()
                    }
                    (KeyCode::Right, _) => {
                        self.clear_selection();
                        if self.cursor_col < self.current_line_len() {
                            self.cursor_col += 1;
                        } else if self.single_line
                            && self.cursor_col >= self.current_line_len()
                            && self.accept_suggestion()
                        {
                            self.run_validate();
                            return Command::message(Message::Changed(self.value()));
                        } else if self.cursor_row < self.lines.len() - 1 {
                            self.cursor_row += 1;
                            self.cursor_col = 0;
                        }
                        Command::none()
                    }
                    (KeyCode::Up, _) if !shift => {
                        // History browsing: in single-line mode always try
                        // history; in multi-line mode only when the buffer
                        // happens to be a single line.
                        if self.single_line || (self.lines.len() == 1 && self.cursor_row == 0) {
                            let current = self.value();
                            let entry = self
                                .history
                                .as_mut()
                                .and_then(|h| h.older(&current))
                                .map(|s| s.to_owned());
                            if let Some(entry) = entry {
                                self.lines = vec![entry.chars().collect()];
                                self.cursor_row = 0;
                                self.cursor_col = self.lines[0].len();
                                self.selection_start = None;
                                self.run_validate();
                                return Command::message(Message::Changed(self.value()));
                            }
                        }
                        self.clear_selection();
                        if self.cursor_row > 0 {
                            self.cursor_row -= 1;
                            self.clamp_cursor_col();
                        }
                        Command::none()
                    }
                    (KeyCode::Down, _) if !shift => {
                        // History browsing: in single-line mode always try
                        // history; in multi-line mode only when the buffer
                        // happens to be a single line.
                        if self.single_line || (self.lines.len() == 1 && self.cursor_row == 0) {
                            if let Some(ref mut history) = self.history {
                                if let Some(entry) = history.newer().map(|s| s.to_owned()) {
                                    self.lines = vec![entry.chars().collect()];
                                    self.cursor_row = 0;
                                    self.cursor_col = self.lines[0].len();
                                    self.selection_start = None;
                                    self.run_validate();
                                    return Command::message(Message::Changed(self.value()));
                                }
                            }
                        }
                        self.clear_selection();
                        if self.cursor_row < self.lines.len() - 1 {
                            self.cursor_row += 1;
                            self.clamp_cursor_col();
                        }
                        Command::none()
                    }
                    (KeyCode::Home, _) => {
                        self.clear_selection();
                        self.cursor_col = 0;
                        Command::none()
                    }
                    (KeyCode::End, _) => {
                        self.clear_selection();
                        self.cursor_col = self.current_line_len();
                        Command::none()
                    }
                    _ => Command::none(),
                }
            }
            Message::Changed(_) => Command::none(),
            _ => Command::none(),
        }
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        let inner = if let Some(ref block) = self.block {
            let inner = block.inner(area);
            frame.render_widget(block.clone(), area);
            inner
        } else {
            area
        };

        // Single-line mode uses a separate rendering path with horizontal
        // scrolling and no line numbers or vertical scroll.
        if self.single_line {
            self.view_single_line(frame, inner);
            return;
        }

        // Show placeholder text when empty and unfocused
        let content_empty = self.lines.len() == 1 && self.lines[0].is_empty();
        if content_empty && !self.focus && !self.placeholder.is_empty() {
            let placeholder_line = Line::from(Span::styled(
                self.placeholder.clone(),
                self.style.placeholder,
            ));
            let paragraph = Paragraph::new(placeholder_line);
            frame.render_widget(paragraph, inner);
            return;
        }

        let visible_height = inner.height as usize;

        // Adjust scroll to keep cursor visible
        let scroll = if self.cursor_row < self.scroll_offset {
            self.cursor_row
        } else if self.cursor_row >= self.scroll_offset + visible_height {
            self.cursor_row.saturating_sub(visible_height) + 1
        } else {
            self.scroll_offset
        };
        // Clamp scroll to valid range
        let scroll = scroll.min(self.lines.len().saturating_sub(1));

        let line_num_width = if self.show_line_numbers {
            format!("{}", self.lines.len()).len() + 1
        } else {
            0
        };

        let prompt_width = self.line_prompt.as_ref().map(|p| p.len()).unwrap_or(0);

        let has_sel = self.has_selection();

        let end = self.lines.len().min(scroll + visible_height);
        let visible_lines = &self.lines[scroll..end];
        let display_lines: Vec<Line> = visible_lines
            .iter()
            .enumerate()
            .map(|(vi, line_chars)| {
                let actual_row = scroll + vi;
                let mut spans = Vec::new();

                if self.show_line_numbers {
                    let num = format!("{:>width$} ", actual_row + 1, width = line_num_width - 1);
                    spans.push(Span::styled(num, self.style.line_number));
                }

                if let Some(ref prompt) = self.line_prompt {
                    spans.push(Span::styled(prompt.clone(), self.style.prompt));
                }

                if has_sel {
                    // Render with selection highlighting
                    let line_len = line_chars.len();
                    let is_cursor_line = self.focus && actual_row == self.cursor_row;

                    // Build spans character by character, grouping consecutive
                    // characters with the same style.
                    let mut i = 0;
                    while i <= line_len {
                        if i == line_len {
                            // At end of line: render cursor if this is the
                            // cursor line and cursor is past all chars.
                            if is_cursor_line && self.cursor_col == line_len {
                                if self.is_selected(actual_row, i) {
                                    // Cursor on selected trailing position
                                    spans.push(Span::styled(" ", self.style.cursor));
                                } else {
                                    spans.push(Span::styled(" ", self.style.cursor));
                                }
                            }
                            break;
                        }

                        let sel = self.is_selected(actual_row, i);
                        let is_cursor = is_cursor_line && i == self.cursor_col;

                        if is_cursor {
                            // Render cursor character
                            spans.push(Span::styled(line_chars[i].to_string(), self.style.cursor));
                            i += 1;
                        } else {
                            // Collect a run of characters that share the same
                            // selected state and are not the cursor.
                            let style = if sel {
                                self.style.selection
                            } else {
                                self.style.text
                            };
                            let start = i;
                            while i < line_len
                                && self.is_selected(actual_row, i) == sel
                                && !(is_cursor_line && i == self.cursor_col)
                            {
                                i += 1;
                            }
                            let chunk: String = line_chars[start..i].iter().collect();
                            if !chunk.is_empty() {
                                spans.push(Span::styled(chunk, style));
                            }
                        }
                    }
                } else if self.focus && actual_row == self.cursor_row {
                    let line_str: String = line_chars.iter().collect();
                    let col = self.cursor_col;
                    let before: String = line_str.chars().take(col).collect();
                    let cursor_char = line_str.chars().nth(col);
                    let after: String = line_str.chars().skip(col + 1).collect();

                    if !before.is_empty() {
                        spans.push(Span::styled(before, self.style.text));
                    }
                    if let Some(c) = cursor_char {
                        spans.push(Span::styled(c.to_string(), self.style.cursor));
                    } else {
                        spans.push(Span::styled(" ", self.style.cursor));
                    }
                    if !after.is_empty() {
                        spans.push(Span::styled(after, self.style.text));
                    }
                } else {
                    let line_str: String = line_chars.iter().collect();
                    spans.push(Span::styled(line_str, self.style.text));
                }

                Line::from(spans)
            })
            .collect();

        // Account for prompt and line number widths when placing the cursor.
        // The cursor_col in the internal model maps to a display column offset
        // by line_num_width + prompt_width.
        let _cursor_display_col = line_num_width + prompt_width + self.cursor_col;

        let paragraph = if self.soft_wrap {
            Paragraph::new(display_lines).wrap(Wrap { trim: false })
        } else {
            Paragraph::new(display_lines)
        };
        frame.render_widget(paragraph, inner);
    }

    fn focused(&self) -> bool {
        self.focus
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Helper to send a key event to a TextArea and return the command.
    fn send_key(ta: &mut TextArea, code: KeyCode, modifiers: KeyModifiers) -> Command<Message> {
        let key = KeyEvent::new(code, modifiers);
        ta.update(Message::KeyPress(key))
    }

    /// Extract the message from a Command if it is an Action::Message.
    fn extract_message(cmd: Command<Message>) -> Option<Message> {
        cmd.into_message()
    }

    #[test]
    fn test_new_textarea_is_empty() {
        let ta = TextArea::new();
        assert_eq!(ta.value(), "");
        assert_eq!(ta.line_count(), 1);
        assert_eq!(ta.cursor_row(), 0);
        assert_eq!(ta.cursor_col(), 0);
    }

    #[test]
    fn test_with_content() {
        let ta = TextArea::new().with_content("hello\nworld");
        assert_eq!(ta.value(), "hello\nworld");
        assert_eq!(ta.line_count(), 2);
    }

    #[test]
    fn test_insert_char() {
        let mut ta = TextArea::new();
        ta.focus();
        send_key(&mut ta, KeyCode::Char('a'), KeyModifiers::NONE);
        assert_eq!(ta.value(), "a");
        assert_eq!(ta.cursor_col(), 1);
    }

    #[test]
    fn test_undo_redo_basic() {
        let mut ta = TextArea::new();
        ta.focus();
        // Type "ab"
        send_key(&mut ta, KeyCode::Char('a'), KeyModifiers::NONE);
        send_key(&mut ta, KeyCode::Char('b'), KeyModifiers::NONE);
        assert_eq!(ta.value(), "ab");

        // Undo once -> should restore state before 'b' was inserted
        send_key(&mut ta, KeyCode::Char('z'), KeyModifiers::CONTROL);
        assert_eq!(ta.value(), "a");

        // Undo again -> empty
        send_key(&mut ta, KeyCode::Char('z'), KeyModifiers::CONTROL);
        assert_eq!(ta.value(), "");

        // Redo -> "a"
        send_key(&mut ta, KeyCode::Char('y'), KeyModifiers::CONTROL);
        assert_eq!(ta.value(), "a");

        // Redo -> "ab"
        send_key(&mut ta, KeyCode::Char('y'), KeyModifiers::CONTROL);
        assert_eq!(ta.value(), "ab");
    }

    #[test]
    fn test_undo_clears_redo_on_new_edit() {
        let mut ta = TextArea::new();
        ta.focus();
        send_key(&mut ta, KeyCode::Char('a'), KeyModifiers::NONE);
        send_key(&mut ta, KeyCode::Char('b'), KeyModifiers::NONE);
        // Undo -> "a"
        send_key(&mut ta, KeyCode::Char('z'), KeyModifiers::CONTROL);
        assert_eq!(ta.value(), "a");
        // Type 'c' -> redo stack should be cleared
        send_key(&mut ta, KeyCode::Char('c'), KeyModifiers::NONE);
        assert_eq!(ta.value(), "ac");
        // Redo should do nothing
        send_key(&mut ta, KeyCode::Char('y'), KeyModifiers::CONTROL);
        assert_eq!(ta.value(), "ac");
    }

    #[test]
    fn test_undo_stack_cap_at_100() {
        let mut ta = TextArea::new();
        ta.focus();
        for i in 0..110 {
            let c = char::from(b'a' + (i % 26) as u8);
            send_key(&mut ta, KeyCode::Char(c), KeyModifiers::NONE);
        }
        assert!(ta.undo_stack.len() <= 100);
    }

    #[test]
    fn test_undo_enter() {
        let mut ta = TextArea::new().with_content("hello");
        ta.focus();
        ta.cursor_col = 5;
        send_key(&mut ta, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(ta.line_count(), 2);
        send_key(&mut ta, KeyCode::Char('z'), KeyModifiers::CONTROL);
        assert_eq!(ta.value(), "hello");
        assert_eq!(ta.line_count(), 1);
    }

    #[test]
    fn test_undo_backspace_at_line_boundary() {
        let mut ta = TextArea::new().with_content("hello\nworld");
        ta.focus();
        ta.cursor_row = 1;
        ta.cursor_col = 0;
        send_key(&mut ta, KeyCode::Backspace, KeyModifiers::NONE);
        assert_eq!(ta.value(), "helloworld");
        send_key(&mut ta, KeyCode::Char('z'), KeyModifiers::CONTROL);
        assert_eq!(ta.value(), "hello\nworld");
    }

    #[test]
    fn test_uppercase_word() {
        let mut ta = TextArea::new().with_content("hello world");
        ta.focus();
        ta.cursor_col = 0;
        send_key(&mut ta, KeyCode::Char('u'), KeyModifiers::ALT);
        assert_eq!(ta.value(), "HELLO world");
        assert_eq!(ta.cursor_col(), 5);
    }

    #[test]
    fn test_lowercase_word() {
        let mut ta = TextArea::new().with_content("HELLO WORLD");
        ta.focus();
        ta.cursor_col = 0;
        send_key(&mut ta, KeyCode::Char('l'), KeyModifiers::ALT);
        assert_eq!(ta.value(), "hello WORLD");
        assert_eq!(ta.cursor_col(), 5);
    }

    #[test]
    fn test_capitalize_word() {
        let mut ta = TextArea::new().with_content("hello world");
        ta.focus();
        ta.cursor_col = 0;
        send_key(&mut ta, KeyCode::Char('c'), KeyModifiers::ALT);
        assert_eq!(ta.value(), "Hello world");
        assert_eq!(ta.cursor_col(), 5);
    }

    #[test]
    fn test_capitalize_word_uppercase_input() {
        let mut ta = TextArea::new().with_content("hELLO world");
        ta.focus();
        ta.cursor_col = 0;
        send_key(&mut ta, KeyCode::Char('c'), KeyModifiers::ALT);
        assert_eq!(ta.value(), "Hello world");
        assert_eq!(ta.cursor_col(), 5);
    }

    #[test]
    fn test_word_case_at_end_of_line() {
        let mut ta = TextArea::new().with_content("hello");
        ta.focus();
        ta.cursor_col = 5; // at end of line
        send_key(&mut ta, KeyCode::Char('u'), KeyModifiers::ALT);
        // Should be a no-op since cursor is at end
        assert_eq!(ta.value(), "hello");
        assert_eq!(ta.cursor_col(), 5);
    }

    #[test]
    fn test_word_case_undo() {
        let mut ta = TextArea::new().with_content("hello world");
        ta.focus();
        ta.cursor_col = 0;
        send_key(&mut ta, KeyCode::Char('u'), KeyModifiers::ALT);
        assert_eq!(ta.value(), "HELLO world");
        send_key(&mut ta, KeyCode::Char('z'), KeyModifiers::CONTROL);
        assert_eq!(ta.value(), "hello world");
    }

    #[test]
    fn test_soft_wrap_builder() {
        let ta = TextArea::new().with_soft_wrap(true);
        assert!(ta.soft_wrap);
    }

    #[test]
    fn test_line_prompt_builder() {
        let ta = TextArea::new().with_line_prompt("> ");
        assert_eq!(ta.line_prompt, Some("> ".to_string()));
    }

    #[test]
    fn test_copy_emits_message() {
        let mut ta = TextArea::new().with_content("hello world");
        ta.focus();
        // Select "hello"
        ta.selection_start = Some((0, 0));
        ta.cursor_col = 5;
        let cmd = send_key(&mut ta, KeyCode::Char('c'), KeyModifiers::CONTROL);
        match extract_message(cmd) {
            Some(Message::Copy(text)) => assert_eq!(text, "hello"),
            other => panic!(
                "Expected Copy message, got {:?}",
                other.map(|m| format!("{:?}", m))
            ),
        }
        // Selection should still be there (copy doesn't remove)
        assert!(ta.has_selection());
    }

    #[test]
    fn test_copy_no_selection() {
        let mut ta = TextArea::new().with_content("hello world");
        ta.focus();
        let cmd = send_key(&mut ta, KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert!(cmd.is_none());
    }

    #[test]
    fn test_cut_emits_message_and_deletes() {
        let mut ta = TextArea::new().with_content("hello world");
        ta.focus();
        ta.selection_start = Some((0, 0));
        ta.cursor_col = 5;
        let cmd = send_key(&mut ta, KeyCode::Char('x'), KeyModifiers::CONTROL);
        // The value should have "hello" removed
        assert_eq!(ta.value(), " world");
        // Should emit Cut + Changed via batch
        let batch = cmd.into_batch().expect("Expected Batch command");
        assert_eq!(batch.len(), 2);
    }

    #[test]
    fn test_cut_no_selection() {
        let mut ta = TextArea::new().with_content("hello world");
        ta.focus();
        let cmd = send_key(&mut ta, KeyCode::Char('x'), KeyModifiers::CONTROL);
        assert!(cmd.is_none());
    }

    #[test]
    fn test_paste_message() {
        let mut ta = TextArea::new();
        ta.focus();
        let cmd = ta.update(Message::Paste("hello\nworld".to_string()));
        assert_eq!(ta.value(), "hello\nworld");
        assert_eq!(ta.line_count(), 2);
        match cmd.into_message() {
            Some(Message::Changed(_)) => {}
            other => panic!(
                "Expected Changed message, got {:?}",
                other.map(|m| format!("{:?}", m))
            ),
        }
    }

    #[test]
    fn test_paste_with_selection_replaces() {
        let mut ta = TextArea::new().with_content("XXXXX");
        ta.focus();
        ta.selection_start = Some((0, 0));
        ta.cursor_col = 5;
        ta.update(Message::Paste("hello".to_string()));
        assert_eq!(ta.value(), "hello");
    }

    #[test]
    fn test_delete_key_undo() {
        let mut ta = TextArea::new().with_content("hello");
        ta.focus();
        ta.cursor_col = 0;
        send_key(&mut ta, KeyCode::Delete, KeyModifiers::NONE);
        assert_eq!(ta.value(), "ello");
        send_key(&mut ta, KeyCode::Char('z'), KeyModifiers::CONTROL);
        assert_eq!(ta.value(), "hello");
    }

    #[test]
    fn test_tab_undo() {
        let mut ta = TextArea::new();
        ta.focus();
        send_key(&mut ta, KeyCode::Tab, KeyModifiers::NONE);
        assert_eq!(ta.value(), "    ");
        send_key(&mut ta, KeyCode::Char('z'), KeyModifiers::CONTROL);
        assert_eq!(ta.value(), "");
    }

    #[test]
    fn test_kill_to_end_undo() {
        let mut ta = TextArea::new().with_content("hello world");
        ta.focus();
        ta.cursor_col = 5;
        send_key(&mut ta, KeyCode::Char('k'), KeyModifiers::CONTROL);
        assert_eq!(ta.value(), "hello");
        send_key(&mut ta, KeyCode::Char('z'), KeyModifiers::CONTROL);
        assert_eq!(ta.value(), "hello world");
    }

    #[test]
    fn test_kill_to_start_undo() {
        let mut ta = TextArea::new().with_content("hello world");
        ta.focus();
        ta.cursor_col = 5;
        send_key(&mut ta, KeyCode::Char('u'), KeyModifiers::CONTROL);
        assert_eq!(ta.value(), " world");
        send_key(&mut ta, KeyCode::Char('z'), KeyModifiers::CONTROL);
        assert_eq!(ta.value(), "hello world");
    }

    #[test]
    fn test_delete_word_backward_undo() {
        let mut ta = TextArea::new().with_content("hello world");
        ta.focus();
        ta.cursor_col = 11;
        send_key(&mut ta, KeyCode::Char('w'), KeyModifiers::CONTROL);
        assert_eq!(ta.value(), "hello ");
        send_key(&mut ta, KeyCode::Char('z'), KeyModifiers::CONTROL);
        assert_eq!(ta.value(), "hello world");
    }

    #[test]
    fn test_delete_word_forward_undo() {
        let mut ta = TextArea::new().with_content("hello world");
        ta.focus();
        ta.cursor_col = 0;
        send_key(&mut ta, KeyCode::Char('d'), KeyModifiers::ALT);
        assert_eq!(ta.value(), "world");
        send_key(&mut ta, KeyCode::Char('z'), KeyModifiers::CONTROL);
        assert_eq!(ta.value(), "hello world");
    }

    #[test]
    fn test_history_browse_single_line() {
        let mut ta = TextArea::new().with_history(100);
        ta.focus();
        ta.push_history("first");
        ta.push_history("second");

        // Type draft
        send_key(&mut ta, KeyCode::Char('d'), KeyModifiers::NONE);
        assert_eq!(ta.value(), "d");

        // Up → most recent
        send_key(&mut ta, KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(ta.value(), "second");

        // Up → older
        send_key(&mut ta, KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(ta.value(), "first");

        // Down → newer
        send_key(&mut ta, KeyCode::Down, KeyModifiers::NONE);
        assert_eq!(ta.value(), "second");

        // Down → back to draft
        send_key(&mut ta, KeyCode::Down, KeyModifiers::NONE);
        assert_eq!(ta.value(), "d");
    }

    #[test]
    fn test_history_does_not_activate_on_multiline() {
        let mut ta = TextArea::new()
            .with_history(100)
            .with_content("line1\nline2");
        ta.focus();
        ta.push_history("old");

        // Buffer has 2 lines; Up should move cursor, not browse history
        ta.cursor_row = 1;
        ta.cursor_col = 0;
        send_key(&mut ta, KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(ta.cursor_row(), 0);
        // Value unchanged (not history entry)
        assert_eq!(ta.value(), "line1\nline2");
    }

    #[test]
    fn single_line_enter_submits() {
        let mut ta = TextArea::new().with_single_line(true);
        ta.focus();
        send_key(&mut ta, KeyCode::Char('h'), KeyModifiers::NONE);
        send_key(&mut ta, KeyCode::Char('i'), KeyModifiers::NONE);
        let cmd = send_key(&mut ta, KeyCode::Enter, KeyModifiers::NONE);
        match cmd.into_message() {
            Some(Message::Submit(s)) => assert_eq!(s, "hi"),
            other => panic!("Expected Submit(\"hi\"), got {:?}", other),
        }
        assert_eq!(ta.line_count(), 1);
    }

    #[test]
    fn single_line_paste_strips_newlines() {
        let mut ta = TextArea::new().with_single_line(true);
        ta.focus();
        ta.update(Message::Paste("hello\nworld\nfoo".into()));
        assert_eq!(ta.value(), "helloworldfoo");
        assert_eq!(ta.line_count(), 1);
    }

    #[test]
    fn single_line_up_down_browse_history() {
        let mut ta = TextArea::new()
            .with_single_line(true)
            .with_history(10);
        ta.focus();
        ta.push_history("first");
        ta.push_history("second");
        send_key(&mut ta, KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(ta.value(), "second");
        send_key(&mut ta, KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(ta.value(), "first");
        send_key(&mut ta, KeyCode::Down, KeyModifiers::NONE);
        assert_eq!(ta.value(), "second");
    }

    #[test]
    fn multiline_submit_with_ctrl_enter() {
        let mut ta = TextArea::new()
            .with_submit(SubmitBinding::CtrlEnter);
        ta.focus();
        send_key(&mut ta, KeyCode::Char('h'), KeyModifiers::NONE);
        send_key(&mut ta, KeyCode::Char('i'), KeyModifiers::NONE);
        send_key(&mut ta, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(ta.line_count(), 2);
        let cmd = send_key(&mut ta, KeyCode::Enter, KeyModifiers::CONTROL);
        match cmd.into_message() {
            Some(Message::Submit(s)) => assert_eq!(s, "hi\n"),
            other => panic!("Expected Submit, got {:?}", other),
        }
    }

    #[test]
    fn multiline_enter_submit_shift_enter_newline() {
        let mut ta = TextArea::new()
            .with_submit(SubmitBinding::Enter);
        ta.focus();
        send_key(&mut ta, KeyCode::Char('a'), KeyModifiers::NONE);
        send_key(&mut ta, KeyCode::Enter, KeyModifiers::SHIFT);
        assert_eq!(ta.line_count(), 2);
        let cmd = send_key(&mut ta, KeyCode::Enter, KeyModifiers::NONE);
        match cmd.into_message() {
            Some(Message::Submit(_)) => {}
            other => panic!("Expected Submit, got {:?}", other),
        }
    }

    #[test]
    fn multiline_no_submit_binding_all_enters_newline() {
        let mut ta = TextArea::new();
        ta.focus();
        send_key(&mut ta, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(ta.line_count(), 2);
    }

    #[test]
    fn test_history_cursor_at_end_after_browse() {
        let mut ta = TextArea::new().with_history(100);
        ta.focus();
        ta.push_history("hello world");

        send_key(&mut ta, KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(ta.value(), "hello world");
        assert_eq!(ta.cursor_col(), 11);
        assert_eq!(ta.cursor_row(), 0);
    }

    #[test]
    fn single_line_h_offset_accessor() {
        let ta = TextArea::new().with_single_line(true);
        assert_eq!(ta.h_offset(), 0);
    }

    #[test]
    fn single_line_h_offset_advances_with_cursor() {
        let mut ta = TextArea::new().with_single_line(true);
        ta.focus();
        // Type enough characters to exceed a small visible width
        for c in "abcdefghijklmnopqrstuvwxyz".chars() {
            send_key(&mut ta, KeyCode::Char(c), KeyModifiers::NONE);
        }
        assert_eq!(ta.cursor_col(), 26);
        // After updating h_offset with a small available width, offset should advance
        ta.update_h_offset(10);
        assert!(ta.h_offset() > 0, "h_offset should advance when cursor is past visible width");
        // Cursor should be visible: h_offset <= cursor_col < h_offset + available
        assert!(ta.cursor_col() >= ta.h_offset());
        assert!(ta.cursor_col() < ta.h_offset() + 10);
    }

    #[test]
    fn single_line_h_offset_tracks_back() {
        let mut ta = TextArea::new().with_single_line(true);
        ta.focus();
        // Type enough characters to force scrolling
        for c in "abcdefghijklmnopqrstuvwxyz".chars() {
            send_key(&mut ta, KeyCode::Char(c), KeyModifiers::NONE);
        }
        ta.update_h_offset(10);
        assert!(ta.h_offset() > 0);

        // Move cursor back to the beginning
        send_key(&mut ta, KeyCode::Home, KeyModifiers::NONE);
        assert_eq!(ta.cursor_col(), 0);
        ta.update_h_offset(10);
        assert_eq!(ta.h_offset(), 0, "h_offset should track back to 0 when cursor is at start");
    }

    #[test]
    fn placeholder_builder() {
        let ta = TextArea::new().with_placeholder("Type here...");
        assert_eq!(ta.value(), "");
    }

    #[test]
    fn prompt_builder() {
        let ta = TextArea::new().with_prompt("> ");
        assert_eq!(ta.value(), "");
    }

    #[test]
    fn echo_mode_password() {
        let mut ta = TextArea::new()
            .with_single_line(true)
            .with_echo_mode(EchoMode::Password('*'));
        ta.focus();
        send_key(&mut ta, KeyCode::Char('s'), KeyModifiers::NONE);
        send_key(&mut ta, KeyCode::Char('e'), KeyModifiers::NONE);
        send_key(&mut ta, KeyCode::Char('c'), KeyModifiers::NONE);
        assert_eq!(ta.value(), "sec");
        assert!(matches!(ta.echo_mode(), EchoMode::Password('*')));
    }

    #[test]
    fn echo_mode_hidden() {
        let ta = TextArea::new()
            .with_single_line(true)
            .with_echo_mode(EchoMode::Hidden)
            .with_content("secret");
        assert_eq!(ta.value(), "secret");
        assert!(matches!(ta.echo_mode(), EchoMode::Hidden));
    }

    #[test]
    fn suggestions_filter_as_user_types() {
        let mut ta = TextArea::new()
            .with_single_line(true)
            .with_suggestions(vec!["apple".into(), "banana".into(), "apricot".into()]);
        ta.focus();
        send_key(&mut ta, KeyCode::Char('a'), KeyModifiers::NONE);
        let avail = ta.available_suggestions();
        assert_eq!(avail.len(), 2);
        assert!(avail.contains(&"apple".to_string()));
        assert!(avail.contains(&"apricot".to_string()));
    }

    #[test]
    fn tab_accepts_suggestion() {
        let mut ta = TextArea::new()
            .with_single_line(true)
            .with_suggestions(vec!["apple".into()]);
        ta.focus();
        send_key(&mut ta, KeyCode::Char('a'), KeyModifiers::NONE);
        assert_eq!(ta.current_suggestion(), Some("apple"));
        send_key(&mut ta, KeyCode::Tab, KeyModifiers::NONE);
        assert_eq!(ta.value(), "apple");
    }

    #[test]
    fn suggestions_ignored_in_multiline() {
        let mut ta = TextArea::new()
            .with_suggestions(vec!["apple".into()]);
        ta.focus();
        send_key(&mut ta, KeyCode::Char('a'), KeyModifiers::NONE);
        assert_eq!(ta.current_suggestion(), Option::<&str>::None);
    }

    #[test]
    fn right_arrow_at_end_accepts_suggestion() {
        let mut ta = TextArea::new()
            .with_single_line(true)
            .with_suggestions(vec!["apple".into()]);
        ta.focus();
        send_key(&mut ta, KeyCode::Char('a'), KeyModifiers::NONE);
        assert_eq!(ta.current_suggestion(), Some("apple"));
        send_key(&mut ta, KeyCode::Right, KeyModifiers::NONE);
        assert_eq!(ta.value(), "apple");
    }

    #[test]
    fn exact_match_clears_suggestions() {
        let mut ta = TextArea::new()
            .with_single_line(true)
            .with_suggestions(vec!["hi".into()]);
        ta.focus();
        send_key(&mut ta, KeyCode::Char('h'), KeyModifiers::NONE);
        assert_eq!(ta.current_suggestion(), Some("hi"));
        send_key(&mut ta, KeyCode::Char('i'), KeyModifiers::NONE);
        assert_eq!(ta.current_suggestion(), None);
    }

    #[test]
    fn validation_sets_error() {
        let mut ta = TextArea::new()
            .with_single_line(true)
            .with_validate(|s| {
                if s.len() > 3 { Err("Too long".into()) } else { Ok(()) }
            });
        ta.focus();
        send_key(&mut ta, KeyCode::Char('a'), KeyModifiers::NONE);
        assert!(ta.err().is_none());
        send_key(&mut ta, KeyCode::Char('b'), KeyModifiers::NONE);
        send_key(&mut ta, KeyCode::Char('c'), KeyModifiers::NONE);
        send_key(&mut ta, KeyCode::Char('d'), KeyModifiers::NONE);
        assert_eq!(ta.err(), Some("Too long"));
    }

    #[test]
    fn validation_clears_on_valid() {
        let mut ta = TextArea::new()
            .with_single_line(true)
            .with_validate(|s| {
                if s.is_empty() { Err("Required".into()) } else { Ok(()) }
            });
        ta.focus();
        assert!(ta.err().is_none());
        send_key(&mut ta, KeyCode::Char('a'), KeyModifiers::NONE);
        assert!(ta.err().is_none());
        send_key(&mut ta, KeyCode::Backspace, KeyModifiers::NONE);
        assert_eq!(ta.err(), Some("Required"));
        send_key(&mut ta, KeyCode::Char('b'), KeyModifiers::NONE);
        assert!(ta.err().is_none());
    }
}
