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

/// A button in the modal dialog.
#[derive(Debug, Clone)]
pub struct Action {
    /// Display label for the button.
    pub label: String,
    /// Optional shortcut key (e.g., 'y' for Yes). Shown after the label.
    pub shortcut: Option<char>,
}

impl Action {
    /// Create a new action with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            shortcut: None,
        }
    }

    /// Set a shortcut key for this action.
    pub fn with_shortcut(mut self, key: char) -> Self {
        self.shortcut = Some(key);
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

    /// Get the index of the currently focused action.
    pub fn focused_action(&self) -> usize {
        self.focused_action
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

impl Component for Modal {
    type Message = Message;

    fn update(&mut self, msg: Message) -> Command<Message> {
        match msg {
            Message::KeyPress(key) => {
                match (key.code, key.modifiers) {
                    (KeyCode::Esc, _) => Command::message(Message::Dismiss),
                    (KeyCode::Left, _) | (KeyCode::Char('h'), KeyModifiers::NONE) => {
                        if self.focused_action > 0 {
                            self.focused_action -= 1;
                        }
                        Command::none()
                    }
                    (KeyCode::Right, _)
                    | (KeyCode::Char('l'), KeyModifiers::NONE)
                    | (KeyCode::Tab, _) => {
                        if !self.actions.is_empty() && self.focused_action < self.actions.len() - 1
                        {
                            self.focused_action += 1;
                        }
                        Command::none()
                    }
                    (KeyCode::Up, _) | (KeyCode::Char('k'), KeyModifiers::NONE) => {
                        if self.focused_action > 0 {
                            self.focused_action -= 1;
                        }
                        Command::none()
                    }
                    (KeyCode::Down, _) | (KeyCode::Char('j'), KeyModifiers::NONE) => {
                        if !self.actions.is_empty() && self.focused_action < self.actions.len() - 1
                        {
                            self.focused_action += 1;
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
                    // Number keys for quick selection (1-indexed)
                    (KeyCode::Char(c), KeyModifiers::NONE) if c.is_ascii_digit() && c != '0' => {
                        let idx = (c as u8 - b'1') as usize;
                        if idx < self.actions.len() {
                            Command::message(Message::Select(idx))
                        } else {
                            Command::none()
                        }
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

        // Layout: body takes remaining space, actions get 1 line at bottom
        let has_actions = !self.actions.is_empty();
        let content_chunks = if has_actions {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(1),
                    Constraint::Length(2), // 1 blank + 1 action line
                ])
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
                    self.style.focused_action
                } else {
                    self.style.action
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

            let actions_paragraph = Paragraph::new(Line::from(spans)).alignment(Alignment::Center);
            frame.render_widget(actions_paragraph, action_line_area);
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

        // Can't go past last
        modal.update(Message::KeyPress(key(KeyCode::Right)));
        assert_eq!(modal.focused_action(), 2);

        modal.update(Message::KeyPress(key(KeyCode::Left)));
        assert_eq!(modal.focused_action(), 1);
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
    fn number_key_selects_action() {
        let mut modal = Modal::new("Test")
            .action(Action::new("First"))
            .action(Action::new("Second"))
            .action(Action::new("Third"));

        let cmd = modal.update(Message::KeyPress(key(KeyCode::Char('2'))));
        match cmd.into_message() {
            Some(Message::Select(1)) => {} // '2' maps to index 1
            other => panic!(
                "Expected Select(1), got {:?}",
                other.map(|m| format!("{:?}", m))
            ),
        }
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
}
