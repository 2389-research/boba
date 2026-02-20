//! Keybinding help formatting utilities.
//!
//! `Help` is a pure rendering/formatting utility — no state, no update loop.
//! It holds a collection of [`HelpBinding`]s and style configuration, and
//! provides methods to produce formatted [`Line`]s for embedding in any widget.
//!
//! For overlay behavior (centered rect, scroll, key handling), compose
//! `help.full_help_view()` with [`overlay::render_overlay()`](crate::overlay::render_overlay)
//! and a scrollable widget like [`Paragraph`](ratatui::widgets::Paragraph).

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// A single keybinding entry displayed in help views.
#[derive(Debug, Clone)]
pub struct HelpBinding {
    /// The key or key combination label (e.g. "ctrl+c").
    pub keys: String,
    /// A short description of what this binding does.
    pub description: String,
    /// The logical group this binding belongs to (used for grouping in the full help view).
    pub group: String,
}

/// Visual style configuration for [`Help`] formatting.
#[derive(Debug, Clone)]
pub struct HelpStyle {
    /// Style applied to key labels.
    pub key: Style,
    /// Style applied to binding descriptions.
    pub description: Style,
    /// Style applied to group headings.
    pub group: Style,
    /// Style applied to the overlay title.
    pub title: Style,
}

impl Default for HelpStyle {
    fn default() -> Self {
        Self {
            key: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            description: Style::default().fg(Color::White),
            group: Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            title: Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        }
    }
}

/// A keybinding help formatter.
///
/// `Help` holds a list of bindings and style configuration. It provides methods
/// to produce formatted [`Line`]s suitable for rendering in a status bar
/// ([`short_help_line`](Help::short_help_line)) or a full help panel
/// ([`full_help_view`](Help::full_help_view)).
///
/// # Example
///
/// ```
/// use boba_widgets::help::Help;
///
/// let mut help = Help::new();
/// help.add_binding("?", "Toggle help", "General");
/// help.add_binding("q", "Quit", "General");
///
/// // For a status bar:
/// let line = help.short_help_line();
///
/// // For a full overlay (compose with overlay + Paragraph yourself):
/// let grouped = vec![help.bindings().to_vec()];
/// let lines = help.full_help_view(&grouped);
/// ```
pub struct Help {
    bindings: Vec<HelpBinding>,
    style: HelpStyle,
    separator: String,
    max_width: Option<u16>,
    ellipsis: String,
}

impl Help {
    /// Create a new help formatter with no bindings and default settings.
    pub fn new() -> Self {
        Self {
            bindings: Vec::new(),
            style: HelpStyle::default(),
            separator: " \u{2022} ".to_string(), // " • "
            max_width: None,
            ellipsis: "\u{2026}".to_string(), // "…"
        }
    }

    /// Set the full list of keybinding entries.
    pub fn with_bindings(mut self, bindings: Vec<HelpBinding>) -> Self {
        self.bindings = bindings;
        self
    }

    /// Set the visual style for this help formatter.
    pub fn with_style(mut self, style: HelpStyle) -> Self {
        self.style = style;
        self
    }

    /// Set the separator used between entries in `short_help_line()`.
    pub fn with_separator(mut self, s: impl Into<String>) -> Self {
        self.separator = s.into();
        self
    }

    /// Set a maximum width for `short_help_line()`. The line will be truncated
    /// with the configured ellipsis if it exceeds this width.
    pub fn with_max_width(mut self, w: u16) -> Self {
        self.max_width = Some(w);
        self
    }

    /// Set the ellipsis string used when truncating. Default is "\u{2026}".
    pub fn with_ellipsis(mut self, s: impl Into<String>) -> Self {
        self.ellipsis = s.into();
        self
    }

    /// Append a single keybinding entry with the given keys, description, and group.
    pub fn add_binding(
        &mut self,
        keys: impl Into<String>,
        description: impl Into<String>,
        group: impl Into<String>,
    ) {
        self.bindings.push(HelpBinding {
            keys: keys.into(),
            description: description.into(),
            group: group.into(),
        });
    }

    /// Return a reference to the current bindings.
    pub fn bindings(&self) -> &[HelpBinding] {
        &self.bindings
    }

    /// Build a short help line from all registered bindings (for a status bar).
    pub fn short_help_line(&self) -> Line<'_> {
        self.short_help_view(&self.bindings)
    }

    /// Render a short help line from externally-provided bindings.
    pub fn short_help_view(&self, bindings: &[HelpBinding]) -> Line<'_> {
        let mut spans: Vec<Span> = Vec::new();
        let mut total_width: usize = 0;
        let max = self.max_width.map(|w| w as usize);

        for (idx, b) in bindings.iter().take(5).enumerate() {
            let entry_spans = vec![
                Span::styled(b.keys.clone(), self.style.key),
                Span::raw(" "),
                Span::styled(b.description.clone(), self.style.description),
            ];

            let entry_width: usize = b.keys.len() + 1 + b.description.len();
            let sep_width = if idx > 0 { self.separator.len() } else { 0 };

            if let Some(max_w) = max {
                if total_width + sep_width + entry_width > max_w {
                    spans.push(Span::raw(self.ellipsis.clone()));
                    break;
                }
            }

            if idx > 0 {
                spans.push(Span::raw(self.separator.clone()));
                total_width += sep_width;
            }

            spans.extend(entry_spans);
            total_width += entry_width;
        }
        Line::from(spans)
    }

    /// Render a full help view from externally-provided grouped bindings.
    pub fn full_help_view<'a>(&'a self, bindings: &'a [Vec<HelpBinding>]) -> Vec<Line<'a>> {
        let mut lines: Vec<Line<'a>> = Vec::new();
        for (group_idx, group) in bindings.iter().enumerate() {
            if group_idx > 0 {
                lines.push(Line::raw(""));
            }
            if let Some(first) = group.first() {
                if !first.group.is_empty() {
                    lines.push(Line::from(Span::styled(
                        first.group.as_str(),
                        self.style.group,
                    )));
                }
            }
            for binding in group {
                lines.push(Line::from(vec![
                    Span::styled(format!("{:<12}", binding.keys), self.style.key),
                    Span::styled(binding.description.as_str(), self.style.description),
                ]));
            }
        }
        lines
    }
}

impl Default for Help {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_help_line_produces_expected_spans() {
        let help = Help::new().with_bindings(vec![
            HelpBinding {
                keys: "q".into(),
                description: "Quit".into(),
                group: "General".into(),
            },
            HelpBinding {
                keys: "?".into(),
                description: "Help".into(),
                group: "General".into(),
            },
        ]);

        let line = help.short_help_line();
        let spans = line.spans;
        // First entry: key, space, desc
        assert_eq!(spans[0].content, "q");
        assert_eq!(spans[1].content, " ");
        assert_eq!(spans[2].content, "Quit");
        // Separator
        assert_eq!(spans[3].content, " \u{2022} ");
        // Second entry: key, space, desc
        assert_eq!(spans[4].content, "?");
        assert_eq!(spans[5].content, " ");
        assert_eq!(spans[6].content, "Help");
    }

    #[test]
    fn short_help_line_truncates_with_ellipsis() {
        let help = Help::new().with_max_width(10).with_bindings(vec![
            HelpBinding {
                keys: "q".into(),
                description: "Quit".into(),
                group: "General".into(),
            },
            HelpBinding {
                keys: "?".into(),
                description: "Help".into(),
                group: "General".into(),
            },
        ]);

        let line = help.short_help_line();
        let spans = line.spans;
        // First entry fits (q + space + Quit = 6 chars)
        assert_eq!(spans[0].content, "q");
        assert_eq!(spans[2].content, "Quit");
        // Second entry would exceed max_width, so ellipsis is appended
        let last = &spans[spans.len() - 1];
        assert_eq!(last.content, "\u{2026}");
    }

    #[test]
    fn full_help_view_groups_bindings() {
        let help = Help::new();

        let nav = vec![
            HelpBinding {
                keys: "up".into(),
                description: "Move up".into(),
                group: "Navigation".into(),
            },
            HelpBinding {
                keys: "down".into(),
                description: "Move down".into(),
                group: "Navigation".into(),
            },
        ];
        let general = vec![HelpBinding {
            keys: "q".into(),
            description: "Quit".into(),
            group: "General".into(),
        }];

        let groups = vec![nav, general];
        let lines = help.full_help_view(&groups);
        // Group header "Navigation", 2 bindings, blank line, group header "General", 1 binding
        assert_eq!(lines.len(), 6);
        // First line is the Navigation group header
        assert_eq!(lines[0].spans[0].content, "Navigation");
        // Blank separator between groups
        assert_eq!(lines[3].spans.len(), 0); // blank Line::raw("")
                                             // General header
        assert_eq!(lines[4].spans[0].content, "General");
    }

    #[test]
    fn full_help_view_empty_bindings_returns_empty() {
        let help = Help::new();
        let lines = help.full_help_view(&[]);
        assert!(lines.is_empty());
    }

    #[test]
    fn bindings_accessor_returns_added_bindings() {
        let mut help = Help::new();
        help.add_binding("q", "Quit", "General");
        help.add_binding("?", "Help", "General");
        assert_eq!(help.bindings().len(), 2);
        assert_eq!(help.bindings()[0].keys, "q");
        assert_eq!(help.bindings()[1].keys, "?");
    }
}
