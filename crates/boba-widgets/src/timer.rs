//! Countdown timer component that counts down from a specified duration.

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

/// Messages for the timer component.
#[derive(Debug, Clone)]
pub enum Message {
    /// Advance the countdown by one interval tick.
    Tick,
    /// Start the timer.
    Start,
    /// Stop (pause) the timer.
    Stop,
    /// Reset the timer to its original timeout.
    Reset,
    /// Toggle between running and stopped.
    Toggle,
    /// Emitted when the timer reaches zero.
    Timeout,
}

/// A counting-down timer component.
///
/// Displays remaining time in `MM:SS.T` format (minutes, seconds, tenths of a second).
/// When the timer reaches zero, it emits `Message::Timeout` and stops.
pub struct Timer {
    timeout: Duration,
    remaining: Duration,
    running: bool,
    interval: Duration,
    timed_out: bool,
    style: Style,
    id: &'static str,
}

impl Timer {
    /// Create a new timer with the given subscription id and timeout duration.
    /// Defaults to a 100ms tick interval and stopped state.
    pub fn new(id: &'static str, timeout: Duration) -> Self {
        Self {
            timeout,
            remaining: timeout,
            running: false,
            interval: Duration::from_millis(100),
            timed_out: false,
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

    /// Get the remaining time.
    pub fn remaining(&self) -> Duration {
        self.remaining
    }

    /// Whether the timer is currently running.
    pub fn running(&self) -> bool {
        self.running
    }

    /// Whether the timer has reached zero.
    pub fn timed_out(&self) -> bool {
        self.timed_out
    }

    /// Start the timer.
    pub fn start(&mut self) {
        if !self.timed_out {
            self.running = true;
        }
    }

    /// Stop (pause) the timer.
    pub fn stop(&mut self) {
        self.running = false;
    }

    /// Reset the timer to its original timeout duration and stop.
    pub fn reset(&mut self) {
        self.remaining = self.timeout;
        self.running = false;
        self.timed_out = false;
    }

    /// Toggle between running and stopped.
    pub fn toggle(&mut self) {
        if self.running {
            self.stop();
        } else {
            self.start();
        }
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

impl Component for Timer {
    type Message = Message;

    fn update(&mut self, msg: Message) -> Command<Message> {
        match msg {
            Message::Tick => {
                if self.running && !self.timed_out {
                    if self.remaining <= self.interval {
                        self.remaining = Duration::ZERO;
                        self.running = false;
                        self.timed_out = true;
                        return Command::message(Message::Timeout);
                    } else {
                        self.remaining -= self.interval;
                    }
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
            Message::Timeout => {
                // This message is emitted by Tick; no additional action needed
                // when received externally.
                Command::none()
            }
        }
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let text = format_duration(self.remaining);
        let span = Span::styled(text, self.style);
        let paragraph = Paragraph::new(span);
        frame.render_widget(paragraph, area);
    }

    fn subscriptions(&self) -> Vec<Subscription<Message>> {
        if self.running && !self.timed_out {
            vec![
                boba_core::subscription::subscribe(Every::new(self.interval, self.id))
                    .map(|_: std::time::Instant| Message::Tick),
            ]
        } else {
            vec![]
        }
    }
}
