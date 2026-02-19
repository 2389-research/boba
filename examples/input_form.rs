//! # Input Form Example
//!
//! Demonstrates component composition with two `TextInput` widgets:
//! - Wrapping child component messages in a parent message enum
//! - Using `Command::map` to lift child commands into the parent message space
//! - Focus management with `FocusGroup`
//! - Password input with `EchoMode::Password`
//!
//! Run with: `cargo run --example input_form`

use boba::crossterm::event::{KeyCode, KeyModifiers};
use boba::ratatui::layout::{Alignment, Constraint, Layout};
use boba::ratatui::style::{Color, Modifier, Style};
use boba::ratatui::text::{Line, Span};
use boba::ratatui::widgets::{Block, Borders, Paragraph};
use boba::ratatui::Frame;
use boba::widgets::chrome::focus_block;
use boba::widgets::focus::FocusGroup;
use boba::widgets::text_input::{self, EchoMode, TextInput};
use boba::{terminal_events, Command, Component, Model, Subscription, TerminalEvent};

/// A form with two text inputs demonstrating component composition.
struct FormApp {
    username: TextInput,
    password: TextInput,
    focus: FocusGroup<3>,
    submitted: Option<String>,
}

// Each child component's message type is wrapped in a parent variant.
// This pattern lets the parent route messages to the correct child.
#[derive(Debug)]
enum Msg {
    Username(text_input::Message),
    Password(text_input::Message),
    FocusNext,
    FocusPrev,
    Submit,
    Quit,
}

impl Model for FormApp {
    type Message = Msg;
    type Flags = ();

    fn init(_: ()) -> (Self, Command<Msg>) {
        let mut username = TextInput::new("Enter username");
        username.focus();
        let password = TextInput::new("Enter password").with_echo_mode(EchoMode::Password('*'));
        (
            FormApp {
                username,
                password,
                focus: FocusGroup::new(),
                submitted: None,
            },
            Command::none(),
        )
    }

    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            // Delegate to the child component and use .map(Msg::Username) to lift
            // any commands the child returns back into the parent message space.
            Msg::Username(m) => self.username.update(m).map(Msg::Username),
            Msg::Password(m) => self.password.update(m).map(Msg::Password),
            Msg::FocusNext | Msg::FocusPrev => {
                if matches!(msg, Msg::FocusNext) {
                    self.focus.focus_next();
                } else {
                    self.focus.focus_prev();
                }
                // Update focus state on children
                match self.focus.focused() {
                    0 => {
                        self.username.focus();
                        self.password.blur();
                    }
                    1 => {
                        self.username.blur();
                        self.password.focus();
                    }
                    _ => {
                        // Submit button focused — blur both inputs
                        self.username.blur();
                        self.password.blur();
                    }
                }
                Command::none()
            }
            Msg::Submit => {
                let user = self.username.value();
                let pass = self.password.value();
                if user.is_empty() {
                    self.submitted = Some("Username is required!".to_string());
                } else {
                    self.submitted =
                        Some(format!("Submitted: {} / {}", user, "*".repeat(pass.len())));
                }
                Command::none()
            }
            Msg::Quit => Command::quit(),
        }
    }

    fn view(&self, frame: &mut Frame) {
        let area = frame.area();

        let [title_area, form_area, status_area, help_area] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(9),
            Constraint::Fill(1),
            Constraint::Length(3),
        ])
        .areas(area);

        // Title
        let title = Paragraph::new("Login Form")
            .alignment(Alignment::Center)
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(title, title_area);

        // Form fields
        let [user_area, pass_area, submit_area] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .areas(form_area);

        let block = focus_block("Username", self.username.focused());
        let inner = block.inner(user_area);
        frame.render_widget(block, user_area);
        self.username.view(frame, inner);

        let block = focus_block("Password", self.password.focused());
        let inner = block.inner(pass_area);
        frame.render_widget(block, pass_area);
        self.password.view(frame, inner);

        // Submit button
        let submit_style = if self.focus.focused() == 2 {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else {
            Style::default().fg(Color::Cyan)
        };
        let submit = Paragraph::new("  [ Submit ]  ")
            .alignment(Alignment::Center)
            .style(submit_style);
        frame.render_widget(submit, submit_area);

        // Status
        if let Some(ref msg) = self.submitted {
            let style = if msg.starts_with("Submitted") {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Red)
            };
            let status = Paragraph::new(msg.as_str())
                .alignment(Alignment::Center)
                .style(style)
                .block(Block::default().borders(Borders::ALL));
            frame.render_widget(status, status_area);
        }

        // Help
        let help = Paragraph::new(Line::from(vec![
            Span::styled("Tab", Style::default().fg(Color::Cyan)),
            Span::raw(" next  "),
            Span::styled("Shift+Tab", Style::default().fg(Color::Cyan)),
            Span::raw(" prev  "),
            Span::styled("Enter", Style::default().fg(Color::Cyan)),
            Span::raw(" submit  "),
            Span::styled("Esc", Style::default().fg(Color::Cyan)),
            Span::raw(" quit"),
        ]))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        frame.render_widget(help, help_area);
    }

    // Focus-aware routing: the subscription captures which child is focused
    // and routes unhandled key events to the appropriate TextInput component.
    fn subscriptions(&self) -> Vec<Subscription<Msg>> {
        let focused = self.focus.focused();
        vec![terminal_events(move |ev| match ev {
            TerminalEvent::Key(key) => match (key.code, key.modifiers) {
                (KeyCode::Esc, _) => Some(Msg::Quit),
                (KeyCode::Char('c'), m) if m.contains(KeyModifiers::CONTROL) => Some(Msg::Quit),
                (KeyCode::Tab, KeyModifiers::NONE) => Some(Msg::FocusNext),
                (KeyCode::BackTab, _) => Some(Msg::FocusPrev),
                (KeyCode::Enter, _) => Some(Msg::Submit),
                _ => {
                    // Route unhandled keys to whichever TextInput currently
                    // has focus, wrapping the child message in the parent enum.
                    let key_msg = text_input::Message::KeyPress(key);
                    if focused == 0 {
                        Some(Msg::Username(key_msg))
                    } else {
                        Some(Msg::Password(key_msg))
                    }
                }
            },
            _ => None,
        })]
    }
}

#[boba::tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    boba::run::<FormApp>(()).await?;
    Ok(())
}
