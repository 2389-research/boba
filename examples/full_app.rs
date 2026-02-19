//! # Full App Example
//!
//! A comprehensive example combining multiple boba components:
//! - Tabs for page navigation
//! - List with item selection
//! - Viewport for scrollable content
//! - Help overlay with keybinding display
//!
//! Run with: `cargo run --example full_app`

use boba::crossterm::event::{KeyCode, KeyModifiers};
use boba::ratatui::layout::{Constraint, Layout};
use boba::ratatui::style::{Color, Modifier, Style};
use boba::ratatui::text::{Line, Span};
use boba::ratatui::widgets::{Block, Borders, Paragraph};
use boba::ratatui::Frame;
use boba::widgets::help::{self, Help};
use boba::widgets::list::{self, List};
use boba::widgets::tabs::{self, Tabs};
use boba::widgets::viewport::{self, Viewport};
use boba::{terminal_events, Command, Component, Model, Subscription, TerminalEvent};

/// A full-featured app demonstrating multiple components together.
struct FullApp {
    tabs: Tabs,
    items: List<String>,
    content: Viewport,
    help: Help,
    panel_focus: usize, // 0 = list, 1 = viewport
}

#[derive(Debug)]
enum Msg {
    Tab(tabs::Message),
    Item(list::Message),
    Content(viewport::Message),
    Help(help::Message),
    FocusLeft,
    FocusRight,
    Quit,
    Noop,
}

const ITEMS: &[(&str, &str)] = &[
    ("Introduction", "Welcome to the boba full app example!\n\nThis demonstrates using multiple boba components together:\n- Tabs for navigation\n- List for item selection\n- Viewport for scrollable content\n- Help overlay for keybindings\n\nUse the arrow keys and tab to navigate."),
    ("Getting Started", "To get started with boba:\n\n1. Add boba to your Cargo.toml\n2. Define your Model with Message and Flags types\n3. Implement init, update, and view\n4. Use boba::run() to start the event loop\n\nThe framework handles terminal setup, event\nprocessing, and rendering for you."),
    ("Components", "Components are reusable pieces of UI that:\n\n- Have their own Message type\n- Implement update() and view()\n- Render into a Rect (not the full frame)\n- Can be composed into parent models\n\nKey composition pattern:\n1. Wrap child messages in parent enum\n2. Delegate in update with .map()\n3. Call child.view(frame, area)"),
    ("Commands", "Commands represent side effects:\n\n- Command::none() - no side effect\n- Command::message(msg) - send immediately\n- Command::quit() - exit the program\n- Command::perform(future, map) - async work\n- Command::batch([...]) - run concurrently\n- Command::sequence([...]) - run in order\n\nCommands are returned from update()."),
    ("Subscriptions", "Subscriptions are long-lived event sources:\n\n- terminal_events() - keyboard, mouse, resize\n- Every::new(duration) - repeating timer\n- After::new(duration) - one-shot delay\n\nSubscriptions are declared in subscriptions()\nand automatically started/stopped based on\nthe current set returned."),
];

impl Model for FullApp {
    type Message = Msg;
    type Flags = ();

    fn init(_: ()) -> (Self, Command<Msg>) {
        let tabs = Tabs::new(vec!["Browse".into(), "About".into()]);

        let items_list: Vec<String> = ITEMS.iter().map(|(name, _)| name.to_string()).collect();
        let mut items = List::new(items_list);
        items.focus();

        let content = Viewport::new(ITEMS[0].1);

        let mut help = Help::new();
        help.add_binding("↑/k", "Move up", "Navigation");
        help.add_binding("↓/j", "Move down", "Navigation");
        help.add_binding("←/h", "Focus list", "Navigation");
        help.add_binding("→/l", "Focus content", "Navigation");
        help.add_binding("Tab", "Switch tab", "Navigation");
        help.add_binding("?", "Toggle help", "General");
        help.add_binding("q/Esc", "Quit", "General");

        (
            FullApp {
                tabs,
                items,
                content,
                help,
                panel_focus: 0,
            },
            Command::none(),
        )
    }

    // Multi-component delegation: each match arm forwards the message to
    // the owning child component. Some arms intercept specific child
    // messages (e.g. Select, Confirm) to coordinate cross-component state.
    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            // Delegate tab navigation to the Tabs component.
            Msg::Tab(m) => self.tabs.update(m).map(Msg::Tab),
            // Intercept item selection to synchronize the viewport content.
            Msg::Item(list::Message::Select(i)) => {
                if i < ITEMS.len() {
                    self.content.set_content(ITEMS[i].1);
                }
                self.items.update(list::Message::Select(i)).map(Msg::Item)
            }
            Msg::Item(list::Message::Confirm(i)) => {
                // Switch focus to content on confirm
                self.panel_focus = 1;
                self.items.blur();
                self.content.focus();
                if i < ITEMS.len() {
                    self.content.set_content(ITEMS[i].1);
                }
                Command::none()
            }
            Msg::Item(m) => self.items.update(m).map(Msg::Item),
            Msg::Content(m) => self.content.update(m).map(Msg::Content),
            Msg::Help(m) => self.help.update(m).map(Msg::Help),
            Msg::FocusLeft => {
                self.panel_focus = 0;
                self.items.focus();
                self.content.blur();
                Command::none()
            }
            Msg::FocusRight => {
                self.panel_focus = 1;
                self.items.blur();
                self.content.focus();
                Command::none()
            }
            Msg::Quit => Command::quit(),
            Msg::Noop => Command::none(),
        }
    }

    fn view(&self, frame: &mut Frame) {
        let area = frame.area();

        let [tab_area, main_area, status_area] = Layout::vertical([
            Constraint::Length(2),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(area);

        // Tabs
        self.tabs.view(frame, tab_area);

        // Tab switching: render different content based on the active tab index.
        match self.tabs.selected() {
            0 => {
                let [list_area, content_area] =
                    Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)])
                        .areas(main_area);

                self.items.view(frame, list_area);
                self.content.view(frame, content_area);
            }
            _ => {
                let about = Paragraph::new(vec![
                    Line::from(Span::styled(
                        "boba",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::raw(""),
                    Line::raw("A Bubble Tea-inspired TUI framework for Rust,"),
                    Line::raw("built on top of ratatui and crossterm."),
                    Line::raw(""),
                    Line::raw("Version: 0.1.0"),
                ])
                .block(Block::default().borders(Borders::ALL));
                frame.render_widget(about, main_area);
            }
        }

        // Status bar
        let status = Paragraph::new(self.help.short_help_line());
        frame.render_widget(status, status_area);

        // Help overlay (renders on top)
        self.help.view(frame, area);
    }

    // Key routing depends on which panel has focus. Direction keys are sent
    // to the focused child component (list or viewport) as child messages.
    fn subscriptions(&self) -> Vec<Subscription<Msg>> {
        let panel_focus = self.panel_focus;
        vec![terminal_events(move |ev| match ev {
            TerminalEvent::Key(key) => match (key.code, key.modifiers) {
                (KeyCode::Char('q'), KeyModifiers::NONE) | (KeyCode::Esc, _) => Some(Msg::Quit),
                (KeyCode::Char('c'), m) if m.contains(KeyModifiers::CONTROL) => Some(Msg::Quit),
                (KeyCode::Char('?'), _) => Some(Msg::Help(help::Message::Toggle)),
                (KeyCode::Tab, _) => Some(Msg::Tab(tabs::Message::KeyPress(key))),
                (KeyCode::Left, _) | (KeyCode::Char('h'), _) => Some(Msg::FocusLeft),
                (KeyCode::Right, _) | (KeyCode::Char('l'), _) => {
                    if panel_focus == 1 {
                        // Already focused on content, let it handle the key
                        Some(Msg::Content(viewport::Message::KeyPress(key)))
                    } else {
                        Some(Msg::FocusRight)
                    }
                }
                (KeyCode::Up, _) | (KeyCode::Char('k'), _) => {
                    if panel_focus == 0 {
                        Some(Msg::Item(list::Message::KeyPress(key)))
                    } else {
                        Some(Msg::Content(viewport::Message::KeyPress(key)))
                    }
                }
                (KeyCode::Down, _) | (KeyCode::Char('j'), _) => {
                    if panel_focus == 0 {
                        Some(Msg::Item(list::Message::KeyPress(key)))
                    } else {
                        Some(Msg::Content(viewport::Message::KeyPress(key)))
                    }
                }
                (KeyCode::Enter, _) => {
                    if panel_focus == 0 {
                        Some(Msg::Item(list::Message::KeyPress(key)))
                    } else {
                        None
                    }
                }
                _ => Some(Msg::Noop),
            },
            _ => None,
        })]
    }
}

#[boba::tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    boba::run::<FullApp>(()).await?;
    Ok(())
}
