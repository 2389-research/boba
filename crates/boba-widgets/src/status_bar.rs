//! A three-section status line (left / center / right) rendered as a single
//! row. Unlike most boba widgets this is a **stateless** ratatui `Widget`,
//! not a `Component`, because it has no internal state or message handling.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::Frame;

/// A three-section status bar that fills one row.
///
/// Renders left-aligned, centered, and right-aligned content sections.
/// Sections are optional; omitting one lets the others expand.
///
/// # Example
///
/// ```ignore
/// use boba_widgets::status_bar::StatusBar;
/// use ratatui::style::{Style, Color};
///
/// let bar = StatusBar::new()
///     .left("main")
///     .center("Jeff v0.4")
///     .right("Tokens: 1234")
///     .style(Style::default().bg(Color::DarkGray));
/// ```
pub struct StatusBar<'a> {
    left: Option<Line<'a>>,
    center: Option<Line<'a>>,
    right: Option<Line<'a>>,
    style: Style,
}

impl<'a> StatusBar<'a> {
    /// Create an empty status bar.
    pub fn new() -> Self {
        Self {
            left: None,
            center: None,
            right: None,
            style: Style::default(),
        }
    }

    /// Set the left-aligned content.
    pub fn left(mut self, content: impl Into<Line<'a>>) -> Self {
        self.left = Some(content.into());
        self
    }

    /// Set the center-aligned content.
    pub fn center(mut self, content: impl Into<Line<'a>>) -> Self {
        self.center = Some(content.into());
        self
    }

    /// Set the right-aligned content.
    pub fn right(mut self, content: impl Into<Line<'a>>) -> Self {
        self.right = Some(content.into());
        self
    }

    /// Set the base style (background color, etc.) for the entire bar.
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Render the status bar into the given frame and area.
    ///
    /// This is a convenience method for use inside a `Component::view()`.
    /// For direct `Widget` rendering, use `frame.render_widget(bar, area)`.
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(34),
                Constraint::Percentage(33),
            ])
            .split(area);

        // Fill the whole area with the background style first.
        let bg = ratatui::widgets::Block::default().style(self.style);
        frame.render_widget(bg, area);

        if let Some(ref line) = self.left {
            let styled = line.clone().patch_style(self.style);
            let p = ratatui::widgets::Paragraph::new(styled);
            frame.render_widget(p, chunks[0]);
        }

        if let Some(ref line) = self.center {
            let styled = line.clone().patch_style(self.style);
            let p = ratatui::widgets::Paragraph::new(styled)
                .alignment(ratatui::layout::Alignment::Center);
            frame.render_widget(p, chunks[1]);
        }

        if let Some(ref line) = self.right {
            let styled = line.clone().patch_style(self.style);
            let p = ratatui::widgets::Paragraph::new(styled)
                .alignment(ratatui::layout::Alignment::Right);
            frame.render_widget(p, chunks[2]);
        }
    }
}

impl<'a> Default for StatusBar<'a> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    #[test]
    fn default_is_empty() {
        let bar = StatusBar::new();
        assert!(bar.left.is_none());
        assert!(bar.center.is_none());
        assert!(bar.right.is_none());
    }

    #[test]
    fn builder_sets_sections() {
        let bar = StatusBar::new()
            .left("left")
            .center("center")
            .right("right");
        assert!(bar.left.is_some());
        assert!(bar.center.is_some());
        assert!(bar.right.is_some());
    }

    #[test]
    fn builder_sets_style() {
        let s = Style::default().bg(Color::Red);
        let bar = StatusBar::new().style(s);
        assert_eq!(bar.style, s);
    }
}
