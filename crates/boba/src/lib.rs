//! **boba** -- A Bubble Tea-inspired TUI framework for [`ratatui`].
//!
//! This is the umbrella crate that re-exports everything you need to build a
//! boba application from a single dependency:
//!
//! ```toml
//! [dependencies]
//! boba = "0.1"
//! ```
//!
//! # Re-exports
//!
//! * All public items from [`boba_core`] are available at the crate root
//!   ([`Model`], [`Component`], [`Command`], [`Subscription`], [`Program`],
//!   [`run`], [`run_with`], etc.).
//! * The [`widgets`] module re-exports everything from [`boba_widgets`]
//!   (text inputs, lists, tables, spinners, and more).
//! * [`ratatui`], [`crossterm`], and [`tokio`] are re-exported so downstream
//!   crates do not need to depend on them directly.
//!
//! # Quick start
//!
//! ```ignore
//! use boba::{Model, Command};
//! use ratatui::Frame;
//! use ratatui::widgets::Paragraph;
//!
//! struct Hello;
//! enum Msg {}
//!
//! impl Model for Hello {
//!     type Message = Msg;
//!     type Flags = ();
//!
//!     fn init(_: ()) -> (Self, Command<Msg>) {
//!         (Hello, Command::none())
//!     }
//!     fn update(&mut self, msg: Msg) -> Command<Msg> {
//!         match msg {}
//!     }
//!     fn view(&self, frame: &mut Frame) {
//!         frame.render_widget(Paragraph::new("Hello, boba!"), frame.area());
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     boba::run::<Hello>(()).await.unwrap();
//! }
//! ```

pub use boba_core::*;
pub mod widgets {
    pub use boba_widgets::*;
}

// Re-export dependencies for use in examples and downstream crates
pub use crossterm;
pub use ratatui;
pub use tokio;
