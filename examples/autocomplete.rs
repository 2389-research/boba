//! # Autocomplete Example
//!
//! Demonstrates composing TextInput + Dropdown for autocomplete behavior.
//! This shows the recommended pattern instead of using a dedicated
//! autocomplete widget — compose building blocks for your specific needs.
//!
//! Run with: `cargo run --example autocomplete`

use boba::crossterm::event::{KeyCode, KeyModifiers};
use boba::ratatui::layout::{Constraint, Layout};
use boba::ratatui::style::{Color, Modifier, Style};
use boba::ratatui::text::{Line, Span};
use boba::ratatui::widgets::Paragraph;
use boba::ratatui::Frame;
use boba::widgets::chrome::focus_block;
use boba::widgets::dropdown::{self, Dropdown};
use boba::widgets::text_input::{self, TextInput};
use boba::{terminal_events, Command, Component, Model, Subscription, TerminalEvent};

const FRUITS: &[&str] = &[
    "Apple",
    "Apricot",
    "Avocado",
    "Banana",
    "Blackberry",
    "Blueberry",
    "Cherry",
    "Coconut",
    "Cranberry",
    "Date",
    "Fig",
    "Grape",
    "Guava",
    "Kiwi",
    "Lemon",
    "Lime",
    "Mango",
    "Melon",
    "Nectarine",
    "Orange",
    "Papaya",
    "Peach",
    "Pear",
    "Pineapple",
    "Plum",
    "Pomegranate",
    "Raspberry",
    "Strawberry",
    "Tangerine",
    "Watermelon",
];

struct AutocompleteApp {
    input: TextInput,
    dropdown: Dropdown,
    selected_fruit: Option<String>,
    dropdown_open: bool,
}

#[derive(Debug)]
enum Msg {
    Input(text_input::Message),
    Drop(dropdown::Message),
    Quit,
}

impl Model for AutocompleteApp {
    type Message = Msg;
    type Flags = ();

    fn init(_: ()) -> (Self, Command<Msg>) {
        let mut input = TextInput::new("Search fruits...");
        input.focus();
        let dropdown = Dropdown::new().with_max_visible(8);
        (
            AutocompleteApp {
                input,
                dropdown,
                selected_fruit: None,
                dropdown_open: false,
            },
            Command::none(),
        )
    }

    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Input(text_input::Message::Changed(ref value)) => {
                let cmd = self
                    .input
                    .update(text_input::Message::Changed(value.clone()))
                    .map(Msg::Input);
                // Filter fruits based on input
                let filtered: Vec<String> = FRUITS
                    .iter()
                    .filter(|f| f.to_lowercase().contains(&value.to_lowercase()))
                    .map(|f| f.to_string())
                    .collect();
                self.dropdown.set_items(filtered);
                self.dropdown_open = !value.is_empty() && !self.dropdown.items().is_empty();
                if self.dropdown_open {
                    self.dropdown.show();
                } else {
                    self.dropdown.hide();
                }
                cmd
            }
            Msg::Input(text_input::Message::Submit(_)) => {
                // Accept from dropdown if open
                if self.dropdown_open {
                    if let Some(val) = self.dropdown.selected_value().map(str::to_owned) {
                        self.selected_fruit = Some(val.clone());
                        self.input.set_value(&val);
                        self.dropdown.hide();
                        self.dropdown_open = false;
                    }
                }
                Command::none()
            }
            Msg::Input(m) => self.input.update(m).map(Msg::Input),
            Msg::Drop(dropdown::Message::Selected(idx, ref val)) => {
                self.selected_fruit = Some(val.clone());
                self.input.set_value(val);
                self.dropdown.hide();
                self.dropdown_open = false;
                self.dropdown
                    .update(dropdown::Message::Selected(idx, val.clone()))
                    .map(Msg::Drop)
            }
            Msg::Drop(m) => self.dropdown.update(m).map(Msg::Drop),
            Msg::Quit => Command::quit(),
        }
    }

    fn view(&self, frame: &mut Frame) {
        let area = frame.area();

        let [title_area, input_area, _dropdown_area, status_area, help_area] = Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Length(10),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(area);

        // Title
        let title = Paragraph::new(Line::from(Span::styled(
            "Fruit Picker",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));
        frame.render_widget(title, title_area);

        // Input with border
        let block = focus_block("Search", self.input.focused());
        let inner = block.inner(input_area);
        frame.render_widget(block, input_area);
        self.input.view(frame, inner);

        // Dropdown (anchored to input_area so it renders directly below the input)
        if self.dropdown_open {
            self.dropdown.view(frame, input_area);
        }

        // Status
        if let Some(ref fruit) = self.selected_fruit {
            let status = Paragraph::new(Line::from(vec![
                Span::raw("Selected: "),
                Span::styled(
                    fruit,
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
            frame.render_widget(status, status_area);
        }

        // Help
        let help = Paragraph::new(Line::from(vec![
            Span::styled("Type", Style::default().fg(Color::DarkGray)),
            Span::raw(" to filter  "),
            Span::styled("Up/Down", Style::default().fg(Color::DarkGray)),
            Span::raw(" navigate  "),
            Span::styled("Enter", Style::default().fg(Color::DarkGray)),
            Span::raw(" select  "),
            Span::styled("Esc", Style::default().fg(Color::DarkGray)),
            Span::raw(" quit"),
        ]));
        frame.render_widget(help, help_area);
    }

    fn subscriptions(&self) -> Vec<Subscription<Msg>> {
        let dropdown_open = self.dropdown_open;
        vec![terminal_events(move |ev| match ev {
            TerminalEvent::Key(key) => match (key.code, key.modifiers) {
                (KeyCode::Esc, _) => Some(Msg::Quit),
                (KeyCode::Char('c'), m) if m.contains(KeyModifiers::CONTROL) => Some(Msg::Quit),
                (KeyCode::Up, _) if dropdown_open => {
                    Some(Msg::Drop(dropdown::Message::KeyPress(key)))
                }
                (KeyCode::Down, _) if dropdown_open => {
                    Some(Msg::Drop(dropdown::Message::KeyPress(key)))
                }
                (KeyCode::Enter, _) => Some(Msg::Input(text_input::Message::KeyPress(key))),
                _ => Some(Msg::Input(text_input::Message::KeyPress(key))),
            },
            _ => None,
        })]
    }
}

#[boba::tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    boba::run::<AutocompleteApp>(()).await?;
    Ok(())
}
