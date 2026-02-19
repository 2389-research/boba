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
//! | [`dropdown`] | Standalone dropdown overlay for selectable items |
//! | [`filepicker`] | Interactive file and directory picker |
//! | [`help`] | Key-binding help formatting utilities |
//! | [`list`] | Scrollable, filterable list |
//! | [`modal`] | Generic modal/dialog overlay with action buttons |
//! | [`paginator`] | Page-dot indicator for multi-page views |
//! | [`progress`] | Determinate progress bar |
//! | [`search`] | Inline search bar with match navigation |
//! | [`select`] | Single-choice selection menu (composes [`dropdown`]) |
//! | [`spinner`] | Animated indeterminate spinner |
//! | [`status_bar`] | Three-section status line (left / center / right) |
//! | [`stopwatch`] | Elapsed-time stopwatch |
//! | [`table`] | Row/column table with selection |
//! | [`tabs`] | Horizontal tab bar |
//! | [`text_area`] | Multi-line text editor |
//! | [`text_input`] | Single-line text input field |
//! | [`timer`] | Countdown timer |
//! | [`viewport`] | Scrollable content viewport |
//!
//! # Feature-Gated Widgets
//!
//! | Module | Feature | Description |
//! |--------|---------|-------------|
//! | [`code_block`] | `syntax-highlighting` | Syntax-highlighted code block (syntect) |
//! | [`markdown`] | `markdown` | CommonMark renderer with highlighting |
//!
//! # Utilities
//!
//! | Module | Description |
//! |--------|-------------|
//! | [`focus`] | [`FocusGroup`](focus::FocusGroup) for managing focus across multiple components |
//! | [`key`] | Key-binding helpers and constants |
//! | [`overlay`] | Overlay positioning and rendering utilities |
//! | [`runeutil`] | Unicode-aware string width and truncation utilities |
//! | [`selection`] | [`SelectionState`](selection::SelectionState) for shared list navigation |
//! | [`text_edit`] | [`TextEditState`](text_edit::TextEditState) for shared single-line text editing |

pub mod chrome;
#[cfg(feature = "syntax-highlighting")]
pub mod code_block;
pub mod cursor;
pub mod dropdown;
pub mod filepicker;
pub mod focus;
pub mod help;
pub mod key;
pub mod list;
#[cfg(feature = "markdown")]
pub mod markdown;
pub mod modal;
pub mod overlay;
pub mod paginator;
pub mod progress;
pub mod runeutil;
pub mod search;
pub mod select;
pub mod selection;
pub mod spinner;
pub mod status_bar;
pub mod stopwatch;
pub mod table;
pub mod tabs;
pub mod text_area;
pub mod text_edit;
pub mod text_input;
pub mod timer;
pub mod viewport;
