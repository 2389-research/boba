//! Tab navigation component for switching between views.

use boba_core::command::Command;
use boba_core::component::Component;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Tabs as RatatuiTabs};
use ratatui::Frame;

/// Messages for the tabs component.
#[derive(Debug, Clone)]
pub enum Message {
    /// A key press event forwarded to the tabs component.
    KeyPress(KeyEvent),
    /// Emitted when a tab is selected, carrying the tab index.
    Select(usize),
}

/// A tab navigation component that renders a horizontal row of labeled tabs
/// and tracks the currently selected index.
pub struct Tabs {
    titles: Vec<String>,
    selected: usize,
    focus: bool,
    style: TabsStyle,
}

/// Visual style configuration for the [`Tabs`] component.
#[derive(Debug, Clone)]
pub struct TabsStyle {
    /// Style applied to unselected tab labels.
    pub normal: Style,
    /// Style applied to the currently selected tab label.
    pub selected: Style,
    /// Style applied to the tab bar border.
    pub border: Style,
    /// String used as a divider between tab labels.
    pub divider: String,
}

impl Default for TabsStyle {
    fn default() -> Self {
        Self {
            normal: Style::default().fg(Color::DarkGray),
            selected: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            border: Style::default().fg(Color::DarkGray),
            divider: " | ".to_string(),
        }
    }
}

impl Tabs {
    /// Create a new tabs component with the given tab titles.
    pub fn new(titles: Vec<String>) -> Self {
        Self {
            titles,
            selected: 0,
            focus: false,
            style: TabsStyle::default(),
        }
    }

    /// Set the visual style for this tabs component.
    pub fn with_style(mut self, style: TabsStyle) -> Self {
        self.style = style;
        self
    }

    /// Give this tabs component keyboard focus.
    pub fn focus(&mut self) {
        self.focus = true;
    }

    /// Remove keyboard focus from this tabs component.
    pub fn blur(&mut self) {
        self.focus = false;
    }

    /// Return the index of the currently selected tab.
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// Select the tab at the given index. No-op if the index is out of bounds.
    pub fn select(&mut self, index: usize) {
        if index < self.titles.len() {
            self.selected = index;
        }
    }

    /// Advance to the next tab, wrapping around to the first tab after the last.
    pub fn select_next(&mut self) {
        if !self.titles.is_empty() {
            self.selected = (self.selected + 1) % self.titles.len();
        }
    }

    /// Move to the previous tab, wrapping around to the last tab before the first.
    pub fn select_prev(&mut self) {
        if !self.titles.is_empty() {
            self.selected = (self.selected + self.titles.len() - 1) % self.titles.len();
        }
    }
}

impl Component for Tabs {
    type Message = Message;

    fn update(&mut self, msg: Message) -> Command<Message> {
        match msg {
            Message::KeyPress(key) if self.focus => match key.code {
                KeyCode::Left | KeyCode::Char('h') => {
                    self.select_prev();
                    Command::message(Message::Select(self.selected))
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    self.select_next();
                    Command::message(Message::Select(self.selected))
                }
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    let idx = c.to_digit(10).unwrap() as usize;
                    if idx > 0 && idx <= self.titles.len() {
                        self.selected = idx - 1;
                        Command::message(Message::Select(self.selected))
                    } else {
                        Command::none()
                    }
                }
                _ => Command::none(),
            },
            Message::Select(i) => {
                self.select(i);
                Command::none()
            }
            _ => Command::none(),
        }
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        let titles: Vec<Line> = self.titles.iter().map(|t| Line::raw(t.as_str())).collect();

        let tabs = RatatuiTabs::new(titles)
            .block(Block::default().borders(Borders::BOTTOM).border_style(self.style.border))
            .select(self.selected)
            .style(self.style.normal)
            .highlight_style(self.style.selected)
            .divider(&self.style.divider);

        frame.render_widget(tabs, area);
    }

    fn focused(&self) -> bool {
        self.focus
    }
}
