// ABOUTME: Focus management utility for cycling keyboard focus across N components.
// ABOUTME: Provides FocusGroup<N> with next/prev/direct focus and is_focused queries.

//! Focus management utility for cycling keyboard focus across components.

/// A utility to simplify the common pattern of routing keyboard input
/// to the focused component. `N` is the number of focusable slots.
///
/// # Example
///
/// ```ignore
/// use boba_widgets::focus::FocusGroup;
///
/// // Three focusable panes: sidebar, main content, detail panel
/// let mut focus = FocusGroup::<3>::new();
///
/// // Tab to cycle focus
/// focus.focus_next(); // now on slot 1 (main content)
///
/// // Route input based on focus
/// match focus.focused() {
///     0 => { /* send keys to sidebar */ }
///     1 => { /* send keys to main content */ }
///     2 => { /* send keys to detail panel */ }
///     _ => unreachable!(),
/// }
///
/// // Check specific slot
/// if focus.is_focused(1) {
///     // main content has focus
/// }
/// ```
///
/// See `examples/input_form.rs` for a complete working example with
/// `FocusGroup` routing between two `TextArea` widgets and a submit button.
pub struct FocusGroup<const N: usize> {
    focused: usize,
}

impl<const N: usize> FocusGroup<N> {
    /// Create a new focus group with focus on the first slot (index 0).
    pub fn new() -> Self {
        Self { focused: 0 }
    }

    /// Return the index of the currently focused slot.
    pub fn focused(&self) -> usize {
        self.focused
    }

    /// Move focus to the next slot, wrapping around after the last.
    pub fn focus_next(&mut self) {
        self.focused = (self.focused + 1) % N;
    }

    /// Move focus to the previous slot, wrapping around before the first.
    pub fn focus_prev(&mut self) {
        self.focused = (self.focused + N - 1) % N;
    }

    /// Set focus to the given slot index, clamped to the valid range.
    pub fn focus(&mut self, index: usize) {
        self.focused = index.min(N - 1);
    }

    /// Return whether the slot at the given index currently has focus.
    pub fn is_focused(&self, index: usize) -> bool {
        self.focused == index
    }
}

impl<const N: usize> Default for FocusGroup<N> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_at_zero() {
        let fg = FocusGroup::<3>::new();
        assert_eq!(fg.focused(), 0);
        assert!(fg.is_focused(0));
    }

    #[test]
    fn focus_next_wraps() {
        let mut fg = FocusGroup::<3>::new();
        fg.focus_next(); // 1
        fg.focus_next(); // 2
        fg.focus_next(); // 0 (wrap)
        assert_eq!(fg.focused(), 0);
    }

    #[test]
    fn focus_prev_wraps() {
        let mut fg = FocusGroup::<3>::new();
        fg.focus_prev(); // 2 (wrap backwards)
        assert_eq!(fg.focused(), 2);
    }

    #[test]
    fn focus_clamps() {
        let mut fg = FocusGroup::<3>::new();
        fg.focus(10);
        assert_eq!(fg.focused(), 2); // clamped to N-1
    }

    #[test]
    fn is_focused() {
        let mut fg = FocusGroup::<3>::new();
        assert!(fg.is_focused(0));
        assert!(!fg.is_focused(1));
        fg.focus_next();
        assert!(!fg.is_focused(0));
        assert!(fg.is_focused(1));
    }
}
