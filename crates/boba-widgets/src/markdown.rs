//! Markdown renderer that converts CommonMark to styled ratatui [`Line`]s.
//!
//! Uses [`pulldown_cmark`] for parsing and delegates fenced code blocks to
//! [`CodeBlock`](crate::code_block::CodeBlock) for syntax highlighting.
//!
//! This module is feature-gated behind `markdown` (which implies
//! `syntax-highlighting`).
//!
//! # Example
//!
//! ```ignore
//! use boba_widgets::markdown::Markdown;
//! use ratatui::style::Color;
//!
//! let md = Markdown::new();
//! let lines = md.parse("# Hello\n\nSome **bold** text.");
//! ```

use crate::code_block::CodeBlock;
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Parser, Tag, TagEnd};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// Style configuration for markdown rendering.
#[derive(Debug, Clone)]
pub struct MarkdownStyle {
    /// Color for H1 headings and bold text.
    pub primary: Color,
    /// Color for H2 headings and inline code.
    pub secondary: Color,
    /// Color for H3+ headings and italic text.
    pub accent: Color,
    /// Style for link URLs shown in parentheses.
    pub link_url: Style,
    /// Number of spaces to indent body text.
    pub indent: usize,
}

impl Default for MarkdownStyle {
    fn default() -> Self {
        Self {
            primary: Color::Cyan,
            secondary: Color::Yellow,
            accent: Color::Red,
            link_url: Style::default().fg(Color::DarkGray),
            indent: 2,
        }
    }
}

/// Stateless markdown-to-ratatui renderer.
///
/// Holds a [`CodeBlock`] for syntax highlighting of fenced code blocks
/// and a [`MarkdownStyle`] for colors.
pub struct Markdown {
    code_block: CodeBlock,
    style: MarkdownStyle,
}

impl Default for Markdown {
    fn default() -> Self {
        Self::new()
    }
}

impl Markdown {
    /// Create a new markdown renderer with default style and theme.
    pub fn new() -> Self {
        Self {
            code_block: CodeBlock::new(),
            style: MarkdownStyle::default(),
        }
    }

    /// Set the markdown style.
    pub fn with_style(mut self, style: MarkdownStyle) -> Self {
        self.style = style;
        self
    }

    /// Set the underlying code block highlighter.
    pub fn with_code_block(mut self, code_block: CodeBlock) -> Self {
        self.code_block = code_block;
        self
    }

    /// Get a reference to the underlying code block highlighter.
    pub fn code_block(&self) -> &CodeBlock {
        &self.code_block
    }

    /// Parse markdown content and return styled [`Line`]s.
    pub fn parse(&self, content: &str) -> Vec<Line<'static>> {
        let parser = Parser::new(content);
        let mut lines: Vec<Line<'static>> = Vec::new();
        let mut current_spans: Vec<Span<'static>> = Vec::new();
        let mut style_stack: Vec<Style> = vec![Style::default()];

        let indent = " ".repeat(self.style.indent);

        // Code block accumulation
        let mut in_code_block = false;
        let mut code_buffer = String::new();
        let mut code_language = String::new();

        // List tracking
        let mut list_stack: Vec<Option<u64>> = Vec::new();
        let mut list_item_counter: Vec<u64> = Vec::new();

        // Link URL tracking
        let mut link_url = String::new();

        // Heading state (suppress paragraph spacing inside headings)
        let mut in_heading = false;

        let flush_line = |current_spans: &mut Vec<Span<'static>>,
                          lines: &mut Vec<Line<'static>>,
                          indent: &str| {
            if !current_spans.is_empty() {
                let mut spans = vec![Span::raw(indent.to_string())];
                spans.append(current_spans);
                lines.push(Line::from(spans));
            }
        };

        for event in parser {
            match event {
                Event::Start(tag) => match tag {
                    Tag::Heading { level, .. } => {
                        in_heading = true;
                        let color = match level {
                            HeadingLevel::H1 => self.style.primary,
                            HeadingLevel::H2 => self.style.secondary,
                            _ => self.style.accent,
                        };
                        style_stack.push(Style::default().fg(color).add_modifier(Modifier::BOLD));
                    }
                    Tag::Strong => {
                        let base = *style_stack.last().unwrap_or(&Style::default());
                        style_stack.push(base.fg(self.style.primary).add_modifier(Modifier::BOLD));
                    }
                    Tag::Emphasis => {
                        let base = *style_stack.last().unwrap_or(&Style::default());
                        style_stack.push(base.fg(self.style.accent).add_modifier(Modifier::ITALIC));
                    }
                    Tag::CodeBlock(kind) => {
                        flush_line(&mut current_spans, &mut lines, &indent);
                        current_spans.clear();
                        in_code_block = true;
                        code_buffer.clear();
                        code_language = match kind {
                            CodeBlockKind::Fenced(lang) => lang.to_string(),
                            CodeBlockKind::Indented => String::new(),
                        };
                    }
                    Tag::List(start) => {
                        list_stack.push(start);
                        list_item_counter.push(start.unwrap_or(1));
                    }
                    Tag::Item => {
                        flush_line(&mut current_spans, &mut lines, &indent);
                        current_spans.clear();

                        let depth = list_stack.len().saturating_sub(1);
                        let list_indent = "  ".repeat(depth);

                        let bullet = if let Some(Some(_)) = list_stack.last() {
                            let counter = list_item_counter.last().copied().unwrap_or(1);
                            let b = format!("{list_indent}{counter}. ");
                            if let Some(c) = list_item_counter.last_mut() {
                                *c += 1;
                            }
                            b
                        } else {
                            format!("{list_indent}\u{2022} ")
                        };

                        let style = *style_stack.last().unwrap_or(&Style::default());
                        current_spans.push(Span::styled(bullet, style));
                    }
                    Tag::Link { dest_url, .. } => {
                        let base = *style_stack.last().unwrap_or(&Style::default());
                        style_stack.push(
                            base.fg(self.style.primary)
                                .add_modifier(Modifier::UNDERLINED),
                        );
                        link_url = dest_url.to_string();
                    }
                    Tag::Paragraph => {
                        flush_line(&mut current_spans, &mut lines, &indent);
                        current_spans.clear();
                    }
                    _ => {}
                },
                Event::End(tag_end) => match tag_end {
                    TagEnd::Heading(_) => {
                        flush_line(&mut current_spans, &mut lines, &indent);
                        current_spans.clear();
                        style_stack.pop();
                        in_heading = false;
                    }
                    TagEnd::Strong | TagEnd::Emphasis => {
                        style_stack.pop();
                    }
                    TagEnd::CodeBlock => {
                        in_code_block = false;
                        // Indent each highlighted line
                        let highlighted = self.code_block.highlight(&code_buffer, &code_language);
                        for hl_line in highlighted {
                            let mut spans = vec![Span::raw(indent.clone())];
                            spans.extend(hl_line.spans);
                            lines.push(Line::from(spans));
                        }
                        code_buffer.clear();
                        code_language.clear();
                    }
                    TagEnd::List(_) => {
                        list_stack.pop();
                        list_item_counter.pop();
                    }
                    TagEnd::Item => {
                        flush_line(&mut current_spans, &mut lines, &indent);
                        current_spans.clear();
                    }
                    TagEnd::Link => {
                        style_stack.pop();
                        if !link_url.is_empty() {
                            let url = std::mem::take(&mut link_url);
                            current_spans
                                .push(Span::styled(format!(" ({url})"), self.style.link_url));
                        }
                    }
                    TagEnd::Paragraph => {
                        flush_line(&mut current_spans, &mut lines, &indent);
                        current_spans.clear();
                        if !in_heading {
                            lines.push(Line::from(""));
                        }
                    }
                    _ => {}
                },
                Event::Text(text) => {
                    if in_code_block {
                        code_buffer.push_str(&text);
                    } else {
                        let style = *style_stack.last().unwrap_or(&Style::default());
                        current_spans.push(Span::styled(text.to_string(), style));
                    }
                }
                Event::Code(code) => {
                    current_spans.push(Span::styled(
                        format!("`{code}`"),
                        Style::default().fg(self.style.secondary),
                    ));
                }
                Event::SoftBreak => {
                    if !in_code_block {
                        current_spans.push(Span::raw(" ".to_string()));
                    }
                }
                Event::HardBreak => {
                    flush_line(&mut current_spans, &mut lines, &indent);
                    current_spans.clear();
                }
                _ => {}
            }
        }

        // Flush remaining spans
        flush_line(&mut current_spans, &mut lines, &indent);

        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Collect all non-empty, non-indent spans from lines.
    fn content_spans<'a>(lines: &'a [Line<'a>]) -> Vec<&'a Span<'a>> {
        lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .filter(|s| !s.content.trim().is_empty())
            .collect()
    }

    fn all_text(lines: &[Line<'_>]) -> String {
        lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.to_string())
            .collect()
    }

    #[test]
    fn h1_bold_primary() {
        let md = Markdown::new();
        let lines = md.parse("# Heading 1");
        let spans = content_spans(&lines);
        let heading = spans.iter().find(|s| s.content == "Heading 1").unwrap();
        assert!(heading.style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(heading.style.fg, Some(Color::Cyan)); // default primary
    }

    #[test]
    fn h2_bold_secondary() {
        let md = Markdown::new();
        let lines = md.parse("## Heading 2");
        let spans = content_spans(&lines);
        let heading = spans.iter().find(|s| s.content == "Heading 2").unwrap();
        assert!(heading.style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(heading.style.fg, Some(Color::Yellow)); // default secondary
    }

    #[test]
    fn bold_text() {
        let md = Markdown::new();
        let lines = md.parse("some **bold** text");
        let spans = content_spans(&lines);
        let bold = spans.iter().find(|s| s.content == "bold").unwrap();
        assert!(bold.style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn italic_text() {
        let md = Markdown::new();
        let lines = md.parse("some *italic* text");
        let spans = content_spans(&lines);
        let italic = spans.iter().find(|s| s.content == "italic").unwrap();
        assert!(italic.style.add_modifier.contains(Modifier::ITALIC));
    }

    #[test]
    fn inline_code() {
        let md = Markdown::new();
        let lines = md.parse("use `code` here");
        let spans = content_spans(&lines);
        let code = spans.iter().find(|s| s.content.contains("code")).unwrap();
        assert!(code.content.starts_with('`') && code.content.ends_with('`'));
        assert_eq!(code.style.fg, Some(Color::Yellow)); // default secondary
    }

    #[test]
    fn unordered_list() {
        let md = Markdown::new();
        let lines = md.parse("- apple\n- banana");
        let text = all_text(&lines);
        assert!(text.contains('\u{2022}'), "should contain bullet");
        assert!(text.contains("apple"));
        assert!(text.contains("banana"));
    }

    #[test]
    fn ordered_list() {
        let md = Markdown::new();
        let lines = md.parse("1. first\n2. second\n3. third");
        let text = all_text(&lines);
        assert!(text.contains("1. "));
        assert!(text.contains("2. "));
        assert!(text.contains("3. "));
    }

    #[test]
    fn fenced_code_block() {
        let md = Markdown::new();
        let lines = md.parse("```rust\nfn main() {}\n```");
        let text = all_text(&lines);
        assert!(text.contains("rust"), "should contain language label");
        assert!(text.contains('\u{250c}'), "should have top border");
        assert!(text.contains('\u{2514}'), "should have bottom border");
    }

    #[test]
    fn paragraph_spacing() {
        let md = Markdown::new();
        let lines = md.parse("First paragraph.\n\nSecond paragraph.");
        let has_empty = lines
            .iter()
            .any(|l| l.spans.iter().all(|s| s.content.trim().is_empty()));
        assert!(has_empty, "should have blank line between paragraphs");
    }

    #[test]
    fn links() {
        let md = Markdown::new();
        let lines = md.parse("[click here](https://example.com)");
        let spans = content_spans(&lines);
        let link = spans.iter().find(|s| s.content == "click here").unwrap();
        assert!(link.style.add_modifier.contains(Modifier::UNDERLINED));
        let text = all_text(&lines);
        assert!(text.contains("https://example.com"));
    }

    #[test]
    fn custom_style() {
        let style = MarkdownStyle {
            primary: Color::Green,
            secondary: Color::Magenta,
            accent: Color::Blue,
            ..Default::default()
        };
        let md = Markdown::new().with_style(style);
        let lines = md.parse("# Green Heading");
        let spans = content_spans(&lines);
        let heading = spans.iter().find(|s| s.content == "Green Heading").unwrap();
        assert_eq!(heading.style.fg, Some(Color::Green));
    }
}
