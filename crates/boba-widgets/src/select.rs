//! Dropdown select/picker component for choosing from a list of options.

use boba_core::command::Command;
use boba_core::component::Component;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, HighlightSpacing, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

/// Messages for the select component.
#[derive(Debug, Clone)]
pub enum Message {
    /// A key press event forwarded to the select component.
    KeyPress(KeyEvent),
    /// Request to open the dropdown menu.
    Open,
    /// Request to close the dropdown menu.
    Close,
    /// Emitted when an option is selected, carrying the index and value.
    Selected(usize, String),
}

/// A dropdown/picker component that presents a list of options in a
/// collapsible overlay and tracks the current selection.
pub struct Select {
    options: Vec<String>,
    selected: Option<usize>,
    cursor: usize,
    open: bool,
    focus: bool,
    title: String,
    placeholder: String,
    style: SelectStyle,
    block: Option<Block<'static>>,
    dropdown_block: Option<Block<'static>>,
}

/// Visual style configuration for the [`Select`] component.
#[derive(Debug, Clone)]
pub struct SelectStyle {
    /// Style applied to normal (unselected) option text.
    pub normal: Style,
    /// Style applied to the currently highlighted/selected option.
    pub selected: Style,
    /// Symbol displayed next to the highlighted option in the dropdown.
    pub highlight_symbol: String,
}

impl Default for SelectStyle {
    fn default() -> Self {
        Self {
            normal: Style::default(),
            selected: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            highlight_symbol: "▸ ".to_string(),
        }
    }
}

impl Select {
    /// Create a new select component with the given list of options.
    pub fn new(options: Vec<String>) -> Self {
        Self {
            options,
            selected: None,
            cursor: 0,
            open: false,
            focus: false,
            title: String::new(),
            placeholder: "Select...".to_string(),
            style: SelectStyle::default(),
            block: None,
            dropdown_block: None,
        }
    }

    /// Set the title displayed in the select border.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Set the placeholder text shown when no option is selected.
    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Set the visual style for this select component.
    pub fn with_style(mut self, style: SelectStyle) -> Self {
        self.style = style;
        self
    }

    /// Set the block (border/title container) for the trigger area.
    pub fn with_block(mut self, block: Block<'static>) -> Self {
        self.block = Some(block);
        self
    }

    /// Set the block (border/title container) for the dropdown overlay.
    pub fn with_dropdown_block(mut self, block: Block<'static>) -> Self {
        self.dropdown_block = Some(block);
        self
    }

    /// Give this select component keyboard focus.
    pub fn focus(&mut self) {
        self.focus = true;
    }

    /// Remove keyboard focus and close the dropdown if open.
    pub fn blur(&mut self) {
        self.focus = false;
        self.open = false;
    }

    /// Return the index of the currently selected option, if any.
    pub fn selected_index(&self) -> Option<usize> {
        self.selected
    }

    /// Return the string value of the currently selected option, if any.
    pub fn selected_value(&self) -> Option<&str> {
        self.selected
            .and_then(|i| self.options.get(i).map(|s| s.as_str()))
    }
}

impl Component for Select {
    type Message = Message;

    fn update(&mut self, msg: Message) -> Command<Message> {
        match msg {
            Message::KeyPress(key) if self.focus => {
                if self.open && !self.options.is_empty() {
                    match key.code {
                        KeyCode::Up | KeyCode::Char('k') => {
                            if self.cursor > 0 {
                                self.cursor -= 1;
                            } else {
                                self.cursor = self.options.len().saturating_sub(1);
                            }
                            Command::none()
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if self.cursor + 1 >= self.options.len() {
                                self.cursor = 0;
                            } else {
                                self.cursor += 1;
                            }
                            Command::none()
                        }
                        KeyCode::Enter => {
                            self.selected = Some(self.cursor);
                            self.open = false;
                            let value = self.options[self.cursor].clone();
                            Command::message(Message::Selected(self.cursor, value))
                        }
                        KeyCode::Esc => {
                            self.open = false;
                            Command::none()
                        }
                        _ => Command::none(),
                    }
                } else {
                    match key.code {
                        KeyCode::Enter | KeyCode::Char(' ') => {
                            self.open = true;
                            if let Some(i) = self.selected {
                                self.cursor = i;
                            }
                            Command::none()
                        }
                        _ => Command::none(),
                    }
                }
            }
            Message::Open => {
                self.open = true;
                Command::none()
            }
            Message::Close => {
                self.open = false;
                Command::none()
            }
            _ => Command::none(),
        }
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        let inner = if let Some(ref block) = self.block {
            let inner = block.inner(area);
            frame.render_widget(block.clone(), area);
            inner
        } else {
            area
        };

        let display_text = if let Some(i) = self.selected {
            Span::styled(&self.options[i], self.style.normal)
        } else {
            Span::styled(&self.placeholder, Style::default().fg(Color::DarkGray))
        };

        let arrow = if self.open { " ▾" } else { " ▸" };
        let line = Line::from(vec![
            display_text,
            Span::styled(arrow, Style::default().fg(Color::DarkGray)),
        ]);

        let paragraph = Paragraph::new(line);
        frame.render_widget(paragraph, inner);

        // Render dropdown overlay when open
        if self.open {
            let dropdown_height = (self.options.len() as u16 + 2).min(10);
            let dropdown_area =
                Rect::new(area.x, area.y + area.height, area.width, dropdown_height);

            // Only render if within frame bounds
            let frame_area = frame.area();
            if dropdown_area.y + dropdown_area.height <= frame_area.height {
                frame.render_widget(Clear, dropdown_area);

                let items: Vec<ListItem> = self
                    .options
                    .iter()
                    .map(|s| ListItem::new(Line::raw(s)))
                    .collect();

                let mut state = ListState::default();
                state.select(Some(self.cursor));

                let mut list = List::new(items)
                    .highlight_style(self.style.selected)
                    .highlight_symbol(self.style.highlight_symbol.as_str())
                    .highlight_spacing(HighlightSpacing::Always);
                if let Some(ref block) = self.dropdown_block {
                    list = list.block(block.clone());
                }

                frame.render_stateful_widget(list, dropdown_area, &mut state);
            }
        }
    }

    fn focused(&self) -> bool {
        self.focus
    }
}
