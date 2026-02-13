//! Stopwatch component that counts up from zero.

use boba_core::command::Command;
use boba_core::component::Component;
use boba_core::subscription::Subscription;
use boba_core::subscriptions::Every;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use std::time::Duration;

/// Messages for the stopwatch component.
#[derive(Debug, Clone)]
pub enum Message {
    /// Advance elapsed time by one interval tick.
    Tick,
    /// Start the stopwatch.
    Start,
    /// Stop (pause) the stopwatch.
    Stop,
    /// Reset elapsed time to zero and stop.
    Reset,
    /// Toggle between running and stopped.
    Toggle,
}

/// A counting-up timer (stopwatch) component.
///
/// Displays elapsed time in `MM:SS.T` format (minutes, seconds, tenths of a second).
pub struct Stopwatch {
    elapsed: Duration,
    running: bool,
    interval: Duration,
    style: Style,
    id: &'static str,
}

impl Stopwatch {
    /// Create a new stopwatch with the given subscription id.
    /// Defaults to a 100ms tick interval and stopped state.
    pub fn new(id: &'static str) -> Self {
        Self {
            elapsed: Duration::ZERO,
            running: false,
            interval: Duration::from_millis(100),
            style: Style::default(),
            id,
        }
    }

    /// Set the tick interval for time updates.
    pub fn with_interval(mut self, d: Duration) -> Self {
        self.interval = d;
        self
    }

    /// Set the display style.
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Get the total elapsed time.
    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// Whether the stopwatch is currently running.
    pub fn running(&self) -> bool {
        self.running
    }

    /// Start the stopwatch.
    pub fn start(&mut self) {
        self.running = true;
    }

    /// Stop (pause) the stopwatch.
    pub fn stop(&mut self) {
        self.running = false;
    }

    /// Reset elapsed time to zero and stop.
    pub fn reset(&mut self) {
        self.elapsed = Duration::ZERO;
        self.running = false;
    }

    /// Toggle between running and stopped.
    pub fn toggle(&mut self) {
        self.running = !self.running;
    }
}

/// Format a duration as `MM:SS.T` (minutes:seconds.tenths).
fn format_duration(d: Duration) -> String {
    let total_secs = d.as_secs();
    let minutes = total_secs / 60;
    let seconds = total_secs % 60;
    let tenths = d.subsec_millis() / 100;
    format!("{:02}:{:02}.{}", minutes, seconds, tenths)
}

impl Component for Stopwatch {
    type Message = Message;

    fn update(&mut self, msg: Message) -> Command<Message> {
        match msg {
            Message::Tick => {
                if self.running {
                    self.elapsed += self.interval;
                }
                Command::none()
            }
            Message::Start => {
                self.start();
                Command::none()
            }
            Message::Stop => {
                self.stop();
                Command::none()
            }
            Message::Reset => {
                self.reset();
                Command::none()
            }
            Message::Toggle => {
                self.toggle();
                Command::none()
            }
        }
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let text = format_duration(self.elapsed);
        let span = Span::styled(text, self.style);
        let paragraph = Paragraph::new(span);
        frame.render_widget(paragraph, area);
    }

    fn subscriptions(&self) -> Vec<Subscription<Message>> {
        if self.running {
            vec![
                boba_core::subscription::subscribe(Every::new(self.interval, self.id))
                    .map(|_: std::time::Instant| Message::Tick),
            ]
        } else {
            vec![]
        }
    }
}
