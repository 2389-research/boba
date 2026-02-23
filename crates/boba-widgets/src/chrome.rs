//! Convenience helpers for common widget chrome patterns.

use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders};

/// Create a bordered block with focus-dependent border color.
///
/// Uses cyan when focused, dark gray when unfocused. Suitable as a
/// default chrome for any widget.
pub fn focus_block(title: &str, focused: bool) -> Block<'_> {
    let color = if focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };
    Block::new()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(color))
}
