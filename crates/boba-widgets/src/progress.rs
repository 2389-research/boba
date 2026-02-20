//! Animated progress bar with spring physics, gradient colors, and customizable fill characters.

use boba_core::command::Command;
use boba_core::component::Component;
use boba_core::subscription::Subscription;
use boba_core::subscriptions::Every;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Gauge};
use ratatui::Frame;
use std::time::Duration;

/// Messages for the progress component.
#[derive(Debug, Clone)]
pub enum Message {
    /// Advance the spring physics simulation by one frame.
    Tick,
    /// Set a new target progress value (0.0 to 1.0).
    SetTarget(f64),
}

/// An animated progress bar with spring physics.
pub struct Progress {
    target: f64,
    current: f64,
    velocity: f64,
    label: String,
    style: ProgressStyle,
    animating: bool,
    id: &'static str,
    // Spring physics constants
    stiffness: f64,
    damping: f64,
    show_percentage: bool,
    filled_char: char,
    empty_char: char,
    label_format: Option<Box<dyn Fn(f64) -> String + Send>>,
    gradient: Option<(Color, Color)>,
    fill_color: Option<Color>,
    empty_color: Option<Color>,
    block: Option<Block<'static>>,
}

/// Visual style configuration for the [`Progress`] component.
#[derive(Debug, Clone)]
pub struct ProgressStyle {
    /// Style applied to the filled portion of the bar.
    pub filled: Style,
    /// Style applied to the unfilled portion of the bar.
    pub unfilled: Style,
    /// Style applied to the label text.
    pub label: Style,
}

impl Default for ProgressStyle {
    fn default() -> Self {
        Self {
            filled: Style::default().fg(Color::Cyan),
            unfilled: Style::default().fg(Color::DarkGray),
            label: Style::default(),
        }
    }
}

impl Progress {
    /// Create a new progress bar with the given subscription identifier.
    /// Starts at 0% progress with default spring physics parameters.
    pub fn new(id: &'static str) -> Self {
        Self {
            target: 0.0,
            current: 0.0,
            velocity: 0.0,
            label: String::new(),
            style: ProgressStyle::default(),
            animating: false,
            id,
            stiffness: 180.0,
            damping: 12.0,
            show_percentage: true,
            filled_char: '\u{2588}', // █
            empty_char: '\u{2591}',  // ░
            label_format: None,
            gradient: None,
            fill_color: None,
            empty_color: None,
            block: None,
        }
    }

    /// Set a static label displayed alongside the progress bar.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Set the visual style for this progress bar.
    pub fn with_style(mut self, style: ProgressStyle) -> Self {
        self.style = style;
        self
    }

    /// Set the block (border/title container) for the progress bar.
    pub fn with_block(mut self, block: Block<'static>) -> Self {
        self.block = Some(block);
        self
    }

    /// Configure spring physics parameters.
    pub fn with_spring_options(mut self, stiffness: f64, damping: f64) -> Self {
        self.stiffness = stiffness;
        self.damping = damping;
        self
    }

    /// Toggle whether the percentage is displayed. Default is `true`.
    pub fn with_percentage(mut self, show: bool) -> Self {
        self.show_percentage = show;
        self
    }

    /// Set custom characters for the filled and empty portions of the bar.
    pub fn with_chars(mut self, filled: char, empty: char) -> Self {
        self.filled_char = filled;
        self.empty_char = empty;
        self
    }

    /// Set a custom label format function. Receives the current ratio (0.0..=1.0)
    /// and should return the full label string.
    pub fn with_label_format(mut self, f: impl Fn(f64) -> String + Send + 'static) -> Self {
        self.label_format = Some(Box::new(f));
        self
    }

    /// Set a gradient for the filled portion. The color will interpolate from
    /// `from` to `to` based on the current progress percentage.
    pub fn with_gradient(mut self, from: Color, to: Color) -> Self {
        self.gradient = Some((from, to));
        self
    }

    /// Set a custom foreground color for the filled portion of the bar.
    pub fn with_fill_color(mut self, color: Color) -> Self {
        self.fill_color = Some(color);
        self
    }

    /// Set a custom foreground color for the empty portion of the bar.
    pub fn with_empty_color(mut self, color: Color) -> Self {
        self.empty_color = Some(color);
        self
    }

    /// Increment the target progress by a relative amount (e.g. 0.1 for 10%).
    pub fn incr_percent(&mut self, amount: f64) {
        if amount.is_finite() {
            self.set_progress(self.target + amount);
        }
    }

    /// Decrement the target progress by a relative amount (e.g. 0.1 for 10%).
    pub fn decr_percent(&mut self, amount: f64) {
        if amount.is_finite() {
            self.set_progress(self.target - amount);
        }
    }

    /// Set the target progress value (0.0 to 1.0). The bar will animate
    /// toward this value using spring physics.
    pub fn set_progress(&mut self, value: f64) {
        self.target = value.clamp(0.0, 1.0);
        self.animating = (self.target - self.current).abs() > 0.001;
    }

    /// Set the progress value immediately without spring animation.
    pub fn set_progress_immediate(&mut self, value: f64) {
        self.target = value.clamp(0.0, 1.0);
        self.current = self.target;
        self.velocity = 0.0;
        self.animating = false;
    }

    /// Return the current (animated) progress value.
    pub fn progress(&self) -> f64 {
        self.current
    }

    /// Return the target progress value that the bar is animating toward.
    pub fn target(&self) -> f64 {
        self.target
    }
}

/// Interpolate between two colors based on parameter `t` in `0.0..=1.0`.
///
/// When both colors are `Color::Rgb`, each channel is linearly interpolated.
/// Otherwise, returns `from` when `t < 0.5` and `to` when `t >= 0.5`.
pub fn interpolate_color(from: Color, to: Color, t: f64) -> Color {
    let t = t.clamp(0.0, 1.0);
    match (from, to) {
        (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => {
            let r = (r1 as f64 + (r2 as f64 - r1 as f64) * t).round() as u8;
            let g = (g1 as f64 + (g2 as f64 - g1 as f64) * t).round() as u8;
            let b = (b1 as f64 + (b2 as f64 - b1 as f64) * t).round() as u8;
            Color::Rgb(r, g, b)
        }
        _ => {
            if t < 0.5 {
                from
            } else {
                to
            }
        }
    }
}

impl Component for Progress {
    type Message = Message;

    fn update(&mut self, msg: Message) -> Command<Message> {
        match msg {
            Message::Tick => {
                if !self.animating {
                    return Command::none();
                }

                let dt = 1.0 / 60.0; // Assume 60fps tick rate
                let displacement = self.current - self.target;
                let spring_force = -self.stiffness * displacement;
                let damping_force = -self.damping * self.velocity;
                let acceleration = spring_force + damping_force;

                self.velocity += acceleration * dt;
                self.current += self.velocity * dt;
                self.current = self.current.clamp(0.0, 1.0);

                // Stop animating when close enough
                if (self.current - self.target).abs() < 0.001 && self.velocity.abs() < 0.001 {
                    self.current = self.target;
                    self.velocity = 0.0;
                    self.animating = false;
                }

                Command::none()
            }
            Message::SetTarget(value) => {
                self.set_progress(value);
                Command::none()
            }
        }
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        let label = if let Some(ref fmt) = self.label_format {
            fmt(self.current)
        } else if self.show_percentage {
            if self.label.is_empty() {
                format!("{:.0}%", self.current * 100.0)
            } else {
                format!("{} {:.0}%", self.label, self.current * 100.0)
            }
        } else if self.label.is_empty() {
            String::new()
        } else {
            self.label.clone()
        };

        // Determine the filled portion style:
        // Gradient takes precedence over fill_color, which takes precedence over default.
        let filled_style = if let Some((from, to)) = self.gradient {
            let t = self.current.clamp(0.0, 1.0);
            let color = interpolate_color(from, to, t);
            Style::default().fg(color)
        } else if let Some(color) = self.fill_color {
            Style::default().fg(color)
        } else {
            self.style.filled
        };

        // Determine the empty portion style (used as the block's base style).
        let empty_style = if let Some(color) = self.empty_color {
            Style::default().fg(color)
        } else {
            self.style.unfilled
        };

        let mut gauge = Gauge::default()
            .gauge_style(filled_style)
            .ratio(self.current.clamp(0.0, 1.0))
            .label(label);
        if let Some(ref block) = self.block {
            gauge = gauge.block(block.clone());
            gauge = gauge.style(empty_style);
        } else {
            gauge = gauge.style(empty_style);
        }

        frame.render_widget(gauge, area);
    }

    fn subscriptions(&self) -> Vec<Subscription<Message>> {
        if self.animating {
            vec![
                boba_core::subscription::subscribe(Every::new(Duration::from_millis(16), self.id))
                    .map(|_: std::time::Instant| Message::Tick),
            ]
        } else {
            vec![]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    #[test]
    fn test_with_gradient_sets_field() {
        let from = Color::Rgb(255, 0, 0);
        let to = Color::Rgb(0, 0, 255);
        let p = Progress::new("test").with_gradient(from, to);
        assert_eq!(p.gradient, Some((from, to)));
    }

    #[test]
    fn test_with_fill_color_sets_field() {
        let color = Color::Rgb(0, 255, 0);
        let p = Progress::new("test").with_fill_color(color);
        assert_eq!(p.fill_color, Some(color));
    }

    #[test]
    fn test_with_empty_color_sets_field() {
        let color = Color::Rgb(128, 128, 128);
        let p = Progress::new("test").with_empty_color(color);
        assert_eq!(p.empty_color, Some(color));
    }

    #[test]
    fn test_interpolate_color_rgb_at_zero() {
        let from = Color::Rgb(0, 0, 0);
        let to = Color::Rgb(255, 255, 255);
        let result = interpolate_color(from, to, 0.0);
        assert_eq!(result, Color::Rgb(0, 0, 0));
    }

    #[test]
    fn test_interpolate_color_rgb_at_one() {
        let from = Color::Rgb(0, 0, 0);
        let to = Color::Rgb(255, 255, 255);
        let result = interpolate_color(from, to, 1.0);
        assert_eq!(result, Color::Rgb(255, 255, 255));
    }

    #[test]
    fn test_interpolate_color_rgb_at_half() {
        let from = Color::Rgb(0, 100, 200);
        let to = Color::Rgb(100, 200, 50);
        let result = interpolate_color(from, to, 0.5);
        assert_eq!(result, Color::Rgb(50, 150, 125));
    }

    #[test]
    fn test_interpolate_color_non_rgb_below_half() {
        let from = Color::Red;
        let to = Color::Blue;
        let result = interpolate_color(from, to, 0.3);
        assert_eq!(result, Color::Red);
    }

    #[test]
    fn test_interpolate_color_non_rgb_above_half() {
        let from = Color::Red;
        let to = Color::Blue;
        let result = interpolate_color(from, to, 0.7);
        assert_eq!(result, Color::Blue);
    }

    #[test]
    fn test_interpolate_color_mixed_rgb_and_indexed() {
        let from = Color::Rgb(100, 100, 100);
        let to = Color::Yellow;
        // Mixed types fall back to the non-RGB path
        let result = interpolate_color(from, to, 0.2);
        assert_eq!(result, from);
        let result = interpolate_color(from, to, 0.8);
        assert_eq!(result, to);
    }
}
