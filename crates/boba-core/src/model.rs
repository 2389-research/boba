use crate::command::Command;
use crate::subscription::Subscription;
use ratatui::Frame;

/// The top-level application trait, following the [Elm Architecture].
///
/// Every boba application implements `Model`. The runtime drives a continuous
/// **init -> update -> view** cycle:
///
/// 1. [`init`](Model::init) creates the initial state and may return a
///    [`Command`] for early side effects (e.g. fetching data).
/// 2. [`view`](Model::view) renders the current state to a [`ratatui::Frame`].
/// 3. External events arrive as messages through [`Subscription`]s.
/// 4. [`update`](Model::update) processes each message, mutates state, and
///    optionally returns a [`Command`] for further work.
/// 5. Steps 2--4 repeat until the program exits.
///
/// `Model` is the equivalent of Bubble Tea's `tea.Model`, but renders to
/// ratatui's `Frame` instead of returning a string.
///
/// # Example
///
/// ```rust,ignore
/// use boba_core::{Model, Command};
/// use ratatui::Frame;
/// use ratatui::widgets::Paragraph;
///
/// struct Counter {
///     count: i32,
/// }
///
/// #[derive(Debug)]
/// enum Msg {
///     Increment,
///     Decrement,
/// }
///
/// impl Model for Counter {
///     type Message = Msg;
///     type Flags = ();
///
///     fn init(_flags: ()) -> (Self, Command<Msg>) {
///         (Counter { count: 0 }, Command::none())
///     }
///
///     fn update(&mut self, msg: Msg) -> Command<Msg> {
///         match msg {
///             Msg::Increment => self.count += 1,
///             Msg::Decrement => self.count -= 1,
///         }
///         Command::none()
///     }
///
///     fn view(&self, frame: &mut Frame) {
///         frame.render_widget(
///             Paragraph::new(format!("Count: {}", self.count)),
///             frame.area(),
///         );
///     }
/// }
/// ```
///
/// [Elm Architecture]: https://guide.elm-lang.org/architecture/
pub trait Model: Sized + Send + 'static {
    /// The application's message type.
    ///
    /// Every event that can affect the application state is represented as a
    /// variant of this type.  Messages arrive from [`Subscription`]s, from
    /// [`Command::message`], or from async work completed via
    /// [`Command::perform`].
    type Message: Send + 'static;

    /// Initialization data passed to [`Model::init`].
    ///
    /// Use `()` when no startup data is needed.  For applications that require
    /// configuration, define a struct carrying the relevant fields and pass it
    /// when constructing a [`Program`](crate::runtime::Program).
    type Flags: Send + 'static;

    /// Create the initial model state and an optional startup command.
    ///
    /// This is called once when the program starts.  Return a tuple of the
    /// initial model value and a [`Command`] for any work that should begin
    /// immediately (e.g. loading data from disk).  Use [`Command::none()`] if
    /// no startup side effects are needed.
    fn init(flags: Self::Flags) -> (Self, Command<Self::Message>);

    /// Process a message, mutate state, and return a command for side effects.
    ///
    /// This is the heart of the application logic.  Pattern-match on the
    /// incoming message, update `self` accordingly, and return a [`Command`]
    /// describing any side effects the runtime should perform.  After `update`
    /// returns, the runtime calls [`view`](Model::view) to re-render and
    /// [`subscriptions`](Model::subscriptions) to reconcile active
    /// subscriptions.
    fn update(&mut self, msg: Self::Message) -> Command<Self::Message>;

    /// Render the current state to a ratatui [`Frame`].
    ///
    /// This method should be a pure function of `&self` -- it reads the model
    /// state and draws widgets into the frame.  The runtime calls `view` after
    /// every update and on the initial render.
    fn view(&self, frame: &mut Frame);

    /// Declare active subscriptions.  Called after every update.
    ///
    /// Return a [`Vec`] of [`Subscription`]s that should be active given the
    /// current model state.  The runtime diffs the returned list against the
    /// previously active set: new subscriptions are started and removed ones
    /// are cancelled.
    ///
    /// The default implementation returns an empty list (no subscriptions).
    fn subscriptions(&self) -> Vec<Subscription<Self::Message>> {
        vec![]
    }
}
