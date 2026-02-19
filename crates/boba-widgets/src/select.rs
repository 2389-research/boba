//! Dropdown select/picker component for choosing from a list of options.
//!
//! This is a convenience wrapper that composes a one-line trigger display
//! with a [`Dropdown`](crate::dropdown::Dropdown) overlay for the actual
//! item list and navigation.

use crate::dropdown::{self, Dropdown, DropdownStyle};
use boba_core::command::Command;
use boba_core::component::Component;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
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
///
/// Internally this composes a [`Dropdown`] for the overlay portion, while
/// rendering its own single-line trigger display.
pub struct Select {
    options: Vec<String>,
    selected: Option<usize>,
    dropdown: Dropdown,
    open: bool,
    focus: bool,
    placeholder: String,
    style: SelectStyle,
    block: Option<Block<'static>>,
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
        let mut dropdown = Dropdown::new().with_max_visible(10);
        dropdown.set_items(options.clone());
        // Dropdown auto-shows on set_items; hide it since Select starts closed
        dropdown.hide();

        Self {
            options,
            selected: None,
            dropdown,
            open: false,
            focus: false,
            placeholder: "Select...".to_string(),
            style: SelectStyle::default(),
            block: None,
        }
    }

    /// Set the title displayed in the select border.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.dropdown.set_title(title);
        self
    }

    /// Set the placeholder text shown when no option is selected.
    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Set the visual style for this select component.
    ///
    /// The [`SelectStyle`] is mapped onto the internal [`Dropdown`]'s
    /// [`DropdownStyle`] so that item and selection colours stay in sync.
    pub fn with_style(mut self, style: SelectStyle) -> Self {
        self.dropdown = self.dropdown.with_style(DropdownStyle {
            item: style.normal,
            selected_item: style.selected,
        });
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
        self.dropdown = self.dropdown.with_block(block);
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
        self.dropdown.hide();
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

    /// Open the dropdown, forwarding to the internal Dropdown.
    fn open_dropdown(&mut self) {
        self.open = true;
        self.dropdown.show();
        if let Some(i) = self.selected {
            self.dropdown.set_selected(i);
        }
    }

    /// Close the dropdown.
    fn close_dropdown(&mut self) {
        self.open = false;
        self.dropdown.hide();
    }

    /// Map a [`dropdown::Message`] to a [`Message`], updating internal state
    /// as needed. Returns the [`Command`] to emit.
    fn handle_dropdown_result(&mut self, cmd: Command<dropdown::Message>) -> Command<Message> {
        cmd.map(|dmsg| match dmsg {
            dropdown::Message::Selected(idx, val) => Message::Selected(idx, val),
            dropdown::Message::Dismissed => Message::Close,
            dropdown::Message::KeyPress(k) => Message::KeyPress(k),
        })
    }
}

impl Component for Select {
    type Message = Message;

    fn update(&mut self, msg: Message) -> Command<Message> {
        match msg {
            Message::KeyPress(key) if self.focus => {
                if self.open {
                    // Forward to Dropdown and map the result
                    let cmd = self.dropdown.update(dropdown::Message::KeyPress(key));
                    // Check if the dropdown closed itself (Selected or Dismissed)
                    if !self.dropdown.is_visible() {
                        // Dropdown hid itself — sync our state
                        self.open = false;
                        // If it was a selection, capture it
                        if key.code == KeyCode::Enter {
                            let idx = self.dropdown.selected_index();
                            if let Some(val) = self.options.get(idx) {
                                self.selected = Some(idx);
                                return Command::message(Message::Selected(idx, val.clone()));
                            }
                        }
                    }
                    self.handle_dropdown_result(cmd)
                } else {
                    match key.code {
                        KeyCode::Enter | KeyCode::Char(' ') => {
                            self.open_dropdown();
                            Command::none()
                        }
                        _ => Command::none(),
                    }
                }
            }
            Message::Open => {
                self.open_dropdown();
                Command::none()
            }
            Message::Close => {
                self.close_dropdown();
                Command::none()
            }
            _ => Command::none(),
        }
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        // Render trigger line
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

        // Delegate dropdown overlay rendering to the internal Dropdown
        if self.open {
            self.dropdown.view(frame, area);
        }
    }

    fn focused(&self) -> bool {
        self.focus
    }
}
