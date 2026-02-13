//! Built-in subscription sources.
//!
//! This module re-exports the standard subscriptions provided by boba-core:
//!
//! - **Terminal events** ([`terminal_events`], [`TerminalEvents`]) -- keyboard,
//!   mouse, resize, focus, and paste events from the terminal.
//! - **Timers** ([`Every`], [`After`]) -- repeating and one-shot timer
//!   subscriptions.

mod terminal;
mod timer;

pub use terminal::*;
pub use timer::*;
