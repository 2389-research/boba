//! Standalone dropdown overlay for displaying a list of selectable items.
//!
//! Items are managed externally — there is no built-in text input or
//! filtering. It renders as a bordered overlay anchored above or below a
//! given area. See the `autocomplete` example for composing this with a
//! [`TextInput`](crate::text_input::TextInput).

use crate::selection::SelectionState;
use boba_core::command::Command;
use boba_core::component::Component;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, Clear, Paragraph};
use ratatui::Frame;

/// Position of the dropdown relative to its anchor area.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Position {
    /// Render above the anchor area.
    Above,
    /// Render below the anchor area (default).
    #[default]
    Below,
}

/// Style configuration for the dropdown.
#[derive(Debug, Clone)]
pub struct DropdownStyle {
    /// Style for unselected items.
    pub item: Style,
    /// Style for the currently selected item.
    pub selected_item: Style,
}

impl Default for DropdownStyle {
    fn default() -> Self {
        Self {
            item: Style::default(),
            selected_item: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        }
    }
}

/// Messages for the dropdown component.
#[derive(Debug, Clone)]
pub enum Message {
    /// A key press event forwarded to the dropdown.
    KeyPress(KeyEvent),
    /// An item was selected (index, value).
    Selected(usize, String),
    /// The dropdown was dismissed (Esc pressed).
    Dismissed,
}

/// A standalone dropdown overlay for displaying selectable items.
///
/// # Example
///
/// ```ignore
/// use boba_widgets::dropdown::{Dropdown, Position};
///
/// let dropdown = Dropdown::new()
///     .with_position(Position::Above)
///     .with_max_visible(5)
///     .with_title(" Suggestions ");
/// ```
pub struct Dropdown {
    items: Vec<String>,
    selection: SelectionState,
    max_visible: usize,
    title: String,
    style: DropdownStyle,
    position: Position,
    visible: bool,
    block: Option<Block<'static>>,
}

impl Dropdown {
    /// Create a new empty dropdown.
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            selection: SelectionState::new(0, 8),
            max_visible: 8,
            title: String::new(),
            style: DropdownStyle::default(),
            position: Position::default(),
            visible: false,
            block: None,
        }
    }

    /// Set the dropdown title (shown in the border).
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Set the maximum number of visible items before scrolling.
    pub fn with_max_visible(mut self, max: usize) -> Self {
        self.max_visible = max.max(1);
        self.selection.set_visible(self.max_visible);
        self
    }

    /// Set the style configuration.
    pub fn with_style(mut self, style: DropdownStyle) -> Self {
        self.style = style;
        self
    }

    /// Set the position relative to the anchor.
    pub fn with_position(mut self, position: Position) -> Self {
        self.position = position;
        self
    }

    /// Set the block (border/title container) for the dropdown.
    pub fn with_block(mut self, block: Block<'static>) -> Self {
        self.block = Some(block);
        self
    }

    /// Set the items and show the dropdown. Resets selection and scroll.
    pub fn set_items(&mut self, items: Vec<String>) {
        self.visible = !items.is_empty();
        self.items = items;
        self.selection.set_count(self.items.len());
        self.selection.select(0);
    }

    /// Set the title (mutable variant).
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
    }

    /// Show the dropdown.
    pub fn show(&mut self) {
        self.visible = true;
    }

    /// Hide the dropdown.
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Whether the dropdown is currently visible.
    pub fn is_visible(&self) -> bool {
        self.visible && !self.items.is_empty()
    }

    /// Get the items.
    pub fn items(&self) -> &[String] {
        &self.items
    }

    /// Get the currently selected index.
    pub fn selected_index(&self) -> usize {
        self.selection.cursor()
    }

    /// Get the currently selected item value.
    pub fn selected_value(&self) -> Option<&str> {
        self.items.get(self.selection.cursor()).map(|s| s.as_str())
    }

    /// Set the selected index programmatically.
    pub fn set_selected(&mut self, index: usize) {
        self.selection.select(index);
    }

    fn select_next(&mut self) {
        self.selection.move_down();
    }

    fn select_prev(&mut self) {
        self.selection.move_up();
    }
}

impl Default for Dropdown {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for Dropdown {
    type Message = Message;

    fn update(&mut self, msg: Message) -> Command<Message> {
        match msg {
            Message::KeyPress(key) => match key.code {
                KeyCode::Esc => {
                    self.visible = false;
                    Command::message(Message::Dismissed)
                }
                KeyCode::Up => {
                    self.select_prev();
                    Command::none()
                }
                KeyCode::Down | KeyCode::Tab => {
                    self.select_next();
                    Command::none()
                }
                KeyCode::Enter => {
                    let idx = self.selection.cursor();
                    if let Some(value) = self.items.get(idx).cloned() {
                        self.visible = false;
                        Command::message(Message::Selected(idx, value))
                    } else {
                        Command::none()
                    }
                }
                _ => Command::none(),
            },
            Message::Selected(..) | Message::Dismissed => Command::none(),
        }
    }

    fn view(&self, frame: &mut Frame, anchor: Rect) {
        if !self.is_visible() {
            return;
        }

        let visible_count = self.items.len().min(self.max_visible);
        let dropdown_height = if self.block.is_some() {
            visible_count as u16 + 2 // +2 for borders
        } else {
            visible_count as u16
        };

        let dropdown_area = match self.position {
            Position::Above => {
                let y = anchor.y.saturating_sub(dropdown_height);
                let height = dropdown_height.min(anchor.y);
                Rect::new(anchor.x, y, anchor.width, height)
            }
            Position::Below => {
                let y = anchor.y + anchor.height;
                Rect::new(anchor.x, y, anchor.width, dropdown_height)
            }
        };

        if dropdown_area.height == 0 || dropdown_area.width < 4 {
            return; // not enough space
        }

        // Clear area behind dropdown
        frame.render_widget(Clear, dropdown_area);

        let inner = if let Some(ref block) = self.block {
            let inner = block.inner(dropdown_area);
            frame.render_widget(block.clone(), dropdown_area);
            inner
        } else {
            dropdown_area
        };

        // Render items
        let offset = self.selection.offset();
        for (i, item) in self
            .items
            .iter()
            .skip(offset)
            .take(visible_count)
            .enumerate()
        {
            let row_area = Rect {
                y: inner.y + i as u16,
                height: 1,
                ..inner
            };

            let is_selected = i + offset == self.selection.cursor();
            let style = if is_selected {
                self.style.selected_item
            } else {
                self.style.item
            };
            let prefix = if is_selected { "▸ " } else { "  " };

            // Truncate if needed
            let max_text_width = row_area.width.saturating_sub(2) as usize; // prefix is 2 chars
            let display = if item.len() > max_text_width {
                format!("{}{}...", prefix, &item[..max_text_width.saturating_sub(3)])
            } else {
                format!("{}{}", prefix, item)
            };

            frame.render_widget(Paragraph::new(Span::styled(display, style)), row_area);
        }
    }

    fn focused(&self) -> bool {
        self.visible
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEventKind, KeyEventState, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn new_is_hidden() {
        let dropdown = Dropdown::new();
        assert!(!dropdown.is_visible());
        assert!(dropdown.items().is_empty());
    }

    #[test]
    fn set_items_shows_dropdown() {
        let mut dropdown = Dropdown::new();
        dropdown.set_items(vec!["a".into(), "b".into()]);
        assert!(dropdown.is_visible());
        assert_eq!(dropdown.selected_index(), 0);
    }

    #[test]
    fn set_items_empty_stays_hidden() {
        let mut dropdown = Dropdown::new();
        dropdown.set_items(vec![]);
        assert!(!dropdown.is_visible());
    }

    #[test]
    fn set_items_resets_selection() {
        let mut dropdown = Dropdown::new();
        dropdown.set_items(vec!["a".into(), "b".into(), "c".into()]);
        dropdown.set_selected(2);
        assert_eq!(dropdown.selected_index(), 2);
        dropdown.set_items(vec!["x".into(), "y".into()]);
        assert_eq!(dropdown.selected_index(), 0);
    }

    #[test]
    fn down_navigates() {
        let mut dropdown = Dropdown::new();
        dropdown.set_items(vec!["a".into(), "b".into(), "c".into()]);

        dropdown.update(Message::KeyPress(key(KeyCode::Down)));
        assert_eq!(dropdown.selected_index(), 1);

        dropdown.update(Message::KeyPress(key(KeyCode::Down)));
        assert_eq!(dropdown.selected_index(), 2);

        // Wraps
        dropdown.update(Message::KeyPress(key(KeyCode::Down)));
        assert_eq!(dropdown.selected_index(), 0);
    }

    #[test]
    fn up_navigates() {
        let mut dropdown = Dropdown::new();
        dropdown.set_items(vec!["a".into(), "b".into(), "c".into()]);

        // Wraps to last
        dropdown.update(Message::KeyPress(key(KeyCode::Up)));
        assert_eq!(dropdown.selected_index(), 2);

        dropdown.update(Message::KeyPress(key(KeyCode::Up)));
        assert_eq!(dropdown.selected_index(), 1);
    }

    #[test]
    fn tab_navigates_forward() {
        let mut dropdown = Dropdown::new();
        dropdown.set_items(vec!["a".into(), "b".into()]);

        dropdown.update(Message::KeyPress(key(KeyCode::Tab)));
        assert_eq!(dropdown.selected_index(), 1);

        // Wraps
        dropdown.update(Message::KeyPress(key(KeyCode::Tab)));
        assert_eq!(dropdown.selected_index(), 0);
    }

    #[test]
    fn enter_selects() {
        let mut dropdown = Dropdown::new();
        dropdown.set_items(vec!["alpha".into(), "beta".into()]);
        dropdown.set_selected(1);

        let cmd = dropdown.update(Message::KeyPress(key(KeyCode::Enter)));
        match cmd.into_message() {
            Some(Message::Selected(1, value)) => assert_eq!(value, "beta"),
            other => panic!(
                "Expected Selected(1, beta), got {:?}",
                other.map(|m| format!("{:?}", m))
            ),
        }
        assert!(!dropdown.is_visible());
    }

    #[test]
    fn esc_dismisses() {
        let mut dropdown = Dropdown::new();
        dropdown.set_items(vec!["a".into()]);
        assert!(dropdown.is_visible());

        let cmd = dropdown.update(Message::KeyPress(key(KeyCode::Esc)));
        match cmd.into_message() {
            Some(Message::Dismissed) => {}
            other => panic!(
                "Expected Dismissed, got {:?}",
                other.map(|m| format!("{:?}", m))
            ),
        }
        assert!(!dropdown.is_visible());
    }

    #[test]
    fn scroll_offset_adjusts() {
        let mut dropdown = Dropdown::new().with_max_visible(2);
        dropdown.set_items(vec!["a".into(), "b".into(), "c".into(), "d".into()]);

        assert_eq!(dropdown.selection.offset(), 0);
        dropdown.update(Message::KeyPress(key(KeyCode::Down))); // sel=1
        assert_eq!(dropdown.selection.offset(), 0);
        dropdown.update(Message::KeyPress(key(KeyCode::Down))); // sel=2, scroll adjusts
        assert_eq!(dropdown.selection.offset(), 1);
        dropdown.update(Message::KeyPress(key(KeyCode::Down))); // sel=3
        assert_eq!(dropdown.selection.offset(), 2);
    }

    #[test]
    fn hide_and_show() {
        let mut dropdown = Dropdown::new();
        dropdown.set_items(vec!["a".into()]);
        assert!(dropdown.is_visible());

        dropdown.hide();
        assert!(!dropdown.is_visible());

        dropdown.show();
        assert!(dropdown.is_visible());
    }

    #[test]
    fn selected_value() {
        let mut dropdown = Dropdown::new();
        assert!(dropdown.selected_value().is_none());

        dropdown.set_items(vec!["hello".into(), "world".into()]);
        assert_eq!(dropdown.selected_value(), Some("hello"));

        dropdown.set_selected(1);
        assert_eq!(dropdown.selected_value(), Some("world"));
    }

    #[test]
    fn builders() {
        let dropdown = Dropdown::new()
            .with_title(" Test ")
            .with_max_visible(3)
            .with_position(Position::Above);

        assert_eq!(dropdown.title, " Test ");
        assert_eq!(dropdown.max_visible, 3);
        assert_eq!(dropdown.position, Position::Above);
    }
}
