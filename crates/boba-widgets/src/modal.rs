//! Generic modal/dialog overlay with title, body, and action buttons.
//!
//! The modal renders as a centered overlay on top of existing content.
//! It captures all input when visible (modal behavior) and supports
//! keyboard navigation of action buttons.

use boba_core::command::Command;
use boba_core::component::Component;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

/// Layout direction for action buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActionLayout {
    /// Actions displayed on a single horizontal line (default).
    #[default]
    Horizontal,
    /// Actions displayed vertically, one per line.
    Vertical,
}

/// A button in the modal dialog.
#[derive(Debug, Clone)]
pub struct Action {
    /// Display label for the button.
    pub label: String,
    /// Optional shortcut key (e.g., 'y' for Yes). Shown after the label.
    pub shortcut: Option<char>,
    /// Optional style override for this action when unfocused.
    pub style: Option<Style>,
    /// Optional style override for this action when focused.
    pub focused_style: Option<Style>,
}

impl Action {
    /// Create a new action with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            shortcut: None,
            style: None,
            focused_style: None,
        }
    }

    /// Set a shortcut key for this action.
    pub fn with_shortcut(mut self, key: char) -> Self {
        self.shortcut = Some(key);
        self
    }

    /// Set a custom style for this action when unfocused.
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }

    /// Set a custom style for this action when focused.
    pub fn with_focused_style(mut self, style: Style) -> Self {
        self.focused_style = Some(style);
        self
    }
}

/// Messages for the modal component.
#[derive(Debug, Clone)]
pub enum Message {
    /// A key press event forwarded to the modal.
    KeyPress(KeyEvent),
    /// An action was selected (by index).
    Select(usize),
    /// The modal was dismissed (Esc pressed).
    Dismiss,
}

/// Style configuration for the modal.
#[derive(Debug, Clone)]
pub struct ModalStyle {
    /// Border style for the modal window.
    pub border: Style,
    /// Style for the title text.
    pub title: Style,
    /// Style for the body text.
    pub body: Style,
    /// Style for unselected action buttons.
    pub action: Style,
    /// Style for the currently focused action button.
    pub focused_action: Style,
}

impl Default for ModalStyle {
    fn default() -> Self {
        Self {
            border: Style::default().fg(Color::Cyan),
            title: Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            body: Style::default(),
            action: Style::default().fg(Color::DarkGray),
            focused_action: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        }
    }
}

/// A centered modal dialog overlay.
///
/// # Example
///
/// ```ignore
/// use boba_widgets::modal::{Modal, Action};
///
/// let modal = Modal::new("Confirm")
///     .body("Are you sure you want to quit?")
///     .action(Action::new("Yes").with_shortcut('y'))
///     .action(Action::new("No").with_shortcut('n'));
/// ```
pub struct Modal {
    title: String,
    body: Vec<Line<'static>>,
    actions: Vec<Action>,
    focused_action: usize,
    style: ModalStyle,
    /// Width as a percentage of the terminal (0-100).
    width_percent: u16,
    /// Height as a percentage of the terminal (0-100).
    height_percent: u16,
    /// Layout direction for action buttons.
    action_layout: ActionLayout,
    /// Optional fixed width in columns (overrides percentage).
    fixed_width: Option<u16>,
    /// Optional fixed height in rows (overrides percentage).
    fixed_height: Option<u16>,
}

impl Modal {
    /// Create a new modal with the given title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            body: Vec::new(),
            actions: Vec::new(),
            focused_action: 0,
            style: ModalStyle::default(),
            width_percent: 50,
            height_percent: 40,
            action_layout: ActionLayout::default(),
            fixed_width: None,
            fixed_height: None,
        }
    }

    /// Set the body text (plain string, split into lines).
    pub fn body(mut self, text: impl Into<String>) -> Self {
        let s: String = text.into();
        self.body = s.lines().map(|l| Line::raw(l.to_string())).collect();
        self
    }

    /// Set the body as pre-styled lines.
    pub fn body_lines(mut self, lines: Vec<Line<'static>>) -> Self {
        self.body = lines;
        self
    }

    /// Add an action button to the modal.
    pub fn action(mut self, action: Action) -> Self {
        self.actions.push(action);
        self
    }

    /// Set the style configuration.
    pub fn with_style(mut self, style: ModalStyle) -> Self {
        self.style = style;
        self
    }

    /// Set the modal size as a percentage of the terminal.
    pub fn with_size(mut self, width_percent: u16, height_percent: u16) -> Self {
        self.width_percent = width_percent.min(100);
        self.height_percent = height_percent.min(100);
        self
    }

    /// Set the modal to a fixed size in columns and rows (overrides percentage sizing).
    pub fn with_fixed_size(mut self, width: u16, height: u16) -> Self {
        self.fixed_width = Some(width);
        self.fixed_height = Some(height);
        self
    }

    /// Set the action button layout direction.
    pub fn with_action_layout(mut self, layout: ActionLayout) -> Self {
        self.action_layout = layout;
        self
    }

    /// Get the index of the currently focused action.
    pub fn focused_action(&self) -> usize {
        self.focused_action
    }

    /// Set the focused action index programmatically.
    pub fn set_focused_action(&mut self, index: usize) {
        if !self.actions.is_empty() {
            self.focused_action = index.min(self.actions.len() - 1);
        }
    }

    /// Get the list of actions.
    pub fn actions(&self) -> &[Action] {
        &self.actions
    }

    /// Get the title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Compute a centered rect within the given area.
    fn centered_rect(&self, area: Rect) -> Rect {
        if let (Some(w), Some(h)) = (self.fixed_width, self.fixed_height) {
            let width = w.min(area.width);
            let height = h.min(area.height);
            let x = area.x + (area.width.saturating_sub(width)) / 2;
            let y = area.y + (area.height.saturating_sub(height)) / 2;
            Rect::new(x, y, width, height)
        } else {
            let v_margin = ((100 - self.height_percent) / 2).max(1);
            let h_margin = ((100 - self.width_percent) / 2).max(1);
            let vertical = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(v_margin),
                    Constraint::Percentage(self.height_percent),
                    Constraint::Percentage(v_margin),
                ])
                .split(area);
            let horizontal = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(h_margin),
                    Constraint::Percentage(self.width_percent),
                    Constraint::Percentage(h_margin),
                ])
                .split(vertical[1]);
            horizontal[1]
        }
    }
}

impl Component for Modal {
    type Message = Message;

    fn update(&mut self, msg: Message) -> Command<Message> {
        match msg {
            Message::KeyPress(key) => {
                match (key.code, key.modifiers) {
                    (KeyCode::Esc, _) => Command::message(Message::Dismiss),
                    (KeyCode::Left, _) | (KeyCode::Char('h'), KeyModifiers::NONE) => {
                        if !self.actions.is_empty() {
                            if self.focused_action > 0 {
                                self.focused_action -= 1;
                            } else {
                                self.focused_action = self.actions.len() - 1;
                            }
                        }
                        Command::none()
                    }
                    (KeyCode::Right, _)
                    | (KeyCode::Char('l'), KeyModifiers::NONE)
                    | (KeyCode::Tab, _) => {
                        if !self.actions.is_empty() {
                            self.focused_action =
                                (self.focused_action + 1) % self.actions.len();
                        }
                        Command::none()
                    }
                    (KeyCode::Up, _) | (KeyCode::Char('k'), KeyModifiers::NONE) => {
                        if !self.actions.is_empty() {
                            if self.focused_action > 0 {
                                self.focused_action -= 1;
                            } else {
                                self.focused_action = self.actions.len() - 1;
                            }
                        }
                        Command::none()
                    }
                    (KeyCode::Down, _) | (KeyCode::Char('j'), KeyModifiers::NONE) => {
                        if !self.actions.is_empty() {
                            self.focused_action =
                                (self.focused_action + 1) % self.actions.len();
                        }
                        Command::none()
                    }
                    (KeyCode::Enter, _) => {
                        if !self.actions.is_empty() {
                            Command::message(Message::Select(self.focused_action))
                        } else {
                            Command::message(Message::Dismiss)
                        }
                    }
                    // Number keys for quick focus (1-indexed)
                    (KeyCode::Char(c), KeyModifiers::NONE) if c.is_ascii_digit() && c != '0' => {
                        let idx = (c as u8 - b'1') as usize;
                        if idx < self.actions.len() {
                            self.focused_action = idx;
                        }
                        Command::none()
                    }
                    // Check shortcuts
                    (KeyCode::Char(c), KeyModifiers::NONE) => {
                        let lower = c.to_ascii_lowercase();
                        for (i, action) in self.actions.iter().enumerate() {
                            if action.shortcut == Some(lower) {
                                return Command::message(Message::Select(i));
                            }
                        }
                        Command::none()
                    }
                    _ => Command::none(),
                }
            }
            Message::Select(_) | Message::Dismiss => Command::none(),
        }
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        let modal_area = self.centered_rect(area);

        // Clear the area behind the modal
        frame.render_widget(Clear, modal_area);

        let block = Block::default()
            .title(self.title.as_str())
            .title_style(self.style.title)
            .borders(Borders::ALL)
            .border_style(self.style.border);

        let inner = block.inner(modal_area);
        frame.render_widget(block, modal_area);

        // Layout: body takes remaining space, actions at bottom
        let has_actions = !self.actions.is_empty();
        let action_height = if has_actions {
            match self.action_layout {
                ActionLayout::Horizontal => 2, // 1 blank + 1 action line
                ActionLayout::Vertical => self.actions.len() as u16 + 1, // 1 blank + N lines
            }
        } else {
            0
        };
        let content_chunks = if has_actions {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(action_height)])
                .split(inner)
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1)])
                .split(inner)
        };

        // Render body
        if !self.body.is_empty() {
            let body = Paragraph::new(self.body.clone())
                .style(self.style.body)
                .wrap(Wrap { trim: false });
            frame.render_widget(body, content_chunks[0]);
        }

        // Render actions
        if has_actions {
            let action_area = content_chunks[1];

            match self.action_layout {
                ActionLayout::Horizontal => {
                    let action_line_area = Rect {
                        y: action_area.y + 1, // skip blank line
                        height: 1,
                        ..action_area
                    };

                    let mut spans = Vec::new();
                    for (i, action) in self.actions.iter().enumerate() {
                        if i > 0 {
                            spans.push(Span::raw("  "));
                        }
                        let style = if i == self.focused_action {
                            action
                                .focused_style
                                .unwrap_or(self.style.focused_action)
                        } else {
                            action.style.unwrap_or(self.style.action)
                        };
                        let label = if let Some(key) = action.shortcut {
                            format!("[{}] {}", key, action.label)
                        } else {
                            action.label.clone()
                        };
                        if i == self.focused_action {
                            spans.push(Span::styled(format!("▸ {}", label), style));
                        } else {
                            spans.push(Span::styled(format!("  {}", label), style));
                        }
                    }

                    let actions_paragraph =
                        Paragraph::new(Line::from(spans)).alignment(Alignment::Center);
                    frame.render_widget(actions_paragraph, action_line_area);
                }
                ActionLayout::Vertical => {
                    for (i, action) in self.actions.iter().enumerate() {
                        let row_area = Rect {
                            y: action_area.y + 1 + i as u16, // skip blank line
                            height: 1,
                            ..action_area
                        };

                        let style = if i == self.focused_action {
                            action
                                .focused_style
                                .unwrap_or(self.style.focused_action)
                        } else {
                            action.style.unwrap_or(self.style.action)
                        };
                        let prefix = if i == self.focused_action {
                            "▸ "
                        } else {
                            "  "
                        };
                        let label = if let Some(key) = action.shortcut {
                            format!("{}[{}] {}", prefix, key, action.label)
                        } else {
                            format!("{}{}", prefix, action.label)
                        };

                        let action_paragraph = Paragraph::new(Span::styled(label, style));
                        frame.render_widget(action_paragraph, row_area);
                    }
                }
            }
        }
    }

    fn focused(&self) -> bool {
        true // Modals always capture input
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEvent, KeyEventKind, KeyEventState};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn builder_works() {
        let modal = Modal::new("Test")
            .body("Hello world")
            .action(Action::new("OK").with_shortcut('o'))
            .action(Action::new("Cancel").with_shortcut('c'));
        assert_eq!(modal.title(), "Test");
        assert_eq!(modal.actions().len(), 2);
        assert_eq!(modal.focused_action(), 0);
    }

    #[test]
    fn arrow_keys_navigate_actions() {
        let mut modal = Modal::new("Test")
            .action(Action::new("A"))
            .action(Action::new("B"))
            .action(Action::new("C"));
        assert_eq!(modal.focused_action(), 0);

        modal.update(Message::KeyPress(key(KeyCode::Right)));
        assert_eq!(modal.focused_action(), 1);

        modal.update(Message::KeyPress(key(KeyCode::Right)));
        assert_eq!(modal.focused_action(), 2);

        // Wraps around to first
        modal.update(Message::KeyPress(key(KeyCode::Right)));
        assert_eq!(modal.focused_action(), 0);

        // Wraps around to last
        modal.update(Message::KeyPress(key(KeyCode::Left)));
        assert_eq!(modal.focused_action(), 2);
    }

    #[test]
    fn enter_selects_focused() {
        let mut modal = Modal::new("Test")
            .action(Action::new("A"))
            .action(Action::new("B"));

        modal.update(Message::KeyPress(key(KeyCode::Right)));
        let cmd = modal.update(Message::KeyPress(key(KeyCode::Enter)));
        match cmd.into_message() {
            Some(Message::Select(1)) => {}
            other => panic!(
                "Expected Select(1), got {:?}",
                other.map(|m| format!("{:?}", m))
            ),
        }
    }

    #[test]
    fn esc_dismisses() {
        let mut modal = Modal::new("Test");
        let cmd = modal.update(Message::KeyPress(key(KeyCode::Esc)));
        match cmd.into_message() {
            Some(Message::Dismiss) => {}
            other => panic!(
                "Expected Dismiss, got {:?}",
                other.map(|m| format!("{:?}", m))
            ),
        }
    }

    #[test]
    fn shortcut_selects_action() {
        let mut modal = Modal::new("Test")
            .action(Action::new("Yes").with_shortcut('y'))
            .action(Action::new("No").with_shortcut('n'));

        let cmd = modal.update(Message::KeyPress(key(KeyCode::Char('n'))));
        match cmd.into_message() {
            Some(Message::Select(1)) => {}
            other => panic!(
                "Expected Select(1), got {:?}",
                other.map(|m| format!("{:?}", m))
            ),
        }
    }

    #[test]
    fn number_key_focuses_action() {
        let mut modal = Modal::new("Test")
            .action(Action::new("First"))
            .action(Action::new("Second"))
            .action(Action::new("Third"));

        // Number keys focus rather than select
        let cmd = modal.update(Message::KeyPress(key(KeyCode::Char('2'))));
        assert!(cmd.is_none()); // No message emitted
        assert_eq!(modal.focused_action(), 1); // '2' focuses index 1
    }

    #[test]
    fn number_key_out_of_range_is_noop() {
        let mut modal = Modal::new("Test").action(Action::new("Only"));

        let cmd = modal.update(Message::KeyPress(key(KeyCode::Char('5'))));
        assert!(cmd.is_none());
    }

    #[test]
    fn enter_on_no_actions_dismisses() {
        let mut modal = Modal::new("Test");
        let cmd = modal.update(Message::KeyPress(key(KeyCode::Enter)));
        match cmd.into_message() {
            Some(Message::Dismiss) => {}
            other => panic!(
                "Expected Dismiss, got {:?}",
                other.map(|m| format!("{:?}", m))
            ),
        }
    }

    #[test]
    fn with_size_clamps() {
        let modal = Modal::new("Test").with_size(150, 200);
        assert_eq!(modal.width_percent, 100);
        assert_eq!(modal.height_percent, 100);
    }

    #[test]
    fn default_layout_is_horizontal() {
        let modal = Modal::new("Test");
        assert_eq!(modal.action_layout, ActionLayout::Horizontal);
    }

    #[test]
    fn vertical_layout_navigation() {
        let mut modal = Modal::new("Test")
            .with_action_layout(ActionLayout::Vertical)
            .action(Action::new("A"))
            .action(Action::new("B"))
            .action(Action::new("C"));

        assert_eq!(modal.focused_action(), 0);

        modal.update(Message::KeyPress(key(KeyCode::Down)));
        assert_eq!(modal.focused_action(), 1);

        modal.update(Message::KeyPress(key(KeyCode::Down)));
        assert_eq!(modal.focused_action(), 2);

        // Wraps
        modal.update(Message::KeyPress(key(KeyCode::Down)));
        assert_eq!(modal.focused_action(), 0);

        // Up wraps to last
        modal.update(Message::KeyPress(key(KeyCode::Up)));
        assert_eq!(modal.focused_action(), 2);
    }

    #[test]
    fn per_action_style_fields() {
        let action = Action::new("OK")
            .with_style(Style::default().fg(Color::Gray))
            .with_focused_style(Style::default().fg(Color::Green));

        assert!(action.style.is_some());
        assert!(action.focused_style.is_some());
        assert_eq!(action.style.unwrap().fg, Some(Color::Gray));
        assert_eq!(action.focused_style.unwrap().fg, Some(Color::Green));
    }

    #[test]
    fn fixed_size_overrides_percentage() {
        let modal = Modal::new("Test").with_fixed_size(60, 20);
        assert_eq!(modal.fixed_width, Some(60));
        assert_eq!(modal.fixed_height, Some(20));
    }
}
