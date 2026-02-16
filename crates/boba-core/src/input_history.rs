//! Browsable input history for text input widgets.
//!
//! Stores previously submitted inputs and allows browsing through them
//! with Up/Down keys, matching typical shell behavior.

/// Browsable input history for text input widgets.
///
/// Stores previously submitted strings and provides Up/Down navigation.
/// When the user starts browsing, the current (unsent) input is saved
/// as a "draft" so it can be restored when they navigate back past the
/// most recent entry.
///
/// # Example
///
/// ```
/// use boba_core::input_history::InputHistory;
///
/// let mut history = InputHistory::new(100);
///
/// history.push("first command");
/// history.push("second command");
///
/// // Browse backward through history
/// assert_eq!(history.older("current draft"), Some("second command"));
/// assert_eq!(history.older(""), Some("first command"));
///
/// // Browse forward
/// assert_eq!(history.newer(), Some("second command"));
///
/// // Back to the draft
/// assert_eq!(history.newer(), Some("current draft"));
///
/// // Past the draft returns None
/// assert_eq!(history.newer(), None);
/// ```
pub struct InputHistory {
    entries: Vec<String>,
    /// Current browsing position. `None` means not browsing (at draft).
    /// `Some(i)` means viewing `entries[i]`.
    index: Option<usize>,
    /// The draft text saved when the user starts browsing history.
    draft: String,
    max_entries: usize,
}

impl InputHistory {
    /// Create a new history with the given maximum number of entries.
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Vec::new(),
            index: None,
            draft: String::new(),
            max_entries,
        }
    }

    /// Add an entry to the history.
    ///
    /// Empty strings are ignored. Consecutive duplicates are not added.
    /// Resets the browsing position.
    pub fn push(&mut self, entry: impl Into<String>) {
        let entry = entry.into();
        if entry.is_empty() {
            return;
        }
        // Skip consecutive duplicates
        if self.entries.last().map(|e| e.as_str()) == Some(&entry) {
            self.reset_browse();
            return;
        }
        self.entries.push(entry);
        if self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }
        self.reset_browse();
    }

    /// Browse to the older (previous) entry.
    ///
    /// On the first call, `current_input` is saved as the draft so it
    /// can be restored later. Returns `Some(entry)` if there is an older
    /// entry, or `None` if already at the oldest.
    pub fn older(&mut self, current_input: &str) -> Option<&str> {
        if self.entries.is_empty() {
            return None;
        }
        match self.index {
            None => {
                // Start browsing: save draft, go to newest entry
                self.draft = current_input.to_string();
                let idx = self.entries.len() - 1;
                self.index = Some(idx);
                Some(&self.entries[idx])
            }
            Some(0) => {
                // Already at oldest entry
                None
            }
            Some(i) => {
                let idx = i - 1;
                self.index = Some(idx);
                Some(&self.entries[idx])
            }
        }
    }

    /// Browse to the newer (next) entry.
    ///
    /// Returns `Some(entry)` for a newer history entry, `Some(draft)` when
    /// returning to the unsent input, or `None` if already past the draft.
    pub fn newer(&mut self) -> Option<&str> {
        match self.index {
            None => {
                // Not browsing
                None
            }
            Some(i) => {
                if i + 1 < self.entries.len() {
                    let idx = i + 1;
                    self.index = Some(idx);
                    Some(&self.entries[idx])
                } else {
                    // Return to draft
                    self.index = None;
                    Some(&self.draft)
                }
            }
        }
    }

    /// Reset browsing state without clearing entries.
    ///
    /// Call this when the user submits input or takes an action that
    /// should exit history browsing mode.
    pub fn reset_browse(&mut self) {
        self.index = None;
        self.draft.clear();
    }

    /// Whether the user is currently browsing history.
    pub fn is_browsing(&self) -> bool {
        self.index.is_some()
    }

    /// The number of entries in the history.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the history is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get all entries (oldest first).
    pub fn entries(&self) -> &[String] {
        &self.entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_history_returns_none() {
        let mut history = InputHistory::new(10);
        assert_eq!(history.older("draft"), None);
        assert_eq!(history.newer(), None);
    }

    #[test]
    fn browse_single_entry() {
        let mut history = InputHistory::new(10);
        history.push("cmd1");

        assert_eq!(history.older("draft"), Some("cmd1"));
        // Can't go older
        assert_eq!(history.older(""), None);
        // Go back to draft
        assert_eq!(history.newer(), Some("draft"));
        // Past draft
        assert_eq!(history.newer(), None);
    }

    #[test]
    fn browse_multiple_entries() {
        let mut history = InputHistory::new(10);
        history.push("first");
        history.push("second");
        history.push("third");

        assert_eq!(history.older("my draft"), Some("third"));
        assert_eq!(history.older(""), Some("second"));
        assert_eq!(history.older(""), Some("first"));
        assert_eq!(history.older(""), None); // at oldest

        assert_eq!(history.newer(), Some("second"));
        assert_eq!(history.newer(), Some("third"));
        assert_eq!(history.newer(), Some("my draft"));
        assert_eq!(history.newer(), None); // past draft
    }

    #[test]
    fn push_resets_browsing() {
        let mut history = InputHistory::new(10);
        history.push("old");
        history.older("draft");
        assert!(history.is_browsing());

        history.push("new");
        assert!(!history.is_browsing());
    }

    #[test]
    fn max_entries_evicts_oldest() {
        let mut history = InputHistory::new(3);
        history.push("a");
        history.push("b");
        history.push("c");
        history.push("d");

        assert_eq!(history.len(), 3);
        assert_eq!(history.entries(), &["b", "c", "d"]);
    }

    #[test]
    fn empty_strings_ignored() {
        let mut history = InputHistory::new(10);
        history.push("");
        assert_eq!(history.len(), 0);
    }

    #[test]
    fn consecutive_duplicates_ignored() {
        let mut history = InputHistory::new(10);
        history.push("same");
        history.push("same");
        assert_eq!(history.len(), 1);

        history.push("different");
        history.push("same");
        assert_eq!(history.len(), 3);
    }

    #[test]
    fn reset_browse_clears_state() {
        let mut history = InputHistory::new(10);
        history.push("cmd");
        history.older("draft");
        assert!(history.is_browsing());

        history.reset_browse();
        assert!(!history.is_browsing());
        // Starting fresh
        assert_eq!(history.older("new draft"), Some("cmd"));
    }

    #[test]
    fn draft_preserved_during_browsing() {
        let mut history = InputHistory::new(10);
        history.push("old1");
        history.push("old2");

        // Start browsing with a draft
        assert_eq!(history.older("my unsent message"), Some("old2"));
        assert_eq!(history.older(""), Some("old1"));
        // Come back
        assert_eq!(history.newer(), Some("old2"));
        assert_eq!(history.newer(), Some("my unsent message"));
    }
}
