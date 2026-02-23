//! Example: Composing a chat interface from Viewport + custom rendering.
//!
//! Demonstrates how to build a chat UI without a dedicated Chat widget,
//! using Viewport for scrolling and manually rendering styled message lines.
//! This pattern is more flexible than a monolithic Chat widget because you
//! control exactly how messages are rendered and can add custom behavior.
//!
//! Run with: `cargo run --example chat`

use std::time::Duration;

use boba::crossterm::event::{KeyCode, KeyModifiers};
use boba::ratatui::layout::{Constraint, Layout};
use boba::ratatui::style::{Color, Modifier, Style};
use boba::ratatui::text::{Line, Span};
use boba::ratatui::widgets::Paragraph;
use boba::ratatui::Frame;
use boba::widgets::chrome::focus_block;
use boba::widgets::viewport::{self, Viewport};
use boba::{
    subscribe, terminal_events, Command, Component, Every, Model, Subscription, TerminalEvent,
};

// ---------------------------------------------------------------------------
// Domain types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    User,
    Assistant,
}

#[derive(Debug, Clone)]
struct ChatMessage {
    role: Role,
    content: String,
    is_streaming: bool,
}

impl ChatMessage {
    fn new(role: Role, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
            is_streaming: false,
        }
    }

    fn streaming(role: Role) -> Self {
        Self {
            role,
            content: String::new(),
            is_streaming: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Application state
// ---------------------------------------------------------------------------

struct ChatApp {
    messages: Vec<ChatMessage>,
    viewport: Viewport,
    /// Track whether we should auto-scroll to bottom on new content.
    auto_scroll: bool,
    /// Simulated streaming state: which response we're on.
    sim_response_idx: usize,
    /// Current character position in the simulated response.
    sim_char_pos: usize,
    /// Whether we're currently streaming a response.
    streaming: bool,
}

/// Canned responses the simulated assistant cycles through.
const RESPONSES: &[&str] = &[
    "Hello! I'm a chat assistant built entirely from boba primitives.\n\
     The Viewport widget handles all the scrolling for me.",
    "This example demonstrates composition over inheritance.\n\
     Instead of a monolithic Chat widget, we combine:\n\
     - Vec<ChatMessage> for state\n\
     - Viewport for scrolling\n\
     - Custom rendering for styled lines",
    "You can scroll up with the arrow keys or k/j.\n\
     Try pressing PageUp/PageDown or Home/End too!\n\
     The Viewport handles all of that automatically.",
    "When new messages arrive, we auto-scroll to the bottom.\n\
     But if you manually scroll up, auto-scroll pauses\n\
     until you press End or Esc to return to the bottom.",
];

// ---------------------------------------------------------------------------
// Messages
// ---------------------------------------------------------------------------

#[derive(Debug)]
enum Msg {
    Viewport(viewport::Message),
    StreamTick,
    SendMessage,
    Quit,
}

// ---------------------------------------------------------------------------
// Model implementation
// ---------------------------------------------------------------------------

impl Model for ChatApp {
    type Message = Msg;
    type Flags = ();

    fn init(_: ()) -> (Self, Command<Msg>) {
        let mut viewport = Viewport::new("");
        viewport.focus();

        let mut app = ChatApp {
            messages: Vec::new(),
            viewport,
            auto_scroll: true,
            sim_response_idx: 0,
            sim_char_pos: 0,
            streaming: false,
        };

        // Seed with a welcome message
        app.messages.push(ChatMessage::new(
            Role::Assistant,
            "Welcome! Press Enter to send a message. Press Esc or Ctrl-C to quit.",
        ));
        app.rebuild_viewport_content();

        (app, Command::none())
    }

    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Viewport(vmsg) => {
                // Track whether user scrolled away from bottom
                let was_at_bottom = self.viewport.at_bottom();
                let cmd = self.viewport.update(vmsg).map(Msg::Viewport);
                if was_at_bottom && !self.viewport.at_bottom() {
                    self.auto_scroll = false;
                }
                if self.viewport.at_bottom() {
                    self.auto_scroll = true;
                }
                cmd
            }
            Msg::StreamTick => {
                if !self.streaming {
                    return Command::none();
                }

                let response = RESPONSES[self.sim_response_idx % RESPONSES.len()];
                let end = (self.sim_char_pos + 3).min(response.len());
                let partial = &response[..end];

                if let Some(msg) = self.messages.last_mut() {
                    msg.content = partial.to_string();
                }

                self.sim_char_pos = end;

                if self.sim_char_pos >= response.len() {
                    // Done streaming
                    if let Some(msg) = self.messages.last_mut() {
                        msg.is_streaming = false;
                    }
                    self.streaming = false;
                    self.sim_response_idx += 1;
                }

                self.rebuild_viewport_content();

                if self.auto_scroll {
                    self.viewport.goto_bottom();
                }

                Command::none()
            }
            Msg::SendMessage => {
                if self.streaming {
                    return Command::none();
                }

                // Add user message
                self.messages.push(ChatMessage::new(
                    Role::User,
                    format!("Message #{}", self.messages.len() / 2 + 1),
                ));

                // Start streaming assistant response
                self.messages.push(ChatMessage::streaming(Role::Assistant));
                self.streaming = true;
                self.sim_char_pos = 0;

                self.rebuild_viewport_content();

                // Auto-scroll to bottom for new message
                self.auto_scroll = true;
                self.viewport.goto_bottom();

                Command::none()
            }
            Msg::Quit => Command::quit(),
        }
    }

    fn view(&self, frame: &mut Frame) {
        let area = frame.area();

        let [title_area, viewport_area, status_area, help_area] = Layout::vertical([
            Constraint::Length(2),
            Constraint::Fill(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .areas(area);

        // Title
        let title = Paragraph::new(Line::from(Span::styled(
            "Chat (composed from Viewport)",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));
        frame.render_widget(title, title_area);

        // Viewport with border
        let block = focus_block("Messages", self.viewport.focused());
        let inner = block.inner(viewport_area);
        frame.render_widget(block, viewport_area);
        self.viewport.view(frame, inner);

        // Status line
        let scroll_status = if self.auto_scroll {
            Span::styled("auto-scroll: on", Style::default().fg(Color::Green))
        } else {
            Span::styled(
                "auto-scroll: off (press End to resume)",
                Style::default().fg(Color::Yellow),
            )
        };
        let streaming_status = if self.streaming {
            Span::styled("  streaming...", Style::default().fg(Color::Yellow))
        } else {
            Span::raw("")
        };
        let status = Paragraph::new(Line::from(vec![scroll_status, streaming_status]));
        frame.render_widget(status, status_area);

        // Help
        let help = Paragraph::new(Line::from(vec![
            Span::styled("Enter", Style::default().fg(Color::DarkGray)),
            Span::raw(" send  "),
            Span::styled("Up/Down", Style::default().fg(Color::DarkGray)),
            Span::raw(" scroll  "),
            Span::styled("Home/End", Style::default().fg(Color::DarkGray)),
            Span::raw(" top/bottom  "),
            Span::styled("Esc", Style::default().fg(Color::DarkGray)),
            Span::raw(" quit"),
        ]));
        frame.render_widget(help, help_area);
    }

    fn subscriptions(&self) -> Vec<Subscription<Msg>> {
        let mut subs = vec![terminal_events(move |ev| match ev {
            TerminalEvent::Key(key) => match (key.code, key.modifiers) {
                (KeyCode::Esc, _) => Some(Msg::Quit),
                (KeyCode::Char('c'), m) if m.contains(KeyModifiers::CONTROL) => Some(Msg::Quit),
                (KeyCode::Enter, _) => Some(Msg::SendMessage),
                _ => Some(Msg::Viewport(viewport::Message::KeyPress(key))),
            },
            _ => None,
        })];

        // Timer for simulated streaming
        if self.streaming {
            subs.push(
                subscribe(Every::new(Duration::from_millis(50), "stream")).map(|_| Msg::StreamTick),
            );
        }

        subs
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

impl ChatApp {
    /// Rebuild the viewport's styled content from the current messages.
    fn rebuild_viewport_content(&mut self) {
        let mut lines: Vec<Line<'static>> = Vec::new();
        let separator = "\u{2500}".repeat(40);

        for (i, msg) in self.messages.iter().enumerate() {
            if i > 0 {
                lines.push(Line::from(Span::styled(
                    separator.clone(),
                    Style::default().fg(Color::DarkGray),
                )));
            }

            // Role header
            let (label, style) = match msg.role {
                Role::User => (
                    "You",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Role::Assistant => (
                    "Assistant",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
            };
            lines.push(Line::from(Span::styled(format!("{label}: "), style)));

            // Message content
            if msg.content.is_empty() && msg.is_streaming {
                lines.push(Line::from(Span::styled(
                    "  ...",
                    Style::default().fg(Color::Yellow),
                )));
            } else {
                for text_line in msg.content.lines() {
                    lines.push(Line::from(format!("  {text_line}")));
                }
                if msg.is_streaming {
                    lines.push(Line::from(Span::styled(
                        "  ...",
                        Style::default().fg(Color::Yellow),
                    )));
                }
            }
        }

        self.viewport.set_styled_content(lines);
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[boba::tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    boba::run::<ChatApp>(()).await?;
    Ok(())
}
