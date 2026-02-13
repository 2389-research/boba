//! Focus management utility for cycling keyboard focus across components.

/// A utility to simplify the common pattern of routing keyboard input
/// to the focused component. `N` is the number of focusable slots.
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
