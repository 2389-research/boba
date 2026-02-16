//! Chat message list with streaming, auto-scroll, and role headers.
//!
//! Renders a scrollable list of chat messages with role-based styling,
//! separator lines, streaming indicators, and automatic scrolling that
//! follows new content.  The parent feeds messages (including partial
//! streaming content) and the widget handles the display.
//!
//! When the `markdown` feature is enabled, message content is rendered
//! through the markdown parser with syntax highlighting.
//!
//! # Example
//!
//! ```ignore
//! use boba_widgets::chat::{Chat, ChatMessage, Role};
//!
//! let mut chat = Chat::new();
//! chat.push_message(ChatMessage::new(Role::User, "Hello!"));
//! chat.push_message(ChatMessage::new(Role::Assistant, "Hi there!"));
//! ```

use std::cell::Cell;

use boba_core::command::Command;
use boba_core::component::Component;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

/// The role of a chat message sender.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
    System,
    Tool,
}

/// A single chat message.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    /// The sender's role.
    pub role: Role,
    /// The message content (plain text or markdown).
    pub content: String,
    /// Whether this message is currently being streamed.
    pub is_streaming: bool,
    /// Optional label override (e.g. tool name instead of "Tool").
    pub label: Option<String>,
}

impl ChatMessage {
    /// Create a new message.
    pub fn new(role: Role, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
            is_streaming: false,
            label: None,
        }
    }

    /// Create a streaming placeholder message.
    pub fn streaming(role: Role) -> Self {
        Self {
            role,
            content: String::new(),
            is_streaming: true,
            label: None,
        }
    }

    /// Set a custom label for this message.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

/// Style configuration for chat messages.
#[derive(Debug, Clone)]
pub struct ChatStyle {
    /// Style for User role header.
    pub user_header: Style,
    /// Style for Assistant role header.
    pub assistant_header: Style,
    /// Style for System role header.
    pub system_header: Style,
    /// Style for Tool role header.
    pub tool_header: Style,
    /// Style for the separator line between messages.
    pub separator: Style,
    /// Style for the streaming indicator.
    pub streaming_indicator: Style,
    /// Border style when focused.
    pub focused_border: Style,
    /// Border style when unfocused.
    pub unfocused_border: Style,
    /// Border style when scroll is locked (user scrolled up).
    pub locked_border: Style,
    /// User header label (default: "You").
    pub user_label: String,
    /// Assistant header label (default: "Assistant").
    pub assistant_label: String,
    /// System header label (default: "System").
    pub system_label: String,
    /// Tool header label (default: "Tool").
    pub tool_label: String,
}

impl Default for ChatStyle {
    fn default() -> Self {
        Self {
            user_header: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            assistant_header: Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
            system_header: Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            tool_header: Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
            separator: Style::default().fg(Color::DarkGray),
            streaming_indicator: Style::default().fg(Color::Yellow),
            focused_border: Style::default().fg(Color::Cyan),
            unfocused_border: Style::default().fg(Color::DarkGray),
            locked_border: Style::default().fg(Color::Yellow),
            user_label: "You".to_string(),
            assistant_label: "Assistant".to_string(),
            system_label: "System".to_string(),
            tool_label: "Tool".to_string(),
        }
    }
}

/// Messages for the chat component.
#[derive(Debug, Clone)]
pub enum Message {
    /// A key press event.
    KeyPress(KeyEvent),
    /// Scroll to the bottom.
    ScrollToBottom,
}

/// Chat message list component.
pub struct Chat {
    messages: Vec<ChatMessage>,
    style: ChatStyle,
    /// Lines from the bottom (0 = at bottom).
    scroll_offset: u16,
    /// Maximum scroll distance (computed during render via interior mutability).
    max_scroll: Cell<u16>,
    /// Whether the user has manually scrolled up (disables auto-scroll).
    scroll_locked: bool,
    focused: bool,
    /// Spinner frame for streaming indicator.
    spinner_frame: usize,
    /// Separator string.
    separator: String,
    /// Whether to render content as markdown (when feature available).
    render_markdown: bool,
    /// Cached rendered lines (invalidated on message changes).
    rendered_lines: Vec<Line<'static>>,
    /// Number of messages when lines were last rendered.
    rendered_message_count: usize,
    /// Content hash for cache invalidation.
    rendered_content_hash: u64,

    #[cfg(feature = "markdown")]
    markdown_renderer: crate::markdown::Markdown,
}

impl Default for Chat {
    fn default() -> Self {
        Self::new()
    }
}

impl Chat {
    /// Create a new empty chat.
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            style: ChatStyle::default(),
            scroll_offset: 0,
            max_scroll: Cell::new(0),
            scroll_locked: false,
            focused: false,
            spinner_frame: 0,
            separator: "\u{2500}".repeat(5),
            render_markdown: false,
            rendered_lines: Vec::new(),
            rendered_message_count: 0,
            rendered_content_hash: 0,
            #[cfg(feature = "markdown")]
            markdown_renderer: crate::markdown::Markdown::new(),
        }
    }

    /// Set the chat style.
    pub fn with_style(mut self, style: ChatStyle) -> Self {
        self.style = style;
        self
    }

    /// Enable markdown rendering (requires `markdown` feature).
    pub fn with_markdown(mut self, enabled: bool) -> Self {
        self.render_markdown = enabled;
        self
    }

    /// Set the markdown renderer (requires `markdown` feature).
    #[cfg(feature = "markdown")]
    pub fn with_markdown_renderer(mut self, renderer: crate::markdown::Markdown) -> Self {
        self.markdown_renderer = renderer;
        self
    }

    /// Set focus state.
    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    /// Push a new message to the chat.
    pub fn push_message(&mut self, message: ChatMessage) {
        self.messages.push(message);
        if !self.scroll_locked {
            self.scroll_offset = 0;
        }
        self.invalidate_cache();
    }

    /// Update the last message's content (for streaming).
    pub fn update_last_content(&mut self, content: &str) {
        if let Some(msg) = self.messages.last_mut() {
            msg.content = content.to_string();
            if !self.scroll_locked {
                self.scroll_offset = 0;
            }
            self.invalidate_cache();
        }
    }

    /// Mark the last message as no longer streaming.
    pub fn finish_streaming(&mut self) {
        if let Some(msg) = self.messages.last_mut() {
            msg.is_streaming = false;
            self.invalidate_cache();
        }
    }

    /// Clear all messages.
    pub fn clear(&mut self) {
        self.messages.clear();
        self.scroll_offset = 0;
        self.scroll_locked = false;
        self.invalidate_cache();
    }

    /// Get the messages.
    pub fn messages(&self) -> &[ChatMessage] {
        &self.messages
    }

    /// Whether auto-scroll is locked (user scrolled up).
    pub fn is_scroll_locked(&self) -> bool {
        self.scroll_locked
    }

    /// Get scroll offset (lines from bottom).
    pub fn scroll_offset(&self) -> u16 {
        self.scroll_offset
    }

    /// Advance the spinner frame (call on tick).
    pub fn tick_spinner(&mut self) {
        self.spinner_frame = (self.spinner_frame + 1) % SPINNER_FRAMES.len();
        // Only invalidate if we have a streaming message
        if self.messages.last().is_some_and(|m| m.is_streaming) {
            self.invalidate_cache();
        }
    }

    fn invalidate_cache(&mut self) {
        self.rendered_content_hash = 0;
    }

    fn content_hash(&self) -> u64 {
        let mut hash: u64 = self.messages.len() as u64;
        for msg in &self.messages {
            hash = hash.wrapping_mul(31).wrapping_add(msg.content.len() as u64);
            hash = hash.wrapping_mul(31).wrapping_add(msg.is_streaming as u64);
        }
        hash = hash
            .wrapping_mul(31)
            .wrapping_add(self.spinner_frame as u64);
        hash
    }

    fn render_lines(&self) -> Vec<Line<'static>> {
        let mut lines = Vec::new();

        for (i, msg) in self.messages.iter().enumerate() {
            // Separator between messages
            if i > 0 {
                lines.push(Line::from(Span::styled(
                    self.separator.clone(),
                    self.style.separator,
                )));
            }

            // Role header
            let (label, style) = match msg.role {
                Role::User => (&self.style.user_label, self.style.user_header),
                Role::Assistant => (&self.style.assistant_label, self.style.assistant_header),
                Role::System => (&self.style.system_label, self.style.system_header),
                Role::Tool => (&self.style.tool_label, self.style.tool_header),
            };
            let display_label = msg.label.as_deref().unwrap_or(label);
            lines.push(Line::from(Span::styled(
                format!("{}: ", display_label),
                style,
            )));

            // Message content
            if msg.content.is_empty() && msg.is_streaming {
                // Streaming placeholder
                lines.push(Line::from(Span::styled(
                    format!("  {} ...", SPINNER_FRAMES[self.spinner_frame]),
                    self.style.streaming_indicator,
                )));
            } else {
                #[cfg(feature = "markdown")]
                {
                    if self.render_markdown {
                        let md_lines = self.markdown_renderer.parse(&msg.content);
                        lines.extend(md_lines);
                    } else {
                        for text_line in msg.content.lines() {
                            lines.push(Line::from(format!("  {}", text_line)));
                        }
                    }
                }
                #[cfg(not(feature = "markdown"))]
                {
                    for text_line in msg.content.lines() {
                        lines.push(Line::from(format!("  {}", text_line)));
                    }
                }

                // Streaming indicator after content
                if msg.is_streaming {
                    lines.push(Line::from(Span::styled(
                        format!("  {} ...", SPINNER_FRAMES[self.spinner_frame]),
                        self.style.streaming_indicator,
                    )));
                }
            }
        }

        lines
    }
}

const SPINNER_FRAMES: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

impl Component for Chat {
    type Message = Message;

    fn update(&mut self, msg: Message) -> Command<Message> {
        match msg {
            Message::KeyPress(key) => {
                match (key.code, key.modifiers) {
                    // Scroll up
                    (KeyCode::Up, _) | (KeyCode::Char('k'), KeyModifiers::NONE) => {
                        self.scroll_offset = self
                            .scroll_offset
                            .saturating_add(1)
                            .min(self.max_scroll.get());
                        if self.scroll_offset > 0 {
                            self.scroll_locked = true;
                        }
                        Command::none()
                    }
                    // Scroll down
                    (KeyCode::Down, _) | (KeyCode::Char('j'), KeyModifiers::NONE) => {
                        self.scroll_offset = self.scroll_offset.saturating_sub(1);
                        if self.scroll_offset == 0 {
                            self.scroll_locked = false;
                        }
                        Command::none()
                    }
                    // Page up
                    (KeyCode::PageUp, _) | (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                        self.scroll_offset = self
                            .scroll_offset
                            .saturating_add(20)
                            .min(self.max_scroll.get());
                        if self.scroll_offset > 0 {
                            self.scroll_locked = true;
                        }
                        Command::none()
                    }
                    // Page down
                    (KeyCode::PageDown, _) | (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                        self.scroll_offset = self.scroll_offset.saturating_sub(20);
                        if self.scroll_offset == 0 {
                            self.scroll_locked = false;
                        }
                        Command::none()
                    }
                    // Go to top
                    (KeyCode::Home, _) => {
                        self.scroll_offset = self.max_scroll.get();
                        self.scroll_locked = true;
                        Command::none()
                    }
                    // Go to bottom (unlock)
                    (KeyCode::End, _) | (KeyCode::Char('G'), KeyModifiers::SHIFT) => {
                        self.scroll_offset = 0;
                        self.scroll_locked = false;
                        Command::none()
                    }
                    // Esc unlocks scroll
                    (KeyCode::Esc, _) if self.scroll_locked => {
                        self.scroll_offset = 0;
                        self.scroll_locked = false;
                        Command::message(Message::ScrollToBottom)
                    }
                    _ => Command::none(),
                }
            }
            Message::ScrollToBottom => {
                self.scroll_offset = 0;
                self.scroll_locked = false;
                Command::none()
            }
        }
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        if area.height < 3 || area.width < 5 {
            return;
        }

        // Build rendered lines (using cache when possible)
        let current_hash = self.content_hash();
        let lines = if current_hash == self.rendered_content_hash
            && self.rendered_message_count == self.messages.len()
            && !self.rendered_lines.is_empty()
        {
            &self.rendered_lines
        } else {
            // We can't mutate self in view(), so always render fresh
            &self.render_lines()
        };

        let content_height = lines.len() as u16;

        // Build title with scroll info
        let border_style = if self.focused {
            if self.scroll_locked {
                self.style.locked_border
            } else {
                self.style.focused_border
            }
        } else {
            self.style.unfocused_border
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style);

        let inner = block.inner(area);
        let visible_height = inner.height;
        let max_scroll = content_height.saturating_sub(visible_height);

        // Compute actual ratatui scroll (from top)
        // Update max_scroll via Cell so update() can use it for capping
        self.max_scroll.set(max_scroll);

        let clamped_offset = self.scroll_offset.min(max_scroll);
        let actual_scroll = max_scroll.saturating_sub(clamped_offset);

        let paragraph = Paragraph::new(lines.clone())
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((actual_scroll, 0));

        frame.render_widget(paragraph, area);
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
    fn new_chat_is_empty() {
        let chat = Chat::new();
        assert!(chat.messages().is_empty());
        assert!(!chat.is_scroll_locked());
        assert_eq!(chat.scroll_offset(), 0);
    }

    #[test]
    fn push_message_adds_to_list() {
        let mut chat = Chat::new();
        chat.push_message(ChatMessage::new(Role::User, "Hello"));
        chat.push_message(ChatMessage::new(Role::Assistant, "Hi!"));
        assert_eq!(chat.messages().len(), 2);
        assert_eq!(chat.messages()[0].role, Role::User);
        assert_eq!(chat.messages()[1].content, "Hi!");
    }

    #[test]
    fn streaming_workflow() {
        let mut chat = Chat::new();
        chat.push_message(ChatMessage::streaming(Role::Assistant));
        assert!(chat.messages().last().unwrap().is_streaming);
        assert!(chat.messages().last().unwrap().content.is_empty());

        chat.update_last_content("Hello ");
        assert_eq!(chat.messages().last().unwrap().content, "Hello ");

        chat.update_last_content("Hello world");
        assert_eq!(chat.messages().last().unwrap().content, "Hello world");

        chat.finish_streaming();
        assert!(!chat.messages().last().unwrap().is_streaming);
    }

    #[test]
    fn scroll_up_locks() {
        let mut chat = Chat::new();
        chat.max_scroll.set(50); // Simulate content taller than viewport
        chat.update(Message::KeyPress(key(KeyCode::Up)));
        assert!(chat.is_scroll_locked());
        assert_eq!(chat.scroll_offset(), 1);
    }

    #[test]
    fn scroll_down_to_bottom_unlocks() {
        let mut chat = Chat::new();
        chat.max_scroll.set(50);
        chat.scroll_offset = 1;
        chat.scroll_locked = true;

        chat.update(Message::KeyPress(key(KeyCode::Down)));
        assert_eq!(chat.scroll_offset(), 0);
        assert!(!chat.is_scroll_locked());
    }

    #[test]
    fn esc_scrolls_to_bottom_and_unlocks() {
        let mut chat = Chat::new();
        chat.max_scroll.set(50);
        chat.scroll_offset = 10;
        chat.scroll_locked = true;

        let cmd = chat.update(Message::KeyPress(key(KeyCode::Esc)));
        assert_eq!(chat.scroll_offset(), 0);
        assert!(!chat.is_scroll_locked());
        assert!(matches!(cmd.into_message(), Some(Message::ScrollToBottom)));
    }

    #[test]
    fn auto_scroll_follows_new_messages() {
        let mut chat = Chat::new();
        // Not locked — pushing messages keeps offset at 0
        chat.push_message(ChatMessage::new(Role::User, "Hello"));
        assert_eq!(chat.scroll_offset(), 0);

        // Lock scroll, then push — offset stays where user left it
        chat.scroll_offset = 5;
        chat.scroll_locked = true;
        chat.push_message(ChatMessage::new(Role::Assistant, "World"));
        assert_eq!(chat.scroll_offset(), 5); // Didn't reset
    }

    #[test]
    fn clear_resets_everything() {
        let mut chat = Chat::new();
        chat.push_message(ChatMessage::new(Role::User, "Hello"));
        chat.scroll_offset = 10;
        chat.scroll_locked = true;

        chat.clear();
        assert!(chat.messages().is_empty());
        assert_eq!(chat.scroll_offset(), 0);
        assert!(!chat.is_scroll_locked());
    }

    #[test]
    fn render_lines_produces_output() {
        let mut chat = Chat::new();
        chat.push_message(ChatMessage::new(Role::User, "Hello"));
        chat.push_message(ChatMessage::new(Role::Assistant, "World"));
        let lines = chat.render_lines();
        // Should have: header + content for each msg + separator between
        assert!(lines.len() >= 5); // 2 headers + 2 content + 1 separator
    }

    #[test]
    fn custom_label() {
        let msg = ChatMessage::new(Role::Tool, "result").with_label("bash");
        assert_eq!(msg.label.as_deref(), Some("bash"));
    }

    #[test]
    fn spinner_tick() {
        let mut chat = Chat::new();
        assert_eq!(chat.spinner_frame, 0);
        chat.tick_spinner();
        assert_eq!(chat.spinner_frame, 1);
    }
}
