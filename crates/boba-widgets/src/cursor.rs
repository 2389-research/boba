//! Blinking cursor component for use in custom input widgets.

use boba_core::command::Command;
use boba_core::component::Component;
use boba_core::subscription::Subscription;
use boba_core::subscriptions::Every;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use std::time::Duration;

/// The cursor display mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorMode {
    /// Cursor is hidden.
    Hide,
    /// Cursor blinks on and off.
    Blink,
    /// Cursor is always visible (no blinking).
    Static,
}

/// Messages for the cursor component.
#[derive(Debug, Clone)]
pub enum Message {
    /// Toggle blink visibility on each tick.
    BlinkTick,
}

/// A standalone cursor component that can blink, remain static, or be hidden.
pub struct Cursor {
    mode: CursorMode,
    visible: bool,
    blink_speed: Duration,
    style: Style,
    cursor_char: char,
    focused: bool,
    id: &'static str,
}

impl Cursor {
    /// Create a new cursor with the given subscription id.
    /// Defaults to `CursorMode::Blink`, blink speed of 530ms, and a space character
    /// rendered with reversed style.
    pub fn new(id: &'static str) -> Self {
        Self {
            mode: CursorMode::Blink,
            visible: true,
            blink_speed: Duration::from_millis(530),
            style: Style::default().add_modifier(Modifier::REVERSED),
            cursor_char: ' ',
            focused: false,
            id,
        }
    }

    /// Set the cursor mode.
    pub fn with_mode(mut self, mode: CursorMode) -> Self {
        self.mode = mode;
        self
    }

    /// Set the blink speed (interval between blink toggles).
    pub fn with_blink_speed(mut self, d: Duration) -> Self {
        self.blink_speed = d;
        self
    }

    /// Set the cursor style.
    pub fn with_style(mut self, s: Style) -> Self {
        self.style = s;
        self
    }

    /// Set the character displayed by the cursor.
    pub fn with_cursor_char(mut self, c: char) -> Self {
        self.cursor_char = c;
        self
    }

    /// Give this cursor focus.
    pub fn focus(&mut self) {
        self.focused = true;
    }

    /// Remove focus from this cursor.
    pub fn blur(&mut self) {
        self.focused = false;
    }

    /// Whether the cursor is currently visible (accounts for blink state).
    pub fn is_visible(&self) -> bool {
        match self.mode {
            CursorMode::Hide => false,
            CursorMode::Static => true,
            CursorMode::Blink => self.visible,
        }
    }

    /// Get the current cursor mode.
    pub fn mode(&self) -> &CursorMode {
        &self.mode
    }

    /// Set the cursor mode at runtime.
    pub fn set_mode(&mut self, mode: CursorMode) {
        self.mode = mode;
        // Reset visibility when mode changes
        self.visible = true;
    }
}

impl Component for Cursor {
    type Message = Message;

    fn update(&mut self, msg: Message) -> Command<Message> {
        match msg {
            Message::BlinkTick => {
                if self.mode == CursorMode::Blink {
                    self.visible = !self.visible;
                }
                Command::none()
            }
        }
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let text = String::from(self.cursor_char);
        let span = if self.is_visible() && self.focused {
            Span::styled(text, self.style)
        } else {
            Span::raw(text)
        };

        let paragraph = Paragraph::new(span);
        frame.render_widget(paragraph, area);
    }

    fn subscriptions(&self) -> Vec<Subscription<Message>> {
        if self.mode == CursorMode::Blink && self.focused {
            vec![
                boba_core::subscription::subscribe(Every::new(self.blink_speed, self.id))
                    .map(|_: std::time::Instant| Message::BlinkTick),
            ]
        } else {
            vec![]
        }
    }

    fn focused(&self) -> bool {
        self.focused
    }
}
