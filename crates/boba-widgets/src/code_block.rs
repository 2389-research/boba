//! Syntax-highlighted code block renderer.
//!
//! Uses [`syntect`] for language-aware highlighting, converting tokens to
//! ratatui [`Line`]s with RGB colors and font styles.  The highlighter
//! loads the default syntect syntax definitions and theme set once, then
//! reuses them for every call to [`CodeBlock::highlight`].
//!
//! This module is feature-gated behind `syntax-highlighting`.
//!
//! # Example
//!
//! ```ignore
//! use boba_widgets::code_block::CodeBlock;
//!
//! let cb = CodeBlock::new();
//! let lines = cb.highlight("fn main() {}", "rust");
//! // `lines` is Vec<Line<'static>> with syntax-colored spans
//! ```

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

/// Style configuration for the code block border and label.
#[derive(Debug, Clone)]
pub struct CodeBlockStyle {
    /// Style for the box-drawing border characters.
    pub border: Style,
    /// Style for the language label in the header.
    pub label: Style,
}

impl Default for CodeBlockStyle {
    fn default() -> Self {
        Self {
            border: Style::default().fg(Color::DarkGray),
            label: Style::default().fg(Color::DarkGray),
        }
    }
}

/// Syntax highlighter backed by [`syntect`].
///
/// Holds the syntax and theme sets for the lifetime of the application.
/// Cheap to query — expensive work happens at construction time.
pub struct CodeBlock {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    theme_name: String,
    style: CodeBlockStyle,
    show_border: bool,
    show_line_numbers: bool,
}

impl Default for CodeBlock {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeBlock {
    /// Create a new code block highlighter with default theme (`base16-ocean.dark`).
    pub fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
            theme_name: "base16-ocean.dark".to_string(),
            style: CodeBlockStyle::default(),
            show_border: true,
            show_line_numbers: false,
        }
    }

    /// Set the syntect theme by name (e.g. `"base16-ocean.dark"`).
    ///
    /// If the name is not found in the default theme set, the current
    /// theme is kept unchanged.
    pub fn with_theme(mut self, theme: &str) -> Self {
        if self.theme_set.themes.contains_key(theme) {
            self.theme_name = theme.to_string();
        }
        self
    }

    /// Set the border/label style.
    pub fn with_style(mut self, style: CodeBlockStyle) -> Self {
        self.style = style;
        self
    }

    /// Toggle the box-drawing border (default: on).
    pub fn with_border(mut self, show: bool) -> Self {
        self.show_border = show;
        self
    }

    /// Toggle line numbers (default: off).
    pub fn with_line_numbers(mut self, show: bool) -> Self {
        self.show_line_numbers = show;
        self
    }

    /// Return the list of available theme names.
    pub fn available_themes(&self) -> Vec<&str> {
        self.theme_set.themes.keys().map(|s| s.as_str()).collect()
    }

    /// Highlight a code string and return styled [`Line`]s.
    ///
    /// `language` is matched first as a syntect token name, then as a
    /// file extension.  Falls back to plain text if nothing matches.
    pub fn highlight(&self, code: &str, language: &str) -> Vec<Line<'static>> {
        let raw = self.highlight_raw(code, language);

        if !self.show_border {
            return self.maybe_add_line_numbers(raw);
        }

        let lang_label = if language.is_empty() {
            "code"
        } else {
            language
        };
        let mut lines = Vec::with_capacity(raw.len() + 2);

        // Top border: ┌── lang
        lines.push(Line::from(Span::styled(
            format!("\u{250c}\u{2500}\u{2500} {lang_label} "),
            self.style.border,
        )));

        // Content lines with left border
        for hl_line in self.maybe_add_line_numbers(raw) {
            let mut spans = vec![Span::styled("\u{2502} ", self.style.border)];
            spans.extend(hl_line.spans);
            lines.push(Line::from(spans));
        }

        // Bottom border: └────────
        lines.push(Line::from(Span::styled(
            "\u{2514}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
            self.style.border,
        )));

        lines
    }

    /// Highlight without border or line numbers — raw styled lines only.
    pub fn highlight_raw(&self, code: &str, language: &str) -> Vec<Line<'static>> {
        let syntax = self
            .syntax_set
            .find_syntax_by_token(language)
            .or_else(|| self.syntax_set.find_syntax_by_extension(language))
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let theme = self
            .theme_set
            .themes
            .get(&self.theme_name)
            .unwrap_or_else(|| {
                self.theme_set
                    .themes
                    .values()
                    .next()
                    .expect("at least one theme")
            });

        let mut highlighter = HighlightLines::new(syntax, theme);
        let mut lines = Vec::new();

        for line in LinesWithEndings::from(code) {
            match highlighter.highlight_line(line, &self.syntax_set) {
                Ok(ranges) => {
                    let spans: Vec<Span<'static>> = ranges
                        .iter()
                        .map(|(style, text)| {
                            let fg = syntect_to_ratatui_color(style.foreground);
                            let mut ratatui_style = Style::default().fg(fg);
                            if style.font_style.contains(FontStyle::BOLD) {
                                ratatui_style = ratatui_style.add_modifier(Modifier::BOLD);
                            }
                            if style.font_style.contains(FontStyle::ITALIC) {
                                ratatui_style = ratatui_style.add_modifier(Modifier::ITALIC);
                            }
                            if style.font_style.contains(FontStyle::UNDERLINE) {
                                ratatui_style = ratatui_style.add_modifier(Modifier::UNDERLINED);
                            }
                            Span::styled(text.to_string(), ratatui_style)
                        })
                        .collect();
                    lines.push(Line::from(spans));
                }
                Err(_) => {
                    lines.push(Line::from(Span::raw(line.to_string())));
                }
            }
        }

        lines
    }

    fn maybe_add_line_numbers(&self, lines: Vec<Line<'static>>) -> Vec<Line<'static>> {
        if !self.show_line_numbers {
            return lines;
        }
        let width = lines.len().to_string().len();
        lines
            .into_iter()
            .enumerate()
            .map(|(i, line)| {
                let num = format!("{:>width$} ", i + 1, width = width);
                let mut spans = vec![Span::styled(num, Style::default().fg(Color::DarkGray))];
                spans.extend(line.spans);
                Line::from(spans)
            })
            .collect()
    }
}

/// Convert a syntect RGBA color to a ratatui RGB color.
fn syntect_to_ratatui_color(color: syntect::highlighting::Color) -> Color {
    Color::Rgb(color.r, color.g, color.b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_theme() {
        let cb = CodeBlock::new();
        assert_eq!(cb.theme_name, "base16-ocean.dark");
    }

    #[test]
    fn highlight_rust_produces_lines() {
        let cb = CodeBlock::new();
        let lines = cb.highlight("fn main() {}", "rust");
        // With border: header + content + footer = at least 3 lines
        assert!(
            lines.len() >= 3,
            "expected at least 3 lines, got {}",
            lines.len()
        );
    }

    #[test]
    fn highlight_raw_no_border() {
        let cb = CodeBlock::new();
        let lines = cb.highlight_raw("hello\nworld\n", "txt");
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn unknown_language_falls_back_to_plain_text() {
        let cb = CodeBlock::new();
        let lines = cb.highlight_raw("some text", "nonexistent_lang_xyz");
        assert!(!lines.is_empty());
    }

    #[test]
    fn border_toggle() {
        let cb = CodeBlock::new().with_border(false);
        let lines = cb.highlight("fn main() {}", "rust");
        // No border means no ┌ or └ characters
        let all_text: String = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.to_string())
            .collect();
        assert!(!all_text.contains('\u{250c}'), "should not have top border");
        assert!(
            !all_text.contains('\u{2514}'),
            "should not have bottom border"
        );
    }

    #[test]
    fn line_numbers() {
        let cb = CodeBlock::new().with_border(false).with_line_numbers(true);
        let lines = cb.highlight("line1\nline2\nline3\n", "txt");
        assert_eq!(lines.len(), 3);
        // First span of each line should be the line number
        assert!(lines[0].spans[0].content.contains('1'));
        assert!(lines[1].spans[0].content.contains('2'));
        assert!(lines[2].spans[0].content.contains('3'));
    }

    #[test]
    fn with_theme_valid() {
        let cb = CodeBlock::new().with_theme("InspiredGitHub");
        assert_eq!(cb.theme_name, "InspiredGitHub");
    }

    #[test]
    fn with_theme_invalid_keeps_default() {
        let cb = CodeBlock::new().with_theme("does_not_exist");
        assert_eq!(cb.theme_name, "base16-ocean.dark");
    }

    #[test]
    fn available_themes_not_empty() {
        let cb = CodeBlock::new();
        assert!(!cb.available_themes().is_empty());
    }

    #[test]
    fn border_has_language_label() {
        let cb = CodeBlock::new();
        let lines = cb.highlight("x = 1", "python");
        let header: String = lines[0]
            .spans
            .iter()
            .map(|s| s.content.to_string())
            .collect();
        assert!(
            header.contains("python"),
            "header should contain language name"
        );
    }
}
