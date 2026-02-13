use crossterm::event::{KeyEvent, MouseEvent};

/// Terminal events produced by the runtime's event loop.
///
/// `TerminalEvent` is delivered to your application through the
/// [`terminal_events`](crate::subscriptions::terminal_events) subscription.
/// You provide a mapping function that converts each `TerminalEvent` into your
/// application's `Message` type.
///
/// Each variant wraps the corresponding
/// [`crossterm::event::Event`] payload, so you can pattern-match on key codes,
/// modifiers, mouse buttons, and so on using the full crossterm API.
///
/// # Example
///
/// ```rust,ignore
/// use boba_core::{subscriptions::terminal_events, TerminalEvent, Subscription};
///
/// fn subscriptions() -> Vec<Subscription<Msg>> {
///     vec![terminal_events(|ev| match ev {
///         TerminalEvent::Key(k) => Msg::Key(k),
///         _ => Msg::Noop,
///     })]
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalEvent {
    /// A keyboard event.
    Key(KeyEvent),
    /// A mouse event.
    Mouse(MouseEvent),
    /// Terminal resized to (columns, rows).
    Resize(u16, u16),
    /// Terminal window gained focus.
    FocusGained,
    /// Terminal window lost focus.
    FocusLost,
    /// Bracketed paste content.
    Paste(String),
}

impl From<crossterm::event::Event> for TerminalEvent {
    fn from(event: crossterm::event::Event) -> Self {
        match event {
            crossterm::event::Event::Key(k) => TerminalEvent::Key(k),
            crossterm::event::Event::Mouse(m) => TerminalEvent::Mouse(m),
            crossterm::event::Event::Resize(w, h) => TerminalEvent::Resize(w, h),
            crossterm::event::Event::FocusGained => TerminalEvent::FocusGained,
            crossterm::event::Event::FocusLost => TerminalEvent::FocusLost,
            crossterm::event::Event::Paste(s) => TerminalEvent::Paste(s),
        }
    }
}
