// ABOUTME: Trait for post-render buffer processing hooks.
// ABOUTME: Implementations run after view() and before the buffer is flushed to the terminal.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

/// A hook that runs after the model's `view()` renders to the buffer,
/// but before the buffer is diffed and flushed to the terminal.
pub trait PostRender: Send {
    fn after_view(&self, buf: &mut Buffer, area: Rect);
}
