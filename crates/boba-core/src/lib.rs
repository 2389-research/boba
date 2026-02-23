//! Core runtime for the **boba** TUI framework.
//!
//! `boba-core` provides the traits, types, and runtime that power every boba
//! application.  The design follows the [Elm Architecture]: your program is
//! expressed as a pure **init -> update -> view** cycle, with side effects
//! pushed to the edges through [`Command`]s and [`Subscription`]s.
//!
//! # Key types
//!
//! | Type | Purpose |
//! |------|---------|
//! | [`Model`] | Top-level application trait (init / update / view) |
//! | [`Component`] | Reusable sub-model that renders into a [`ratatui::layout::Rect`] |
//! | [`Command`] | Describes a side effect to be executed by the runtime |
//! | [`Subscription`] | Long-lived event source (terminal events, timers, etc.) |
//! | [`Program`] | Wires a [`Model`] to a real terminal and drives the event loop |
//! | [`TestProgram`](testing::TestProgram) | Headless harness for unit-testing a [`Model`] without a terminal |
//!
//! # Architecture
//!
//! 1. **init** -- The [`Model::init`] function creates the initial state and
//!    may return a [`Command`] to kick off early work.
//! 2. **view** -- The runtime calls [`Model::view`] to render the current state
//!    to a [`ratatui::Frame`].
//! 3. **event** -- External events (key presses, mouse, timers) arrive via
//!    [`Subscription`]s and are mapped into the model's `Message` type.
//! 4. **update** -- [`Model::update`] receives a message, mutates state, and
//!    optionally returns a [`Command`] for further side effects.
//! 5. **repeat** -- Steps 2-4 repeat until the program exits.
//!
//! # Quick example
//!
//! ```ignore
//! use boba_core::{Model, Command};
//! use ratatui::Frame;
//! use ratatui::widgets::Paragraph;
//!
//! struct Counter { count: i32 }
//!
//! enum Msg { Increment, Decrement }
//!
//! impl Model for Counter {
//!     type Message = Msg;
//!     type Flags = ();
//!
//!     fn init(_flags: ()) -> (Self, Command<Msg>) {
//!         (Counter { count: 0 }, Command::none())
//!     }
//!
//!     fn update(&mut self, msg: Msg) -> Command<Msg> {
//!         match msg {
//!             Msg::Increment => self.count += 1,
//!             Msg::Decrement => self.count -= 1,
//!         }
//!         Command::none()
//!     }
//!
//!     fn view(&self, frame: &mut Frame) {
//!         frame.render_widget(
//!             Paragraph::new(format!("Count: {}", self.count)),
//!             frame.area(),
//!         );
//!     }
//! }
//! ```
//!
//! [Elm Architecture]: https://guide.elm-lang.org/architecture/

pub mod command;
pub mod component;
pub mod event;
pub mod input_history;
pub mod input_layer;
pub mod key_sequence;
pub mod model;
pub mod quit;
pub mod runtime;
pub mod subscription;
pub mod subscriptions;
pub mod testing;

pub use command::{Command, CursorStyle, ExecCommand, MouseMode, TerminalCommand};
pub use component::Component;
pub use event::TerminalEvent;
pub use input_history::InputHistory;
pub use input_layer::{InputLayer, LayeredModel};
pub use key_sequence::KeySequenceTracker;
pub use model::Model;
pub use quit::QuitConfirmation;
pub use runtime::{
    log_to_file, OutputTarget, Program, ProgramError, ProgramHandle, ProgramOptions,
};
pub use subscription::{subscribe, Subscription, SubscriptionId, SubscriptionSource};
pub use subscriptions::{terminal_events, After, Every};

/// Run a boba application with default options.
pub async fn run<M: Model>(flags: M::Flags) -> Result<M, ProgramError> {
    Program::<M>::new(flags)?.run().await
}

/// Run with custom options.
pub async fn run_with<M: Model>(
    flags: M::Flags,
    options: ProgramOptions,
) -> Result<M, ProgramError> {
    Program::<M>::with_options(flags, options)?.run().await
}
