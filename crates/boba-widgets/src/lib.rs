//! Ready-made widgets for the **boba** TUI framework.
//!
//! Every widget in this crate implements [`boba_core::Component`], so it can be
//! embedded inside any [`boba_core::Model`] and composed freely within
//! [`ratatui`] layouts.
//!
//! # Widgets
//!
//! | Module | Description |
//! |--------|-------------|
//! | [`cursor`] | Blinking / styled cursor helper |
//! | [`filepicker`] | Interactive file and directory picker |
//! | [`help`] | Key-binding help overlay / bubble |
//! | [`list`] | Scrollable, filterable list |
//! | [`paginator`] | Page-dot indicator for multi-page views |
//! | [`progress`] | Determinate progress bar |
//! | [`select`] | Single-choice selection menu |
//! | [`spinner`] | Animated indeterminate spinner |
//! | [`stopwatch`] | Elapsed-time stopwatch |
//! | [`table`] | Row/column table with selection |
//! | [`tabs`] | Horizontal tab bar |
//! | [`text_area`] | Multi-line text editor |
//! | [`text_input`] | Single-line text input field |
//! | [`timer`] | Countdown timer |
//! | [`viewport`] | Scrollable content viewport |
//!
//! # Utilities
//!
//! | Module | Description |
//! |--------|-------------|
//! | [`focus`] | [`FocusGroup`](focus::FocusGroup) for managing focus across multiple components |
//! | [`key`] | Key-binding helpers and constants |
//! | [`runeutil`] | Unicode-aware string width and truncation utilities |

pub mod cursor;
pub mod filepicker;
pub mod focus;
pub mod help;
pub mod key;
pub mod list;
pub mod paginator;
pub mod progress;
pub mod runeutil;
pub mod select;
pub mod spinner;
pub mod stopwatch;
pub mod table;
pub mod tabs;
pub mod text_area;
pub mod text_input;
pub mod timer;
pub mod viewport;
