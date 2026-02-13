//! Scrollable content area with support for plain text, styled lines,
//! ANSI escape sequences, mouse wheel scrolling, and horizontal scroll.

use std::cell::Cell;

use boba_core::command::Command;
use boba_core::component::Component;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};
use ratatui::Frame;

use crate::runeutil;

/// Messages for the viewport component.
#[derive(Debug, Clone)]
pub enum Message {
    /// A key press event forwarded to the viewport for handling.
    KeyPress(KeyEvent),
    /// Scroll up by the given number of lines.
    ScrollUp(u16),
    /// Scroll down by the given number of lines.
    ScrollDown(u16),
    /// Scroll left by the given number of columns.
    ScrollLeft(u16),
    /// Scroll right by the given number of columns.
    ScrollRight(u16),
    /// Scroll to the very top of the content.
    ScrollToTop,
    /// Scroll to the very bottom of the content.
    ScrollToBottom,
    /// Alias for `ScrollToTop`.
    GotoTop,
    /// Alias for `ScrollToBottom`.
    GotoBottom,
    /// Mouse wheel event. `up` is true for scroll-up, false for scroll-down.
    MouseWheel { up: bool },
    /// Scroll up by one full visible page.
    ViewUp,
    /// Scroll down by one full visible page.
    ViewDown,
    /// Scroll up by half a visible page.
    HalfViewUp,
    /// Scroll down by half a visible page.
    HalfViewDown,
}

/// A scrollable content area with vertical and horizontal scrolling.
///
/// Supports plain text, pre-styled lines, and content with ANSI escape
/// sequences. A vertical scrollbar is rendered automatically when the
/// content exceeds the visible area.
///
/// # Example
///
/// ```ignore
/// let mut vp = Viewport::new("Hello, world!\nLine two\nLine three");
/// vp.focus();
/// // Or load ANSI-colored output:
/// // vp.set_ansi_content(ansi_string);
/// ```
pub struct Viewport {
    content: String,
    /// Pre-styled content lines (takes precedence over `content` when `Some`).
    styled_content: Option<Vec<Line<'static>>>,
    offset: u16,
    h_offset: u16,
    focus: bool,
    style: ViewportStyle,
    mouse_wheel_enabled: bool,
    mouse_wheel_delta: u16,
    /// Updated during each `view()` call via interior mutability.
    visible_height: Cell<u16>,
}

/// Style configuration for the viewport.
#[derive(Debug, Clone)]
pub struct ViewportStyle {
    /// Border style when the viewport does not have focus.
    pub border: Style,
    /// Border style when the viewport has focus.
    pub focused_border: Style,
    /// Style applied to the vertical scrollbar.
    pub scrollbar: Style,
}

impl Default for ViewportStyle {
    fn default() -> Self {
        Self {
            border: Style::default().fg(Color::DarkGray),
            focused_border: Style::default().fg(Color::Cyan),
            scrollbar: Style::default(),
        }
    }
}

impl Viewport {
    /// Create a viewport with the given plain text content.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            styled_content: None,
            offset: 0,
            h_offset: 0,
            focus: false,
            style: ViewportStyle::default(),
            mouse_wheel_enabled: true,
            mouse_wheel_delta: 3,
            visible_height: Cell::new(24),
        }
    }

    /// Replace the content with new plain text, resetting scroll offsets.
    pub fn set_content(&mut self, content: impl Into<String>) {
        self.content = content.into();
        self.styled_content = None;
        self.offset = 0;
        self.h_offset = 0;
    }

    /// Set pre-styled content lines directly.
    ///
    /// Clears the plain string content and resets scroll offsets. The styled
    /// content takes precedence over `content` when rendering.
    pub fn set_styled_content(&mut self, lines: Vec<Line<'static>>) {
        self.styled_content = Some(lines);
        self.content.clear();
        self.offset = 0;
        self.h_offset = 0;
    }

    /// Set content from a string containing ANSI escape sequences.
    ///
    /// Parses the ANSI sequences into styled `Line` objects and stores them
    /// as styled content. Clears the plain string content and resets scroll
    /// offsets.
    pub fn set_ansi_content(&mut self, content: impl Into<String>) {
        let s: String = content.into();
        let lines = runeutil::parse_ansi(&s);
        self.styled_content = Some(lines);
        self.content.clear();
        self.offset = 0;
        self.h_offset = 0;
    }

    /// Set the viewport style configuration.
    pub fn with_style(mut self, style: ViewportStyle) -> Self {
        self.style = style;
        self
    }

    /// Enable or disable mouse wheel scrolling.
    pub fn with_mouse_wheel(mut self, enabled: bool) -> Self {
        self.mouse_wheel_enabled = enabled;
        self
    }

    /// Set the number of lines scrolled per mouse wheel tick.
    pub fn with_mouse_wheel_delta(mut self, delta: u16) -> Self {
        self.mouse_wheel_delta = delta;
        self
    }

    /// Give focus to the viewport, enabling keyboard scrolling.
    pub fn focus(&mut self) {
        self.focus = true;
    }

    /// Remove focus from the viewport.
    pub fn blur(&mut self) {
        self.focus = false;
    }

    // ---- Public scroll info methods ----

    /// Current vertical scroll offset.
    pub fn y_offset(&self) -> u16 {
        self.offset
    }

    /// Set the vertical scroll position (will be clamped during render).
    pub fn set_y_offset(&mut self, offset: u16) {
        self.offset = offset;
    }

    /// Whether the viewport is scrolled to the very top.
    pub fn at_top(&self) -> bool {
        self.offset == 0
    }

    /// Whether the viewport is scrolled to the very bottom.
    pub fn at_bottom(&self) -> bool {
        let vh = self.visible_height.get();
        self.offset >= self.max_offset(vh)
    }

    /// Current scroll position as a fraction between 0.0 and 1.0.
    /// Returns 1.0 if the content fits entirely within the viewport.
    pub fn scroll_percent(&self) -> f64 {
        let vh = self.visible_height.get();
        let max = self.max_offset(vh);
        if max == 0 {
            return 1.0;
        }
        (self.offset.min(max) as f64) / (max as f64)
    }

    /// Total number of lines in the content.
    pub fn total_line_count(&self) -> usize {
        if let Some(ref lines) = self.styled_content {
            lines.len()
        } else {
            self.content.lines().count()
        }
    }

    /// Number of lines currently visible (the lesser of total lines and visible height).
    pub fn visible_line_count(&self) -> usize {
        let total = self.total_line_count();
        let vh = self.visible_height.get() as usize;
        total.min(vh)
    }

    /// Whether the content overflows the viewport (more lines than visible height).
    pub fn past_bottom(&self) -> bool {
        self.total_line_count() > self.visible_height.get() as usize
    }

    // ---- Goto convenience methods ----

    /// Scroll to the very top.
    pub fn goto_top(&mut self) {
        self.offset = 0;
    }

    /// Scroll to the very bottom.
    pub fn goto_bottom(&mut self) {
        self.offset = u16::MAX; // Will be clamped in view
    }

    // ---- Internal helpers ----

    fn total_lines(&self) -> u16 {
        let count = if let Some(ref lines) = self.styled_content {
            lines.len()
        } else {
            self.content.lines().count()
        };
        if count > u16::MAX as usize { u16::MAX } else { count as u16 }
    }

    fn max_offset(&self, visible_height: u16) -> u16 {
        self.total_lines().saturating_sub(visible_height)
    }
}

impl Component for Viewport {
    type Message = Message;

    fn update(&mut self, msg: Message) -> Command<Message> {
        match msg {
            Message::KeyPress(key) if self.focus => match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.offset = self.offset.saturating_sub(1);
                    Command::none()
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.offset = self.offset.saturating_add(1);
                    Command::none()
                }
                KeyCode::Left | KeyCode::Char('h') => {
                    self.h_offset = self.h_offset.saturating_sub(1);
                    Command::none()
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    self.h_offset = self.h_offset.saturating_add(1);
                    Command::none()
                }
                KeyCode::PageUp => {
                    let vh = self.visible_height.get();
                    self.offset = self.offset.saturating_sub(vh);
                    Command::none()
                }
                KeyCode::PageDown => {
                    let vh = self.visible_height.get();
                    self.offset = self.offset.saturating_add(vh);
                    Command::none()
                }
                KeyCode::Home | KeyCode::Char('g') => {
                    self.offset = 0;
                    Command::none()
                }
                KeyCode::End | KeyCode::Char('G') => {
                    self.offset = u16::MAX; // Will be clamped in view
                    Command::none()
                }
                _ => Command::none(),
            },
            Message::ScrollUp(n) => {
                self.offset = self.offset.saturating_sub(n);
                Command::none()
            }
            Message::ScrollDown(n) => {
                self.offset = self.offset.saturating_add(n);
                Command::none()
            }
            Message::ScrollLeft(n) => {
                self.h_offset = self.h_offset.saturating_sub(n);
                Command::none()
            }
            Message::ScrollRight(n) => {
                self.h_offset = self.h_offset.saturating_add(n);
                Command::none()
            }
            Message::ScrollToTop | Message::GotoTop => {
                self.offset = 0;
                Command::none()
            }
            Message::ScrollToBottom | Message::GotoBottom => {
                self.offset = u16::MAX;
                Command::none()
            }
            Message::MouseWheel { up } => {
                if self.mouse_wheel_enabled {
                    if up {
                        self.offset = self.offset.saturating_sub(self.mouse_wheel_delta);
                    } else {
                        self.offset = self.offset.saturating_add(self.mouse_wheel_delta);
                    }
                }
                Command::none()
            }
            Message::ViewUp => {
                let vh = self.visible_height.get();
                self.offset = self.offset.saturating_sub(vh);
                Command::none()
            }
            Message::ViewDown => {
                let vh = self.visible_height.get();
                self.offset = self.offset.saturating_add(vh);
                Command::none()
            }
            Message::HalfViewUp => {
                let half = self.visible_height.get() / 2;
                self.offset = self.offset.saturating_sub(half);
                Command::none()
            }
            Message::HalfViewDown => {
                let half = self.visible_height.get() / 2;
                self.offset = self.offset.saturating_add(half);
                Command::none()
            }
            _ => Command::none(),
        }
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        let border_style = if self.focus {
            self.style.focused_border
        } else {
            self.style.border
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style);

        let inner = block.inner(area);

        // Update visible_height via interior mutability.
        self.visible_height.set(inner.height);

        let max = self.max_offset(inner.height);
        let offset = self.offset.min(max);

        let text = if let Some(ref lines) = self.styled_content {
            Text::from(lines.clone())
        } else {
            Text::raw(&self.content)
        };

        let paragraph = Paragraph::new(text)
            .block(block)
            .scroll((offset, self.h_offset));

        frame.render_widget(paragraph, area);

        // Render scrollbar if content exceeds visible area
        if self.total_lines() > inner.height {
            let mut scrollbar_state = ScrollbarState::new(max as usize).position(offset as usize);
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
            frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
        }
    }

    fn focused(&self) -> bool {
        self.focus
    }
}
