//! Autocomplete input: a text input with a filtered dropdown of suggestions.
//!
//! The parent provides a list of suggestion strings. As the user types, the
//! dropdown shows matching items.  Arrow keys navigate the dropdown, Tab or
//! Enter accepts the selected suggestion, and Esc dismisses it.
//!
//! # Example
//!
//! ```ignore
//! use boba_widgets::autocomplete::Autocomplete;
//!
//! let ac = Autocomplete::new()
//!     .with_suggestions(vec!["apple", "banana", "cherry"])
//!     .with_max_visible(5);
//! ```

use boba_core::command::Command;
use boba_core::component::Component;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};
use ratatui::Frame;

use crate::text_edit::TextEditState;

/// Messages for the autocomplete component.
#[derive(Debug, Clone)]
pub enum Message {
    /// A key press event.
    KeyPress(KeyEvent),
    /// The input value changed.
    InputChanged(String),
    /// A suggestion was accepted.
    Accepted(String),
    /// The dropdown was dismissed.
    Dismissed,
}

/// Style configuration for the autocomplete.
#[derive(Debug, Clone)]
pub struct AutocompleteStyle {
    /// Style for the input text.
    pub input: Style,
    /// Style for the input prompt.
    pub prompt: Style,
    /// Style for the cursor.
    pub cursor: Style,
    /// Style for unselected dropdown items.
    pub item: Style,
    /// Style for the selected dropdown item.
    pub selected_item: Style,
}

impl Default for AutocompleteStyle {
    fn default() -> Self {
        Self {
            input: Style::default(),
            prompt: Style::default().fg(Color::Cyan),
            cursor: Style::default().add_modifier(Modifier::REVERSED),
            item: Style::default().fg(Color::White),
            selected_item: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        }
    }
}

/// Autocomplete input component.
pub struct Autocomplete {
    editor: TextEditState,
    suggestions: Vec<String>,
    filtered: Vec<String>,
    selected: usize,
    dropdown_visible: bool,
    max_visible: usize,
    scroll_offset: usize,
    style: AutocompleteStyle,
    prompt: String,
    focused: bool,
    dropdown_block: Option<Block<'static>>,
}

impl Default for Autocomplete {
    fn default() -> Self {
        Self::new()
    }
}

impl Autocomplete {
    /// Create a new autocomplete input.
    pub fn new() -> Self {
        Self {
            editor: TextEditState::new(),
            suggestions: Vec::new(),
            filtered: Vec::new(),
            selected: 0,
            dropdown_visible: false,
            max_visible: 8,
            scroll_offset: 0,
            style: AutocompleteStyle::default(),
            prompt: String::new(),
            focused: false,
            dropdown_block: None,
        }
    }

    /// Set the list of suggestions.
    pub fn with_suggestions(mut self, suggestions: Vec<impl Into<String>>) -> Self {
        self.suggestions = suggestions.into_iter().map(|s| s.into()).collect();
        self
    }

    /// Set the maximum number of visible dropdown items.
    pub fn with_max_visible(mut self, max: usize) -> Self {
        self.max_visible = max.max(1);
        self
    }

    /// Set the style.
    pub fn with_style(mut self, style: AutocompleteStyle) -> Self {
        self.style = style;
        self
    }

    /// Set the block (border/title container) for the dropdown overlay.
    pub fn with_dropdown_block(mut self, block: Block<'static>) -> Self {
        self.dropdown_block = Some(block);
        self
    }

    /// Set the prompt text.
    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = prompt.into();
        self
    }

    /// Get the current input value.
    pub fn value(&self) -> String {
        self.editor.value()
    }

    /// Set the input value programmatically.
    pub fn set_value(&mut self, value: impl Into<String>) {
        let v: String = value.into();
        self.editor.set_value(&v);
        self.update_filtered();
    }

    /// Update the suggestions list.
    pub fn set_suggestions(&mut self, suggestions: Vec<String>) {
        self.suggestions = suggestions;
        self.update_filtered();
    }

    /// Whether the dropdown is currently visible.
    pub fn is_dropdown_visible(&self) -> bool {
        self.dropdown_visible
    }

    /// Set focus state.
    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
        if !focused {
            self.dropdown_visible = false;
        }
    }

    /// Get the currently selected suggestion, if any.
    pub fn selected_suggestion(&self) -> Option<&str> {
        self.filtered.get(self.selected).map(|s| s.as_str())
    }

    fn update_filtered(&mut self) {
        if self.editor.is_empty() {
            self.filtered = self.suggestions.clone();
        } else {
            let query = self.editor.value().to_lowercase();
            self.filtered = self
                .suggestions
                .iter()
                .filter(|s| s.to_lowercase().contains(&query))
                .cloned()
                .collect();
        }
        self.selected = 0;
        self.scroll_offset = 0;
        self.dropdown_visible = !self.filtered.is_empty();
    }

    fn ensure_selected_visible(&mut self) {
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if self.selected >= self.scroll_offset + self.max_visible {
            self.scroll_offset = self.selected.saturating_sub(self.max_visible - 1);
        }
    }

}

impl Component for Autocomplete {
    type Message = Message;

    fn update(&mut self, msg: Message) -> Command<Message> {
        match msg {
            Message::KeyPress(key) => match (key.code, key.modifiers) {
                (KeyCode::Esc, _) => {
                    if self.dropdown_visible {
                        self.dropdown_visible = false;
                        Command::message(Message::Dismissed)
                    } else {
                        Command::none()
                    }
                }
                (KeyCode::Down, _) if self.dropdown_visible => {
                    if self.selected + 1 < self.filtered.len() {
                        self.selected += 1;
                        self.ensure_selected_visible();
                    }
                    Command::none()
                }
                (KeyCode::Up, _) if self.dropdown_visible => {
                    if self.selected > 0 {
                        self.selected -= 1;
                        self.ensure_selected_visible();
                    }
                    Command::none()
                }
                (KeyCode::Tab, _) | (KeyCode::Enter, _) if self.dropdown_visible => {
                    if let Some(suggestion) = self.filtered.get(self.selected).cloned() {
                        self.editor.set_value(&suggestion);
                        self.dropdown_visible = false;
                        Command::message(Message::Accepted(suggestion))
                    } else {
                        Command::none()
                    }
                }
                (KeyCode::Backspace, _) => {
                    if self.editor.delete_back() {
                        self.update_filtered();
                        Command::message(Message::InputChanged(self.editor.value()))
                    } else {
                        Command::none()
                    }
                }
                (KeyCode::Left, _) => {
                    self.editor.move_left();
                    Command::none()
                }
                (KeyCode::Right, _) => {
                    self.editor.move_right();
                    Command::none()
                }
                (KeyCode::Home, _) => {
                    self.editor.move_home();
                    Command::none()
                }
                (KeyCode::End, _) => {
                    self.editor.move_end();
                    Command::none()
                }
                (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                    self.editor.insert_char(c);
                    self.update_filtered();
                    Command::message(Message::InputChanged(self.editor.value()))
                }
                _ => Command::none(),
            },
            Message::InputChanged(_) | Message::Accepted(_) | Message::Dismissed => Command::none(),
        }
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        if area.height == 0 {
            return;
        }

        // Input line (first row of area)
        let input_area = Rect { height: 1, ..area };

        let mut spans = Vec::new();
        if !self.prompt.is_empty() {
            spans.push(Span::styled(format!("{} ", self.prompt), self.style.prompt));
        }

        // Value with cursor
        let chars = self.editor.chars();
        let cursor = self.editor.cursor();
        if chars.is_empty() {
            spans.push(Span::styled(" ", self.style.cursor));
        } else {
            let before: String = chars[..cursor].iter().collect();
            if !before.is_empty() {
                spans.push(Span::styled(before, self.style.input));
            }
            if cursor < chars.len() {
                let cursor_char: String = chars[cursor..cursor + 1].iter().collect();
                spans.push(Span::styled(cursor_char, self.style.cursor));
                let after: String = chars[cursor + 1..].iter().collect();
                if !after.is_empty() {
                    spans.push(Span::styled(after, self.style.input));
                }
            } else {
                spans.push(Span::styled(" ", self.style.cursor));
            }
        }

        let input = Paragraph::new(Line::from(spans));
        frame.render_widget(input, input_area);

        // Dropdown (below the input, if visible and we have space)
        if self.dropdown_visible && area.height > 1 {
            let visible_count = self.filtered.len().min(self.max_visible);
            let dropdown_height = if self.dropdown_block.is_some() {
                (visible_count as u16 + 2).min(area.height - 1) // +2 for borders
            } else {
                (visible_count as u16).min(area.height - 1)
            };
            let dropdown_area = Rect {
                x: area.x,
                y: area.y + 1,
                width: area.width,
                height: dropdown_height,
            };

            frame.render_widget(Clear, dropdown_area);

            let inner = if let Some(ref block) = self.dropdown_block {
                let inner = block.inner(dropdown_area);
                frame.render_widget(block.clone(), dropdown_area);
                inner
            } else {
                dropdown_area
            };

            // Render visible items
            let end = (self.scroll_offset + self.max_visible).min(self.filtered.len());
            for (i, idx) in (self.scroll_offset..end).enumerate() {
                if i as u16 >= inner.height {
                    break;
                }
                let item_area = Rect {
                    x: inner.x,
                    y: inner.y + i as u16,
                    width: inner.width,
                    height: 1,
                };
                let style = if idx == self.selected {
                    self.style.selected_item
                } else {
                    self.style.item
                };
                let prefix = if idx == self.selected { "▸ " } else { "  " };
                let text = format!("{}{}", prefix, &self.filtered[idx]);
                frame.render_widget(
                    Paragraph::new(Line::from(Span::styled(text, style))),
                    item_area,
                );
            }
        }
    }

    fn focused(&self) -> bool {
        self.focused
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

    #[test]
    fn typing_filters_suggestions() {
        let mut ac = Autocomplete::new().with_suggestions(vec!["apple", "banana", "avocado"]);
        ac.set_focused(true);

        ac.update(Message::KeyPress(key(KeyCode::Char('a'))));
        assert_eq!(ac.value(), "a");
        assert_eq!(ac.filtered.len(), 3); // apple, banana (contains 'a'), avocado
        assert!(ac.is_dropdown_visible());
    }

    #[test]
    fn arrow_keys_navigate() {
        let mut ac = Autocomplete::new().with_suggestions(vec!["apple", "avocado", "apricot"]);
        ac.set_focused(true);
        ac.update(Message::KeyPress(key(KeyCode::Char('a'))));

        assert_eq!(ac.selected, 0);
        ac.update(Message::KeyPress(key(KeyCode::Down)));
        assert_eq!(ac.selected, 1);
        ac.update(Message::KeyPress(key(KeyCode::Down)));
        assert_eq!(ac.selected, 2);
        // Can't go past last
        ac.update(Message::KeyPress(key(KeyCode::Down)));
        assert_eq!(ac.selected, 2);
        ac.update(Message::KeyPress(key(KeyCode::Up)));
        assert_eq!(ac.selected, 1);
    }

    #[test]
    fn tab_accepts_suggestion() {
        let mut ac = Autocomplete::new().with_suggestions(vec!["apple", "banana"]);
        ac.set_focused(true);
        ac.update(Message::KeyPress(key(KeyCode::Char('a'))));
        // "apple" is first filtered match
        let cmd = ac.update(Message::KeyPress(key(KeyCode::Tab)));
        assert_eq!(ac.value(), "apple");
        assert!(!ac.is_dropdown_visible());
        match cmd.into_message() {
            Some(Message::Accepted(s)) => assert_eq!(s, "apple"),
            other => panic!(
                "Expected Accepted, got {:?}",
                other.map(|m| format!("{:?}", m))
            ),
        }
    }

    #[test]
    fn esc_dismisses_dropdown() {
        let mut ac = Autocomplete::new().with_suggestions(vec!["apple"]);
        ac.set_focused(true);
        ac.update(Message::KeyPress(key(KeyCode::Char('a'))));
        assert!(ac.is_dropdown_visible());

        let cmd = ac.update(Message::KeyPress(key(KeyCode::Esc)));
        assert!(!ac.is_dropdown_visible());
        assert!(matches!(cmd.into_message(), Some(Message::Dismissed)));
    }

    #[test]
    fn no_matching_suggestions_hides_dropdown() {
        let mut ac = Autocomplete::new().with_suggestions(vec!["apple", "banana"]);
        ac.set_focused(true);
        ac.update(Message::KeyPress(key(KeyCode::Char('z'))));
        assert!(!ac.is_dropdown_visible());
        assert!(ac.filtered.is_empty());
    }

    #[test]
    fn backspace_updates_filter() {
        let mut ac = Autocomplete::new().with_suggestions(vec!["apple", "banana", "avocado"]);
        ac.set_focused(true);
        ac.update(Message::KeyPress(key(KeyCode::Char('a'))));
        ac.update(Message::KeyPress(key(KeyCode::Char('p'))));
        assert_eq!(ac.filtered.len(), 1); // "apple"

        ac.update(Message::KeyPress(key(KeyCode::Backspace)));
        assert_eq!(ac.value(), "a");
        assert_eq!(ac.filtered.len(), 3); // "apple", "banana" (contains 'a'), "avocado"
    }

    #[test]
    fn set_suggestions_updates_filter() {
        let mut ac = Autocomplete::new();
        ac.set_focused(true);
        ac.set_value("b");
        ac.set_suggestions(vec!["apple".into(), "banana".into(), "blueberry".into()]);
        assert_eq!(ac.filtered.len(), 2); // "banana", "blueberry"
    }

    #[test]
    fn multibyte_chars_do_not_panic() {
        let mut ac = Autocomplete::new().with_suggestions(vec!["café", "naïve", "résumé"]);
        ac.set_focused(true);

        // Type multi-byte: "é"
        ac.update(Message::KeyPress(key(KeyCode::Char('é'))));
        assert_eq!(ac.value(), "é");
        assert_eq!(ac.editor.cursor(), 1);

        // Backspace removes it
        ac.update(Message::KeyPress(key(KeyCode::Backspace)));
        assert_eq!(ac.value(), "");
        assert_eq!(ac.editor.cursor(), 0);

        // Type "café" and navigate
        ac.update(Message::KeyPress(key(KeyCode::Char('c'))));
        ac.update(Message::KeyPress(key(KeyCode::Char('a'))));
        ac.update(Message::KeyPress(key(KeyCode::Char('f'))));
        ac.update(Message::KeyPress(key(KeyCode::Char('é'))));
        assert_eq!(ac.value(), "café");

        // Left then right
        ac.update(Message::KeyPress(key(KeyCode::Left)));
        ac.update(Message::KeyPress(key(KeyCode::Right)));
        assert_eq!(ac.editor.cursor(), 4);
    }

    #[test]
    fn max_visible_limits_display() {
        let ac = Autocomplete::new()
            .with_suggestions(vec!["a", "b", "c", "d", "e"])
            .with_max_visible(3);
        assert_eq!(ac.max_visible, 3);
    }
}
