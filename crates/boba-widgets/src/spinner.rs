//! Animated loading spinner with multiple built-in frame sets.

use boba_core::command::Command;
use boba_core::component::Component;
use boba_core::subscription::Subscription;
use boba_core::subscriptions::Every;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use std::time::Duration;

/// Built-in spinner frame sets.
pub mod frames {
    /// Braille dot spinner cycling through ten positions.
    pub const DOTS: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    /// Classic ASCII line spinner: |, /, -, \.
    pub const LINE: &[&str] = &["|", "/", "-", "\\"];
    /// Compact braille dot spinner with six frames.
    pub const MINI_DOT: &[&str] = &["⠋", "⠙", "⠸", "⠴", "⠦", "⠇"];
    /// Braille pattern that appears to jump up and down.
    pub const JUMP: &[&str] = &["⢄", "⢂", "⢁", "⡁", "⡈", "⡐", "⡠"];
    /// Block characters that pulse between solid and transparent.
    pub const PULSE: &[&str] = &["█", "▓", "▒", "░", "▒", "▓"];
    /// Three-dot pattern with a moving filled dot.
    pub const POINTS: &[&str] = &["∙∙∙", "●∙∙", "∙●∙", "∙∙●"];
    /// Rotating globe emoji sequence.
    pub const GLOBE: &[&str] = &["🌍", "🌎", "🌏"];
    /// Moon phase emoji sequence cycling through all phases.
    pub const MOON: &[&str] = &["🌑", "🌒", "🌓", "🌔", "🌕", "🌖", "🌗", "🌘"];
    /// Meter-style bar that fills and empties.
    pub const METER: &[&str] = &["▱▱▱", "▰▱▱", "▰▰▱", "▰▰▰", "▰▰▱", "▰▱▱"];
    /// Growing ellipsis from empty to three dots.
    pub const ELLIPSIS: &[&str] = &["", ".", "..", "..."];
}

/// Messages for the spinner component.
#[derive(Debug, Clone)]
pub enum Message {
    /// Advance the spinner to its next frame.
    Tick,
}

/// An animated spinner component that cycles through a set of frames
/// at a configurable interval while spinning is active.
pub struct Spinner {
    frames: &'static [&'static str],
    frame_index: usize,
    title: String,
    style: Style,
    interval: Duration,
    spinning: bool,
    id: &'static str,
}

impl Spinner {
    /// Create a new spinner with the given subscription identifier.
    /// Defaults to the [`frames::DOTS`] frame set and a 100ms interval.
    pub fn new(id: &'static str) -> Self {
        Self {
            frames: frames::DOTS,
            frame_index: 0,
            title: String::new(),
            style: Style::default().fg(Color::Cyan),
            interval: Duration::from_millis(100),
            spinning: true,
            id,
        }
    }

    /// Set the frame set used by this spinner (e.g. [`frames::LINE`]).
    pub fn with_frames(mut self, frames: &'static [&'static str]) -> Self {
        self.frames = frames;
        self
    }

    /// Set the title text displayed after the spinner frame.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Set the text style applied to the spinner and title.
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Set the duration between frame advances.
    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    /// Start the spinner animation.
    pub fn start(&mut self) {
        self.spinning = true;
    }

    /// Stop the spinner animation.
    pub fn stop(&mut self) {
        self.spinning = false;
    }

    /// Return whether the spinner is currently animating.
    pub fn is_spinning(&self) -> bool {
        self.spinning
    }
}

impl Component for Spinner {
    type Message = Message;

    fn update(&mut self, msg: Message) -> Command<Message> {
        match msg {
            Message::Tick => {
                if self.spinning && !self.frames.is_empty() {
                    self.frame_index = (self.frame_index + 1) % self.frames.len();
                }
                Command::none()
            }
        }
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        if !self.spinning || area.width == 0 || area.height == 0 {
            return;
        }

        let spinner_frame = self.frames[self.frame_index];
        let text = if self.title.is_empty() {
            spinner_frame.to_string()
        } else {
            format!("{} {}", spinner_frame, self.title)
        };

        let paragraph = Paragraph::new(Span::styled(text, self.style));
        frame.render_widget(paragraph, area);
    }

    fn subscriptions(&self) -> Vec<Subscription<Message>> {
        if self.spinning {
            vec![
                boba_core::subscription::subscribe(Every::new(self.interval, self.id))
                    .map(|_: std::time::Instant| Message::Tick),
            ]
        } else {
            vec![]
        }
    }
}
