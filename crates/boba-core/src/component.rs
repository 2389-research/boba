use crate::command::Command;
use crate::subscription::Subscription;
use ratatui::{layout::Rect, Frame};

/// A reusable sub-model that renders into a given [`Rect`] area.
///
/// `Component` is nearly identical to [`Model`](crate::Model) but with one key
/// difference: its [`view`](Component::view) method receives an `area: Rect`
/// parameter, making components composable within layouts.  A parent model (or
/// another component) decides *where* each child renders by passing it a
/// sub-region of the frame.
///
/// # Composition pattern
///
/// To embed a `Component` inside a [`Model`](crate::Model), wrap the
/// component's message type in a variant of the parent message and use
/// [`Command::map`] to translate commands:
///
/// ```rust,ignore
/// use boba_core::{Model, Component, Command};
/// use ratatui::Frame;
/// use ratatui::layout::{Layout, Direction, Constraint, Rect};
///
/// // -- child component ---------------------------------------------------
///
/// struct SearchBar { query: String }
///
/// #[derive(Debug)]
/// enum SearchMsg { Input(char), Clear }
///
/// impl Component for SearchBar {
///     type Message = SearchMsg;
///
///     fn update(&mut self, msg: SearchMsg) -> Command<SearchMsg> {
///         match msg {
///             SearchMsg::Input(ch) => self.query.push(ch),
///             SearchMsg::Clear     => self.query.clear(),
///         }
///         Command::none()
///     }
///
///     fn view(&self, frame: &mut Frame, area: Rect) {
///         // ... render into `area` ...
///     }
/// }
///
/// // -- parent model ------------------------------------------------------
///
/// struct App { search: SearchBar }
///
/// #[derive(Debug)]
/// enum AppMsg { Search(SearchMsg) }
///
/// impl Model for App {
///     type Message = AppMsg;
///     type Flags = ();
///
///     fn init(_: ()) -> (Self, Command<AppMsg>) {
///         (App { search: SearchBar { query: String::new() } }, Command::none())
///     }
///
///     fn update(&mut self, msg: AppMsg) -> Command<AppMsg> {
///         match msg {
///             AppMsg::Search(m) => self.search.update(m).map(AppMsg::Search),
///         }
///     }
///
///     fn view(&self, frame: &mut Frame) {
///         let chunks = Layout::default()
///             .direction(Direction::Vertical)
///             .constraints([Constraint::Length(3), Constraint::Min(0)])
///             .split(frame.area());
///         self.search.view(frame, chunks[0]);
///     }
/// }
/// ```
pub trait Component: Send + 'static {
    /// The component's internal message type.
    ///
    /// Parent models typically wrap this in one of their own message variants
    /// so that events can be routed to the correct child.
    type Message: Send + 'static;

    /// Process a message, mutate state, and return a [`Command`] for side effects.
    ///
    /// Works exactly like [`Model::update`](crate::Model::update).  The
    /// returned command uses the component's own `Message` type; the parent
    /// should call [`.map()`](Command::map) to lift it into the parent message
    /// type.
    fn update(&mut self, msg: Self::Message) -> Command<Self::Message>;

    /// Render into a specific `area` of the [`Frame`].
    ///
    /// Unlike [`Model::view`](crate::Model::view), this method receives an
    /// `area: Rect` so the parent can control where the component is drawn.
    /// Implementations should confine all rendering to the given rectangle.
    fn view(&self, frame: &mut Frame, area: Rect);

    /// Declare active subscriptions for this component.
    ///
    /// The parent is responsible for collecting child subscriptions (calling
    /// this method) and including them in its own
    /// [`Model::subscriptions`](crate::Model::subscriptions) return value,
    /// mapping messages appropriately.
    ///
    /// The default implementation returns an empty list (no subscriptions).
    fn subscriptions(&self) -> Vec<Subscription<Self::Message>> {
        vec![]
    }

    /// Whether this component currently has focus.
    ///
    /// This is a hint for input routing.  A parent can query `focused()` to
    /// decide which child should receive keyboard events.  The default
    /// implementation returns `false`.
    fn focused(&self) -> bool {
        false
    }
}
