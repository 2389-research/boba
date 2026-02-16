//! Tracks multi-key sequences (like vim's `gg`) with a configurable timeout.

use crossterm::event::KeyCode;
use std::time::{Duration, Instant};

/// Tracks pending key sequences with a configurable timeout.
///
/// Used by widgets that need vim-style multi-key bindings (e.g., `gg` to go to
/// the first item). The tracker holds at most one pending key and checks whether
/// the next key completes a sequence within the timeout window.
///
/// # Example
///
/// ```
/// use boba_core::key_sequence::KeySequenceTracker;
/// use crossterm::event::KeyCode;
///
/// let mut tracker = KeySequenceTracker::new();
///
/// // First 'g' — no pending key, so no sequence yet.
/// assert!(tracker.completes_sequence(KeyCode::Char('g')).is_none());
/// tracker.set_pending(KeyCode::Char('g'));
///
/// // Second 'g' within timeout — completes the sequence.
/// assert_eq!(
///     tracker.completes_sequence(KeyCode::Char('g')),
///     Some(KeyCode::Char('g')),
/// );
/// ```
pub struct KeySequenceTracker {
    pending: Option<(KeyCode, Instant)>,
    timeout: Duration,
}

impl Default for KeySequenceTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl KeySequenceTracker {
    /// Create a new tracker with the default timeout of 500 ms.
    pub fn new() -> Self {
        Self {
            pending: None,
            timeout: Duration::from_millis(500),
        }
    }

    /// Create a new tracker with a custom timeout.
    pub fn with_timeout(timeout: Duration) -> Self {
        Self {
            pending: None,
            timeout,
        }
    }

    /// Check whether `current` completes a pending sequence.
    ///
    /// Returns `Some(prev_key)` if a key was pending and the timeout has not
    /// expired, allowing the caller to match on `(prev_key, current)`.
    /// Always clears the pending state.
    pub fn completes_sequence(&mut self, _current: KeyCode) -> Option<KeyCode> {
        if let Some((prev, time)) = self.pending.take() {
            if time.elapsed() < self.timeout {
                return Some(prev);
            }
        }
        None
    }

    /// Set a key as pending for a potential sequence.
    pub fn set_pending(&mut self, code: KeyCode) {
        self.pending = Some((code, Instant::now()));
    }

    /// Clear any pending key.
    pub fn clear(&mut self) {
        self.pending = None;
    }

    /// Returns `true` if there is a key waiting for a follow-up.
    pub fn has_pending(&self) -> bool {
        self.pending.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_pending_returns_none() {
        let mut tracker = KeySequenceTracker::new();
        assert!(tracker.completes_sequence(KeyCode::Char('g')).is_none());
    }

    #[test]
    fn sequence_completes_within_timeout() {
        let mut tracker = KeySequenceTracker::new();
        // First key
        assert!(tracker.completes_sequence(KeyCode::Char('g')).is_none());
        tracker.set_pending(KeyCode::Char('g'));
        // Second key immediately
        assert_eq!(
            tracker.completes_sequence(KeyCode::Char('g')),
            Some(KeyCode::Char('g')),
        );
    }

    #[test]
    fn sequence_expires_after_timeout() {
        let mut tracker = KeySequenceTracker::with_timeout(Duration::from_millis(1));
        tracker.set_pending(KeyCode::Char('g'));
        std::thread::sleep(Duration::from_millis(5));
        assert!(tracker.completes_sequence(KeyCode::Char('g')).is_none());
    }

    #[test]
    fn clear_removes_pending() {
        let mut tracker = KeySequenceTracker::new();
        tracker.set_pending(KeyCode::Char('g'));
        assert!(tracker.has_pending());
        tracker.clear();
        assert!(!tracker.has_pending());
        assert!(tracker.completes_sequence(KeyCode::Char('g')).is_none());
    }

    #[test]
    fn completes_sequence_clears_pending() {
        let mut tracker = KeySequenceTracker::new();
        tracker.set_pending(KeyCode::Char('g'));
        let _ = tracker.completes_sequence(KeyCode::Char('g'));
        // Pending is cleared after completes_sequence
        assert!(!tracker.has_pending());
        assert!(tracker.completes_sequence(KeyCode::Char('g')).is_none());
    }

    #[test]
    fn different_second_key_still_returns_prev() {
        let mut tracker = KeySequenceTracker::new();
        tracker.set_pending(KeyCode::Char('g'));
        // 'j' is not 'g', but completes_sequence still returns the pending key
        // so the caller can decide whether (g, j) is meaningful.
        assert_eq!(
            tracker.completes_sequence(KeyCode::Char('j')),
            Some(KeyCode::Char('g')),
        );
    }
}
