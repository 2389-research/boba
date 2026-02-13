//! # Async HTTP Example
//!
//! Demonstrates async side effects using `Command::perform`:
//! - Initiating async operations from `update`
//! - Handling async results with message mapping
//! - Conditional subscriptions (spinner only while loading)
//!
//! Run with: `cargo run --example async_http`

use boba::crossterm::event::{KeyCode, KeyModifiers};
use boba::ratatui::layout::{Alignment, Constraint, Layout};
use boba::ratatui::style::{Color, Modifier, Style};
use boba::ratatui::text::{Line, Span};
use boba::ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use boba::ratatui::Frame;
use boba::{terminal_events, Command, Model, Subscription, TerminalEvent};

/// An app that demonstrates async commands by fetching data.
struct HttpApp {
    status: Status,
    frame_idx: usize,
}

enum Status {
    Idle,
    Loading,
    Done(String),
    Error(String),
}

#[derive(Debug)]
enum Msg {
    Fetch,
    Fetched(Result<String, String>),
    Quit,
    Tick,
    Noop,
}

async fn fetch_data() -> Result<String, String> {
    // Simulate an HTTP request with a delay
    boba::tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    Ok("{\n  \"id\": 1,\n  \"title\": \"Hello from boba!\",\n  \"completed\": true,\n  \"description\": \"This data was fetched asynchronously using Command::perform.\"\n}".to_string())
}

impl Model for HttpApp {
    type Message = Msg;
    type Flags = ();

    fn init(_: ()) -> (Self, Command<Msg>) {
        (
            HttpApp {
                status: Status::Idle,
                frame_idx: 0,
            },
            Command::none(),
        )
    }

    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            // Command::perform spawns the async future and maps its result
            // into a Msg::Fetched message that arrives back in update.
            Msg::Fetch => {
                self.status = Status::Loading;
                self.frame_idx = 0;
                Command::perform(fetch_data(), Msg::Fetched)
            }
            Msg::Fetched(result) => {
                self.status = match result {
                    Ok(data) => Status::Done(data),
                    Err(e) => Status::Error(e),
                };
                Command::none()
            }
            Msg::Quit => Command::quit(),
            Msg::Tick => {
                self.frame_idx = (self.frame_idx + 1) % 10;
                Command::none()
            }
            Msg::Noop => Command::none(),
        }
    }

    fn view(&self, frame: &mut Frame) {
        let area = frame.area();

        let [header, body, footer] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Fill(1),
            Constraint::Length(3),
        ])
        .areas(area);

        // Header
        let title = Paragraph::new("Async HTTP Example")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan)));
        frame.render_widget(title, header);

        // Body
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        let content = match &self.status {
            Status::Idle => {
                Paragraph::new(Line::from(vec![
                    Span::raw("Press "),
                    Span::styled("f", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::raw(" to fetch data..."),
                ]))
            }
            Status::Loading => {
                let spinner_frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
                let spinner = spinner_frames[self.frame_idx];
                Paragraph::new(Line::from(vec![
                    Span::styled(spinner, Style::default().fg(Color::Yellow)),
                    Span::raw(" Fetching data..."),
                ]))
            }
            Status::Done(data) => {
                Paragraph::new(data.as_str())
                    .style(Style::default().fg(Color::Green))
                    .wrap(Wrap { trim: false })
            }
            Status::Error(err) => {
                Paragraph::new(format!("Error: {}", err))
                    .style(Style::default().fg(Color::Red))
            }
        };

        frame.render_widget(content.block(block), body);

        // Footer
        let help = Paragraph::new(Line::from(vec![
            Span::styled("f", Style::default().fg(Color::Cyan)),
            Span::raw(" fetch  "),
            Span::styled("q", Style::default().fg(Color::Cyan)),
            Span::raw(" quit"),
        ]))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
        frame.render_widget(help, footer);
    }

    fn subscriptions(&self) -> Vec<Subscription<Msg>> {
        let mut subs = vec![terminal_events(|ev| match ev {
            TerminalEvent::Key(key) => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => Some(Msg::Quit),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    Some(Msg::Quit)
                }
                KeyCode::Char('f') => Some(Msg::Fetch),
                _ => Some(Msg::Noop),
            },
            _ => None,
        })];

        // Conditional subscription: the spinner timer is only active while
        // loading. When the status changes away from Loading, the runtime
        // automatically stops this subscription.
        if matches!(self.status, Status::Loading) {
            subs.push(boba::subscribe(boba::Every::new(
                std::time::Duration::from_millis(80),
                "spinner",
            )).map(|_: std::time::Instant| Msg::Tick));
        }

        subs
    }
}

#[boba::tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    boba::run::<HttpApp>(()).await?;
    Ok(())
}
