//! Key binding definitions and key map trait for help display integration.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// A key binding that maps one or more key combinations to a described action.
pub struct Binding {
    /// The set of key combinations that trigger this binding.
    pub keys: Vec<KeyCombination>,
    /// A human-readable description of the action this binding performs.
    pub description: String,
    /// Whether this binding is currently active. Disabled bindings never match.
    pub enabled: bool,
}

/// A single key press with optional modifier keys (Ctrl, Alt, Shift).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyCombination {
    /// The base key code (e.g. a character, arrow key, or function key).
    pub code: KeyCode,
    /// Modifier keys that must be held alongside the base key.
    pub modifiers: KeyModifiers,
}

impl Binding {
    /// Create a new binding for a single key combination with the given description.
    pub fn new(key: KeyCombination, description: impl Into<String>) -> Self {
        Self {
            keys: vec![key],
            description: description.into(),
            enabled: true,
        }
    }

    /// Create a new binding for multiple key combinations with the given description.
    pub fn with_keys(keys: Vec<KeyCombination>, description: impl Into<String>) -> Self {
        Self {
            keys,
            description: description.into(),
            enabled: true,
        }
    }

    /// Return whether the given key event matches any of this binding's key combinations.
    /// Always returns `false` when the binding is disabled.
    pub fn matches(&self, event: &KeyEvent) -> bool {
        if !self.enabled {
            return false;
        }
        self.keys
            .iter()
            .any(|k| k.code == event.code && event.modifiers.contains(k.modifiers))
    }

    /// Set whether this binding is enabled. Disabled bindings never match key events.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl KeyCombination {
    /// Create a key combination with no modifier keys.
    pub fn new(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::NONE,
        }
    }

    /// Create a key combination with the Ctrl modifier.
    pub fn ctrl(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::CONTROL,
        }
    }

    /// Create a key combination with the Alt modifier.
    pub fn alt(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::ALT,
        }
    }

    /// Create a key combination with the Shift modifier.
    pub fn shift(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::SHIFT,
        }
    }

    /// Create a key combination with an explicit set of modifier keys.
    pub fn with_modifiers(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
    }
}

/// Trait for types that define key bindings, enabling integration with the
/// [`Help`](crate::help::Help) component.
pub trait KeyMap {
    /// Return a flat list of the most important bindings for the short help line.
    fn short_help(&self) -> Vec<&Binding>;
    /// Return bindings grouped by category for the full help overlay.
    fn full_help(&self) -> Vec<Vec<&Binding>>;
}
