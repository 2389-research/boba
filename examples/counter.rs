//! # Counter Example
//!
//! A minimal counter app demonstrating the core boba architecture:
//! - Implementing the [`Model`] trait with `init`, `update`, `view`, and `subscriptions`
//! - Mapping terminal events to application messages
//! - Using `Command::none()` and `Command::quit()`
//!
//! Run with: `cargo run --example counter`

use boba::crossterm::event::{KeyCode, KeyModifiers};
use boba::ratatui::layout::{Alignment, Constraint, Layout};
use boba::ratatui::style::{Color, Modifier, Style};
use boba::ratatui::text::{Line, Span};
use boba::ratatui::widgets::{Block, Borders, Paragraph};
use boba::ratatui::Frame;
use boba::{terminal_events, Command, Model, Subscription, TerminalEvent};

/// A minimal counter app that validates the core loop.
struct Counter {
    count: i64,
}

#[derive(Debug)]
enum Msg {
    Increment,
    Decrement,
    Reset,
    Quit,
    Noop,
}

impl Model for Counter {
    type Message = Msg;
    type Flags = ();

    fn init(_: ()) -> (Self, Command<Msg>) {
        (Counter { count: 0 }, Command::none())
    }

    // Each match arm handles a single message variant. Most arms mutate state
    // and fall through to Command::none(), but Quit returns early with
    // Command::quit() to exit the event loop.
    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Increment => self.count += 1,
            Msg::Decrement => self.count -= 1,
            Msg::Reset => self.count = 0,
            Msg::Quit => return Command::quit(),
            Msg::Noop => {}
        }
        Command::none()
    }

    // The view function builds the UI declaratively each frame. Layout
    // constraints vertically center a fixed-height block on screen.
    fn view(&self, frame: &mut Frame) {
        let area = frame.area();

        let [_, mid, _] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(5),
            Constraint::Fill(1),
        ])
        .areas(area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" Counter ");

        let count_style = if self.count > 0 {
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else if self.count < 0 {
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        };

        let text = vec![
            Line::from(Span::styled(format!("Count: {}", self.count), count_style)),
            Line::raw(""),
            Line::from(vec![
                Span::styled("↑/k", Style::default().fg(Color::Cyan)),
                Span::raw(" inc  "),
                Span::styled("↓/j", Style::default().fg(Color::Cyan)),
                Span::raw(" dec  "),
                Span::styled("r", Style::default().fg(Color::Cyan)),
                Span::raw(" reset  "),
                Span::styled("q", Style::default().fg(Color::Cyan)),
                Span::raw(" quit"),
            ]),
        ];

        let paragraph = Paragraph::new(text)
            .block(block)
            .alignment(Alignment::Center);

        frame.render_widget(paragraph, mid);
    }

    // The subscription closure receives raw terminal events and maps them
    // to application messages. Returning None for an event discards it.
    fn subscriptions(&self) -> Vec<Subscription<Msg>> {
        vec![terminal_events(|ev| match ev {
            TerminalEvent::Key(key) => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => Some(Msg::Quit),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    Some(Msg::Quit)
                }
                KeyCode::Up | KeyCode::Char('k') => Some(Msg::Increment),
                KeyCode::Down | KeyCode::Char('j') => Some(Msg::Decrement),
                KeyCode::Char('r') => Some(Msg::Reset),
                _ => Some(Msg::Noop),
            },
            _ => None,
        })]
    }
}

#[boba::tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = boba::run::<Counter>(()).await?;
    println!("Final count: {}", model.count);
    Ok(())
}
