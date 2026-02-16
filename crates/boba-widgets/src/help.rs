//! Keybinding help overlay with short and full help views.

use std::cell::Cell;

use boba_core::command::Command;
use boba_core::component::Component;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

/// Messages for the help component.
#[derive(Debug, Clone)]
pub enum Message {
    /// A key press event forwarded to the help component.
    KeyPress(KeyEvent),
    /// Toggle the help overlay visibility.
    Toggle,
    /// Show the help overlay.
    Show,
    /// Hide the help overlay.
    Hide,
}

/// A keybinding help display component that can render a short inline help
/// line or a full scrollable overlay listing all registered bindings.
pub struct Help {
    bindings: Vec<HelpBinding>,
    visible: bool,
    style: HelpStyle,
    separator: String,
    max_width: Option<u16>,
    ellipsis: String,
    scroll_offset: usize,
    visible_height: Cell<u16>,
}

/// A single keybinding entry displayed in the help overlay.
#[derive(Debug, Clone)]
pub struct HelpBinding {
    /// The key or key combination label (e.g. "ctrl+c").
    pub keys: String,
    /// A short description of what this binding does.
    pub description: String,
    /// The logical group this binding belongs to (used for grouping in the full help view).
    pub group: String,
}

/// Visual style configuration for the [`Help`] component.
#[derive(Debug, Clone)]
pub struct HelpStyle {
    /// Style applied to key labels.
    pub key: Style,
    /// Style applied to binding descriptions.
    pub description: Style,
    /// Style applied to group headings.
    pub group: Style,
    /// Style applied to the overlay border.
    pub border: Style,
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
            border: Style::default().fg(Color::DarkGray),
            title: Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        }
    }
}

impl Help {
    /// Create a new help component with no bindings and default settings.
    pub fn new() -> Self {
        Self {
            bindings: Vec::new(),
            visible: false,
            style: HelpStyle::default(),
            separator: " \u{2022} ".to_string(), // " • "
            max_width: None,
            ellipsis: "\u{2026}".to_string(), // "…"
            scroll_offset: 0,
            visible_height: Cell::new(24),
        }
    }

    /// Set the full list of keybinding entries.
    pub fn with_bindings(mut self, bindings: Vec<HelpBinding>) -> Self {
        self.bindings = bindings;
        self
    }

    /// Set the visual style for this help component.
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

    /// Set the ellipsis string used when truncating. Default is "...".
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

    /// Return whether the help overlay is currently visible.
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Show the help overlay and reset the scroll position.
    pub fn show(&mut self) {
        self.visible = true;
        self.scroll_offset = 0;
    }

    /// Hide the help overlay and reset the scroll position.
    pub fn hide(&mut self) {
        self.visible = false;
        self.scroll_offset = 0;
    }

    /// Toggle help overlay visibility. Resets scroll position when showing.
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        if self.visible {
            self.scroll_offset = 0;
        }
    }

    /// Build a short help line (for a status bar).
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

impl Component for Help {
    type Message = Message;

    fn update(&mut self, msg: Message) -> Command<Message> {
        match msg {
            Message::KeyPress(key) => match key.code {
                KeyCode::Char('?') => {
                    self.toggle();
                    Command::none()
                }
                KeyCode::Esc if self.visible => {
                    self.hide();
                    Command::none()
                }
                KeyCode::Up | KeyCode::Char('k') if self.visible => {
                    self.scroll_offset = self.scroll_offset.saturating_sub(1);
                    Command::none()
                }
                KeyCode::Down | KeyCode::Char('j') if self.visible => {
                    // Clamp to total bindings as a reasonable upper bound.
                    // The exact max depends on rendered height, which is clamped in view().
                    let max = self.bindings.len();
                    self.scroll_offset = self.scroll_offset.saturating_add(1).min(max);
                    Command::none()
                }
                KeyCode::PageUp if self.visible => {
                    let visible_height = self.visible_height.get() as usize;
                    self.scroll_offset = self.scroll_offset.saturating_sub(visible_height);
                    Command::none()
                }
                KeyCode::PageDown if self.visible => {
                    let visible_height = self.visible_height.get() as usize;
                    let max = self.bindings.len();
                    self.scroll_offset = self.scroll_offset.saturating_add(visible_height).min(max);
                    Command::none()
                }
                KeyCode::Home if self.visible => {
                    self.scroll_offset = 0;
                    Command::none()
                }
                KeyCode::End if self.visible => {
                    // Set to bindings.len(); view() will clamp to the true max_scroll.
                    self.scroll_offset = self.bindings.len();
                    Command::none()
                }
                _ => Command::none(),
            },
            Message::Toggle => {
                self.toggle();
                Command::none()
            }
            Message::Show => {
                self.show();
                Command::none()
            }
            Message::Hide => {
                self.hide();
                Command::none()
            }
        }
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        // Center the help overlay
        let width = area.width.min(60);
        let height = area.height.min((self.bindings.len() as u16 + 4).min(20));
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 2;
        let overlay = Rect::new(x, y, width, height);

        frame.render_widget(Clear, overlay);

        let mut lines: Vec<Line> = Vec::new();

        let mut current_group = String::new();
        for binding in &self.bindings {
            if binding.group != current_group {
                if !current_group.is_empty() {
                    lines.push(Line::raw(""));
                }
                lines.push(Line::from(Span::styled(&binding.group, self.style.group)));
                current_group = binding.group.clone();
            }

            lines.push(Line::from(vec![
                Span::styled(format!("{:<12}", binding.keys), self.style.key),
                Span::styled(&binding.description, self.style.description),
            ]));
        }

        let block = Block::default()
            .title(" Help ")
            .title_style(self.style.title)
            .borders(Borders::ALL)
            .border_style(self.style.border);

        // Update visible_height via interior mutability.
        let inner_height = block.inner(overlay).height as usize;
        self.visible_height.set(inner_height as u16);
        let total_lines = lines.len();
        let max_scroll = total_lines.saturating_sub(inner_height);
        let offset = self.scroll_offset.min(max_scroll);

        let paragraph = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((offset as u16, 0));

        frame.render_widget(paragraph, overlay);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boba_core::component::Component;
    use crossterm::event::{KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn make_help(binding_count: usize) -> Help {
        let mut h = Help::new();
        for i in 0..binding_count {
            h.add_binding(format!("key{i}"), format!("desc{i}"), "group".to_string());
        }
        h.show();
        h
    }

    #[test]
    fn page_down_scrolls_by_visible_height() {
        let mut h = make_help(100);
        // Default visible_height is 24.
        assert_eq!(h.scroll_offset, 0);

        h.update(Message::KeyPress(key(KeyCode::PageDown)));
        assert_eq!(h.scroll_offset, 24);

        h.update(Message::KeyPress(key(KeyCode::PageDown)));
        assert_eq!(h.scroll_offset, 48);
    }

    #[test]
    fn page_up_scrolls_by_visible_height() {
        let mut h = make_help(100);
        h.scroll_offset = 50;

        h.update(Message::KeyPress(key(KeyCode::PageUp)));
        assert_eq!(h.scroll_offset, 26);

        h.update(Message::KeyPress(key(KeyCode::PageUp)));
        assert_eq!(h.scroll_offset, 2);
    }

    #[test]
    fn page_up_saturates_at_zero() {
        let mut h = make_help(100);
        h.scroll_offset = 10;

        h.update(Message::KeyPress(key(KeyCode::PageUp)));
        assert_eq!(h.scroll_offset, 0);
    }

    #[test]
    fn page_down_clamps_at_max() {
        let mut h = make_help(30);
        // visible_height defaults to 24, bindings.len() == 30.
        h.scroll_offset = 20;

        h.update(Message::KeyPress(key(KeyCode::PageDown)));
        // 20 + 24 = 44, clamped to 30.
        assert_eq!(h.scroll_offset, 30);
    }

    #[test]
    fn home_scrolls_to_top() {
        let mut h = make_help(100);
        h.scroll_offset = 42;

        h.update(Message::KeyPress(key(KeyCode::Home)));
        assert_eq!(h.scroll_offset, 0);
    }

    #[test]
    fn end_scrolls_to_bottom() {
        let mut h = make_help(100);
        assert_eq!(h.scroll_offset, 0);

        h.update(Message::KeyPress(key(KeyCode::End)));
        assert_eq!(h.scroll_offset, 100);
    }

    #[test]
    fn page_keys_no_op_when_hidden() {
        let mut h = make_help(100);
        h.hide();

        h.update(Message::KeyPress(key(KeyCode::PageDown)));
        assert_eq!(h.scroll_offset, 0);

        h.update(Message::KeyPress(key(KeyCode::PageUp)));
        assert_eq!(h.scroll_offset, 0);

        h.update(Message::KeyPress(key(KeyCode::Home)));
        assert_eq!(h.scroll_offset, 0);

        h.update(Message::KeyPress(key(KeyCode::End)));
        assert_eq!(h.scroll_offset, 0);
    }

    #[test]
    fn custom_visible_height_affects_page_scroll() {
        let mut h = make_help(100);
        h.visible_height.set(10);

        h.update(Message::KeyPress(key(KeyCode::PageDown)));
        assert_eq!(h.scroll_offset, 10);

        h.update(Message::KeyPress(key(KeyCode::PageUp)));
        assert_eq!(h.scroll_offset, 0);
    }
}
