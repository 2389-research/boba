//! Quit-confirmation helper for "press Ctrl+C again to quit" UX.

use std::time::{Duration, Instant};

/// Tracks double-press quit confirmation (e.g., "press Ctrl+C again to quit").
///
/// In raw terminal mode, Ctrl+C arrives as a key event rather than SIGINT.
/// This helper tracks the timing of quit requests so the Model can implement
/// a two-step quit flow: first press shows a warning, second press within
/// the timeout actually quits.
///
/// # Example
///
/// ```
/// use boba_core::quit::QuitConfirmation;
/// use std::time::Duration;
///
/// let mut quit = QuitConfirmation::new(Duration::from_secs(2));
///
/// // First press — not yet confirmed
/// assert!(!quit.request_quit());
///
/// // Second press immediately — confirmed
/// assert!(quit.request_quit());
/// ```
pub struct QuitConfirmation {
    last_request: Option<Instant>,
    timeout: Duration,
}

impl QuitConfirmation {
    /// Create a new quit confirmation tracker with the given timeout.
    ///
    /// Typical values are 1–3 seconds.
    pub fn new(timeout: Duration) -> Self {
        Self {
            last_request: None,
            timeout,
        }
    }

    /// Record a quit request (e.g., Ctrl+C was pressed).
    ///
    /// Returns `true` if this is the second request within the timeout window,
    /// meaning the application should actually quit. Returns `false` on the
    /// first request (the Model should show a "press again to quit" message).
    pub fn request_quit(&mut self) -> bool {
        if let Some(last) = self.last_request {
            if last.elapsed() < self.timeout {
                return true;
            }
        }
        self.last_request = Some(Instant::now());
        false
    }

    /// Reset the confirmation state.
    ///
    /// Call this when the user takes another action after the first Ctrl+C,
    /// so a stale first press doesn't count toward a future double-press.
    pub fn reset(&mut self) {
        self.last_request = None;
    }

    /// Returns `true` if a quit was requested and the timeout has not yet expired.
    ///
    /// Useful for showing a "press Ctrl+C again to quit" indicator in the view.
    pub fn is_pending(&self) -> bool {
        self.last_request
            .map(|t| t.elapsed() < self.timeout)
            .unwrap_or(false)
    }

    /// Returns the configured timeout duration.
    pub fn timeout(&self) -> Duration {
        self.timeout
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_request_returns_false() {
        let mut quit = QuitConfirmation::new(Duration::from_secs(2));
        assert!(!quit.request_quit());
    }

    #[test]
    fn second_request_within_timeout_returns_true() {
        let mut quit = QuitConfirmation::new(Duration::from_secs(2));
        assert!(!quit.request_quit());
        assert!(quit.request_quit());
    }

    #[test]
    fn second_request_after_timeout_returns_false() {
        let mut quit = QuitConfirmation::new(Duration::from_millis(1));
        assert!(!quit.request_quit());
        std::thread::sleep(Duration::from_millis(5));
        assert!(!quit.request_quit());
    }

    #[test]
    fn reset_clears_pending() {
        let mut quit = QuitConfirmation::new(Duration::from_secs(2));
        assert!(!quit.request_quit());
        assert!(quit.is_pending());
        quit.reset();
        assert!(!quit.is_pending());
        // After reset, next request is treated as first
        assert!(!quit.request_quit());
    }

    #[test]
    fn is_pending_tracks_state() {
        let mut quit = QuitConfirmation::new(Duration::from_secs(2));
        assert!(!quit.is_pending());
        quit.request_quit();
        assert!(quit.is_pending());
    }

    #[test]
    fn is_pending_expires_after_timeout() {
        let mut quit = QuitConfirmation::new(Duration::from_millis(1));
        quit.request_quit();
        std::thread::sleep(Duration::from_millis(5));
        assert!(!quit.is_pending());
    }
}
