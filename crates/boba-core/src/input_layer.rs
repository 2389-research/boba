//! Priority-based input routing for layered UIs.
//!
//! When a TUI has modals, overlays, search bars, and focused widgets all
//! competing for keyboard input, the [`InputLayer`] enum provides a
//! declarative way to express which layer should handle events.
//!
//! The [`LayeredModel`] trait extends [`Model`] with an `active_layer()`
//! method. The top-level `update()` can use this to route key events
//! cleanly:
//!
//! ```ignore
//! fn update(&mut self, msg: Msg) -> Command<Msg> {
//!     match msg {
//!         Msg::Key(key) => match self.active_layer() {
//!             InputLayer::Modal => self.route_modal(key),
//!             InputLayer::Overlay => self.route_overlay(key),
//!             InputLayer::Focused => self.route_focused(key),
//!         },
//!         // ...
//!     }
//! }
//! ```

/// Input routing priority layers.
///
/// Higher-priority layers capture input before lower-priority ones.
/// When a modal is active, it should capture all input. When an overlay
/// (like a search bar) is active, it captures most input. Otherwise,
/// the focused widget handles it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputLayer {
    /// Modal overlays — capture all input when active.
    ///
    /// Use for approval dialogs, question modals, wizards, confirmations.
    Modal,

    /// Overlays — capture most input when active.
    ///
    /// Use for search bars, autocomplete dropdowns, quick-action menus.
    Overlay,

    /// Focused widget — normal pane-based input routing.
    ///
    /// The default layer when no modal or overlay is active.
    Focused,
}

/// Extension trait for [`Model`](crate::Model) that adds input layer routing.
///
/// Implement this on your top-level Model to declare which layer should
/// handle input at any given time. The runtime does not enforce this —
/// it's a convention for your `update()` method to use.
pub trait LayeredModel: crate::Model {
    /// Return the active input layer.
    ///
    /// The model inspects its own state (are any modals open? is search
    /// active?) and returns the appropriate layer. The parent `update()`
    /// then routes key events accordingly.
    fn active_layer(&self) -> InputLayer;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layers_are_distinct() {
        assert_ne!(InputLayer::Modal, InputLayer::Overlay);
        assert_ne!(InputLayer::Modal, InputLayer::Focused);
        assert_ne!(InputLayer::Overlay, InputLayer::Focused);
    }

    #[test]
    fn layers_are_cloneable_and_debuggable() {
        let layer = InputLayer::Modal;
        let cloned = layer;
        assert_eq!(layer, cloned);
        assert_eq!(format!("{:?}", layer), "Modal");
    }
}
