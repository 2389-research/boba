//! Overlay positioning and rendering utilities.
//!
//! Provides functions for computing centered sub-rects and clearing overlay
//! areas, used by Modal, Help, and any custom overlay composition.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Block, Clear};
use ratatui::Frame;

/// Compute a centered sub-rect within `area` using percentage dimensions.
///
/// The percentages control the size of the inner rect relative to the outer area.
/// For example, `centered_rect(50, 40, area)` produces a rect that is 50% of the
/// width and 40% of the height, centered within `area`.
pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let v_margin = ((100u16.saturating_sub(percent_y)) / 2).max(1);
    let h_margin = ((100u16.saturating_sub(percent_x)) / 2).max(1);
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(v_margin),
            Constraint::Percentage(percent_y),
            Constraint::Percentage(v_margin),
        ])
        .split(area);
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(h_margin),
            Constraint::Percentage(percent_x),
            Constraint::Percentage(h_margin),
        ])
        .split(vertical[1]);
    horizontal[1]
}

/// Compute a centered sub-rect with fixed dimensions, clamped to `area`.
///
/// If `width` or `height` exceed the area dimensions, they are clamped.
pub fn centered_fixed(width: u16, height: u16, area: Rect) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    Rect::new(x, y, w, h)
}

/// Clear the overlay area and optionally render a block border.
///
/// Returns the inner area (after block padding, if any). This is the typical
/// pattern for overlay widgets: clear background, draw border, get inner area.
pub fn render_overlay(frame: &mut Frame, area: Rect, block: Option<&Block>) -> Rect {
    frame.render_widget(Clear, area);
    if let Some(block) = block {
        let inner = block.inner(area);
        frame.render_widget(block.clone(), area);
        inner
    } else {
        area
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centered_rect_50_50() {
        let area = Rect::new(0, 0, 100, 50);
        let result = centered_rect(50, 50, area);
        // Should be roughly centered — the exact values depend on Layout rounding
        assert!(result.x > 0);
        assert!(result.y > 0);
        assert!(result.width > 0 && result.width <= 50);
        assert!(result.height > 0 && result.height <= 25);
    }

    #[test]
    fn centered_fixed_basic() {
        let area = Rect::new(10, 5, 80, 40);
        let result = centered_fixed(40, 20, area);
        assert_eq!(result.width, 40);
        assert_eq!(result.height, 20);
        assert_eq!(result.x, 30); // 10 + (80-40)/2
        assert_eq!(result.y, 15); // 5 + (40-20)/2
    }

    #[test]
    fn centered_fixed_clamps_to_area() {
        let area = Rect::new(0, 0, 20, 10);
        let result = centered_fixed(100, 50, area);
        assert_eq!(result.width, 20);
        assert_eq!(result.height, 10);
        assert_eq!(result.x, 0);
        assert_eq!(result.y, 0);
    }

    #[test]
    fn centered_fixed_zero_area() {
        let area = Rect::new(0, 0, 0, 0);
        let result = centered_fixed(10, 10, area);
        assert_eq!(result.width, 0);
        assert_eq!(result.height, 0);
    }
}
