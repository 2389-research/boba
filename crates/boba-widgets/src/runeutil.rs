//! Unicode text utilities for TUI rendering.
//!
//! Provides functions for sanitizing strings, calculating display widths
//! of Unicode text, truncating strings to fit within a given width, and
//! parsing ANSI escape sequences into styled ratatui text.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// Sanitize a string by removing non-printable characters.
///
/// Keeps printable characters, spaces, and common whitespace (newlines, tabs).
/// Removes all other control characters.
pub fn sanitize(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .collect()
}

/// Calculate the display width of a single character.
///
/// Uses a simple heuristic without external dependencies:
/// - ASCII printable: width 1
/// - CJK Unified Ideographs (U+4E00..U+9FFF): width 2
/// - CJK Unified Ideographs Extension A (U+3400..U+4DBF): width 2
/// - CJK Unified Ideographs Extension B (U+20000..U+2A6DF): width 2
/// - CJK Compatibility Ideographs (U+F900..U+FAFF): width 2
/// - Fullwidth forms (U+FF01..U+FF60): width 2
/// - Fullwidth forms (U+FFE0..U+FFE6): width 2
/// - Hangul Syllables (U+AC00..U+D7AF): width 2
/// - Control characters: width 0
/// - Most other characters: width 1
fn char_width(c: char) -> usize {
    if c.is_control() {
        return 0;
    }

    let cp = c as u32;

    // CJK Unified Ideographs
    if (0x4E00..=0x9FFF).contains(&cp) {
        return 2;
    }
    // CJK Unified Ideographs Extension A
    if (0x3400..=0x4DBF).contains(&cp) {
        return 2;
    }
    // CJK Unified Ideographs Extension B
    if (0x20000..=0x2A6DF).contains(&cp) {
        return 2;
    }
    // CJK Compatibility Ideographs
    if (0xF900..=0xFAFF).contains(&cp) {
        return 2;
    }
    // Fullwidth Forms
    if (0xFF01..=0xFF60).contains(&cp) {
        return 2;
    }
    // Fullwidth Symbol Variants
    if (0xFFE0..=0xFFE6).contains(&cp) {
        return 2;
    }
    // Hangul Syllables
    if (0xAC00..=0xD7AF).contains(&cp) {
        return 2;
    }
    // CJK Radicals Supplement, Kangxi Radicals
    if (0x2E80..=0x2FDF).contains(&cp) {
        return 2;
    }
    // CJK Symbols and Punctuation, Hiragana, Katakana
    if (0x3000..=0x30FF).contains(&cp) {
        return 2;
    }
    // Katakana Phonetic Extensions
    if (0x31F0..=0x31FF).contains(&cp) {
        return 2;
    }
    // Enclosed CJK Letters and Months
    if (0x3200..=0x32FF).contains(&cp) {
        return 2;
    }
    // CJK Compatibility
    if (0x3300..=0x33FF).contains(&cp) {
        return 2;
    }

    1
}

/// Calculate the display width of a string, accounting for wide characters.
///
/// CJK characters, fullwidth forms, and similar characters count as 2 columns.
/// Control characters count as 0 columns. All other printable characters count
/// as 1 column.
pub fn display_width(s: &str) -> usize {
    s.chars().map(char_width).sum()
}

/// Truncate a string to fit within `max_width` display columns.
///
/// If the string fits within `max_width`, it is returned unchanged.
/// If truncated, `tail` (e.g., "...") is appended. The total display width
/// of the result (including the tail) will not exceed `max_width`.
///
/// # Examples
///
/// ```
/// use boba_widgets::runeutil::truncate;
///
/// assert_eq!(truncate("hello world", 8, "..."), "hello...");
/// assert_eq!(truncate("hi", 10, "..."), "hi");
/// ```
pub fn truncate(s: &str, max_width: usize, tail: &str) -> String {
    let s_width = display_width(s);
    if s_width <= max_width {
        return s.to_string();
    }

    let tail_width = display_width(tail);
    if tail_width >= max_width {
        // The tail itself is wider than max_width; just return truncated tail
        let mut result = String::new();
        let mut width = 0;
        for c in tail.chars() {
            let cw = char_width(c);
            if width + cw > max_width {
                break;
            }
            result.push(c);
            width += cw;
        }
        return result;
    }

    let target_width = max_width - tail_width;
    let mut result = String::new();
    let mut width = 0;

    for c in s.chars() {
        let cw = char_width(c);
        if width + cw > target_width {
            break;
        }
        result.push(c);
        width += cw;
    }

    result.push_str(tail);
    result
}

/// Map a standard ANSI foreground color code (30-37) to a ratatui `Color`.
fn ansi_fg_color(code: u16) -> Option<Color> {
    match code {
        30 => Some(Color::Black),
        31 => Some(Color::Red),
        32 => Some(Color::Green),
        33 => Some(Color::Yellow),
        34 => Some(Color::Blue),
        35 => Some(Color::Magenta),
        36 => Some(Color::Cyan),
        37 => Some(Color::White),
        _ => None,
    }
}

/// Map a bright ANSI foreground color code (90-97) to a ratatui `Color`.
fn ansi_bright_fg_color(code: u16) -> Option<Color> {
    match code {
        90 => Some(Color::DarkGray),
        91 => Some(Color::LightRed),
        92 => Some(Color::LightGreen),
        93 => Some(Color::LightYellow),
        94 => Some(Color::LightBlue),
        95 => Some(Color::LightMagenta),
        96 => Some(Color::LightCyan),
        97 => Some(Color::White),
        _ => None,
    }
}

/// Map a standard ANSI background color code (40-47) to a ratatui `Color`.
fn ansi_bg_color(code: u16) -> Option<Color> {
    match code {
        40 => Some(Color::Black),
        41 => Some(Color::Red),
        42 => Some(Color::Green),
        43 => Some(Color::Yellow),
        44 => Some(Color::Blue),
        45 => Some(Color::Magenta),
        46 => Some(Color::Cyan),
        47 => Some(Color::White),
        _ => None,
    }
}

/// Map a bright ANSI background color code (100-107) to a ratatui `Color`.
fn ansi_bright_bg_color(code: u16) -> Option<Color> {
    match code {
        100 => Some(Color::DarkGray),
        101 => Some(Color::LightRed),
        102 => Some(Color::LightGreen),
        103 => Some(Color::LightYellow),
        104 => Some(Color::LightBlue),
        105 => Some(Color::LightMagenta),
        106 => Some(Color::LightCyan),
        107 => Some(Color::White),
        _ => None,
    }
}

/// Apply a sequence of SGR parameter codes to a `Style`.
///
/// Handles standard attributes (bold, dim, italic, underline, reversed),
/// foreground/background colors (standard and bright), and 256-color
/// extended sequences (`38;5;N` for foreground, `48;5;N` for background).
fn apply_sgr_codes(codes: &[u16], style: &mut Style) {
    let mut i = 0;
    while i < codes.len() {
        let code = codes[i];
        match code {
            0 => *style = Style::default(),
            1 => *style = style.add_modifier(Modifier::BOLD),
            2 => *style = style.add_modifier(Modifier::DIM),
            3 => *style = style.add_modifier(Modifier::ITALIC),
            4 => *style = style.add_modifier(Modifier::UNDERLINED),
            7 => *style = style.add_modifier(Modifier::REVERSED),
            30..=37 => {
                if let Some(c) = ansi_fg_color(code) {
                    *style = style.fg(c);
                }
            }
            40..=47 => {
                if let Some(c) = ansi_bg_color(code) {
                    *style = style.bg(c);
                }
            }
            90..=97 => {
                if let Some(c) = ansi_bright_fg_color(code) {
                    *style = style.fg(c);
                }
            }
            100..=107 => {
                if let Some(c) = ansi_bright_bg_color(code) {
                    *style = style.bg(c);
                }
            }
            38 => {
                // 256-color foreground: 38;5;N
                if i + 2 < codes.len() && codes[i + 1] == 5 {
                    let n = codes[i + 2];
                    *style = style.fg(Color::Indexed(n as u8));
                    i += 2;
                }
            }
            48 => {
                // 256-color background: 48;5;N
                if i + 2 < codes.len() && codes[i + 1] == 5 {
                    let n = codes[i + 2];
                    *style = style.bg(Color::Indexed(n as u8));
                    i += 2;
                }
            }
            _ => {}
        }
        i += 1;
    }
}

/// Parse a single line of text containing ANSI escape sequences into a
/// `Line` of styled `Span` objects.
fn parse_ansi_line(line: &str) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut current_style = Style::default();
    let mut buf = String::new();
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Check for CSI sequence: ESC [
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['

                // Read parameter bytes (digits and semicolons)
                let mut param_str = String::new();
                while let Some(&pc) = chars.peek() {
                    if pc.is_ascii_digit() || pc == ';' {
                        param_str.push(pc);
                        chars.next();
                    } else {
                        break;
                    }
                }

                // Read the final byte
                let final_byte = chars.next();

                if final_byte == Some('m') {
                    // SGR sequence — flush accumulated text
                    if !buf.is_empty() {
                        spans.push(Span::styled(
                            std::mem::take(&mut buf),
                            current_style,
                        ));
                    }

                    // Parse SGR codes
                    let codes: Vec<u16> = if param_str.is_empty() {
                        vec![0] // ESC[m is equivalent to ESC[0m (reset)
                    } else {
                        param_str
                            .split(';')
                            .filter_map(|s| s.parse::<u16>().ok())
                            .collect()
                    };

                    apply_sgr_codes(&codes, &mut current_style);
                }
                // Non-SGR CSI sequences are silently ignored
            } else {
                // Lone ESC without '[' — just skip
            }
        } else {
            buf.push(c);
        }
    }

    // Flush remaining text
    if !buf.is_empty() {
        spans.push(Span::styled(buf, current_style));
    }

    if spans.is_empty() {
        Line::from(vec![Span::raw(String::new())])
    } else {
        Line::from(spans)
    }
}

/// Parse a string containing ANSI SGR escape sequences into styled `Line` objects.
///
/// Splits the input by newlines and converts each line into a `Line` with
/// appropriately styled `Span`s. Supports standard attributes (bold, dim,
/// italic, underline, reversed), foreground/background colors (standard 30-37,
/// bright 90-97, and 256-color via 38;5;N / 48;5;N), and reset (0).
///
/// # Examples
///
/// ```
/// use boba_widgets::runeutil::parse_ansi;
///
/// let lines = parse_ansi("\x1b[31mhello\x1b[0m world");
/// assert_eq!(lines.len(), 1);
/// ```
pub fn parse_ansi(input: &str) -> Vec<Line<'static>> {
    input.split('\n').map(|line| parse_ansi_line(line)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_removes_control_chars() {
        assert_eq!(sanitize("hello\x00world"), "helloworld");
        assert_eq!(sanitize("abc\x07def"), "abcdef");
    }

    #[test]
    fn sanitize_keeps_newlines_and_tabs() {
        assert_eq!(sanitize("hello\nworld"), "hello\nworld");
        assert_eq!(sanitize("hello\tworld"), "hello\tworld");
    }

    #[test]
    fn sanitize_keeps_printable() {
        let s = "Hello, World! 123 @#$";
        assert_eq!(sanitize(s), s);
    }

    #[test]
    fn display_width_ascii() {
        assert_eq!(display_width("hello"), 5);
        assert_eq!(display_width(""), 0);
    }

    #[test]
    fn display_width_cjk() {
        // Each CJK character is width 2
        assert_eq!(display_width("\u{4E16}\u{754C}"), 4); // "世界"
    }

    #[test]
    fn display_width_mixed() {
        // "hi世界" = 2 + 4 = 6
        assert_eq!(display_width("hi\u{4E16}\u{754C}"), 6);
    }

    #[test]
    fn display_width_control_chars() {
        assert_eq!(display_width("\x00\x01\x02"), 0);
    }

    #[test]
    fn truncate_no_truncation_needed() {
        assert_eq!(truncate("hello", 10, "..."), "hello");
    }

    #[test]
    fn truncate_basic() {
        assert_eq!(truncate("hello world", 8, "..."), "hello...");
    }

    #[test]
    fn truncate_exact_fit() {
        assert_eq!(truncate("hello", 5, "..."), "hello");
    }

    #[test]
    fn truncate_with_cjk() {
        // "世界abc" has width 4+3=7. Truncate to 6 with "…" (width 1).
        // Target width = 5. "世界a" = 5. Result: "世界a…"
        let result = truncate("\u{4E16}\u{754C}abc", 6, "\u{2026}");
        assert_eq!(display_width(&result), 6);
    }

    #[test]
    fn truncate_empty_tail() {
        assert_eq!(truncate("hello world", 5, ""), "hello");
    }

    #[test]
    fn truncate_tail_wider_than_max() {
        assert_eq!(truncate("hello", 2, "..."), "..");
    }

    // ---- ANSI parser tests ----

    #[test]
    fn parse_ansi_basic_foreground_color() {
        let lines = parse_ansi("\x1b[31mhello\x1b[0m");
        assert_eq!(lines.len(), 1);
        let spans = &lines[0].spans;
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "hello");
        assert_eq!(spans[0].style, Style::default().fg(Color::Red));
    }

    #[test]
    fn parse_ansi_bold_and_color() {
        let lines = parse_ansi("\x1b[1;31mbold red\x1b[0m");
        assert_eq!(lines.len(), 1);
        let spans = &lines[0].spans;
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "bold red");
        assert_eq!(
            spans[0].style,
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD)
        );
    }

    #[test]
    fn parse_ansi_reset_mid_line() {
        let lines = parse_ansi("\x1b[32mgreen\x1b[0m plain");
        assert_eq!(lines.len(), 1);
        let spans = &lines[0].spans;
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].content, "green");
        assert_eq!(spans[0].style, Style::default().fg(Color::Green));
        assert_eq!(spans[1].content, " plain");
        assert_eq!(spans[1].style, Style::default());
    }

    #[test]
    fn parse_ansi_multi_line() {
        let lines = parse_ansi("\x1b[34mblue\x1b[0m\n\x1b[33myellow\x1b[0m");
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].spans[0].content, "blue");
        assert_eq!(lines[0].spans[0].style, Style::default().fg(Color::Blue));
        assert_eq!(lines[1].spans[0].content, "yellow");
        assert_eq!(lines[1].spans[0].style, Style::default().fg(Color::Yellow));
    }

    #[test]
    fn parse_ansi_no_escapes() {
        let lines = parse_ansi("plain text here");
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].spans[0].content, "plain text here");
        assert_eq!(lines[0].spans[0].style, Style::default());
    }

    #[test]
    fn parse_ansi_256_color() {
        // 38;5;208 = foreground indexed 208 (orange)
        let lines = parse_ansi("\x1b[38;5;208morange\x1b[0m");
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].spans[0].content, "orange");
        assert_eq!(
            lines[0].spans[0].style,
            Style::default().fg(Color::Indexed(208))
        );
    }

    #[test]
    fn parse_ansi_256_color_background() {
        // 48;5;42 = background indexed 42
        let lines = parse_ansi("\x1b[48;5;42mhighlight\x1b[0m");
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].spans[0].content, "highlight");
        assert_eq!(
            lines[0].spans[0].style,
            Style::default().bg(Color::Indexed(42))
        );
    }

    #[test]
    fn parse_ansi_bright_colors() {
        let lines = parse_ansi("\x1b[91mlight red\x1b[0m");
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].spans[0].content, "light red");
        assert_eq!(
            lines[0].spans[0].style,
            Style::default().fg(Color::LightRed)
        );
    }

    #[test]
    fn parse_ansi_background_color() {
        let lines = parse_ansi("\x1b[41mred bg\x1b[0m");
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].spans[0].content, "red bg");
        assert_eq!(lines[0].spans[0].style, Style::default().bg(Color::Red));
    }

    #[test]
    fn parse_ansi_empty_input() {
        let lines = parse_ansi("");
        assert_eq!(lines.len(), 1);
    }
}
