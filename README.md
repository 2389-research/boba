# boba

A Bubble Tea-inspired TUI framework for Rust, built on ratatui.

boba brings the Elm Architecture to the terminal. Define your state, describe
how it updates, and render it -- boba handles the event loop, subscriptions,
and screen drawing for you.

## Features

- **Elm Architecture** -- `Model` trait with `init`, `update`, `view`, and `subscriptions` keeps logic and rendering cleanly separated.
- **Async commands via tokio** -- return a `Command` from `update` to spawn futures, emit messages, batch work, or quit the program.
- **Subscription system with auto-diffing** -- declare active subscriptions each cycle; boba starts new ones and stops removed ones automatically.
- **14+ ready-made widgets** -- text input, text area, list, table, select, tabs, spinner, progress bar, timer, stopwatch, file picker, viewport, paginator, and help bubble.
- **Component trait for composition** -- embed reusable sub-models that render into a caller-supplied `Rect`.
- **TestProgram for headless testing** -- send messages, drain queues, and assert against rendered buffers without a real terminal.
- **First-class ratatui integration** -- `view` receives a `ratatui::Frame` directly, so every ratatui widget and layout primitive works out of the box.

## Quick Start

Add boba to your project:

```toml
[dependencies]
boba = "0.1"
```

A minimal counter application:

```rust
use boba::crossterm::event::{KeyCode, KeyModifiers};
use boba::ratatui::layout::{Alignment, Constraint, Layout};
use boba::ratatui::style::{Color, Style};
use boba::ratatui::text::{Line, Span};
use boba::ratatui::widgets::{Block, Borders, Paragraph};
use boba::ratatui::Frame;
use boba::{terminal_events, Command, Model, Subscription, TerminalEvent};

struct Counter { count: i64 }

#[derive(Debug)]
enum Msg { Increment, Decrement, Reset, Quit, Noop }

impl Model for Counter {
    type Message = Msg;
    type Flags = ();

    fn init(_: ()) -> (Self, Command<Msg>) {
        (Counter { count: 0 }, Command::none())
    }

    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Increment => self.count += 1,
            Msg::Decrement => self.count -= 1,
            Msg::Reset     => self.count = 0,
            Msg::Quit      => return Command::quit(),
            Msg::Noop      => {}
        }
        Command::none()
    }

    fn view(&self, frame: &mut Frame) {
        let area = frame.area();
        let [_, mid, _] = Layout::vertical([
            Constraint::Fill(1), Constraint::Length(3), Constraint::Fill(1),
        ]).areas(area);

        let paragraph = Paragraph::new(format!("Count: {}", self.count))
            .block(Block::default().borders(Borders::ALL).title(" Counter "))
            .alignment(Alignment::Center);
        frame.render_widget(paragraph, mid);
    }

    fn subscriptions(&self) -> Vec<Subscription<Msg>> {
        vec![terminal_events(|ev| match ev {
            TerminalEvent::Key(key) => match key.code {
                KeyCode::Up    => Some(Msg::Increment),
                KeyCode::Down  => Some(Msg::Decrement),
                KeyCode::Char('r') => Some(Msg::Reset),
                KeyCode::Char('q') | KeyCode::Esc => Some(Msg::Quit),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Msg::Quit),
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
```

## Architecture

boba follows the Elm Architecture (also known as Model-View-Update):

```
                +------------------+
                |     Runtime      |
                +--------+---------+
                         |
          +--------------+--------------+
          |              |              |
     Subscriptions    Commands      View
     (terminal_events, (futures,    (ratatui
      timers, streams)  messages,    Frame)
          |              quit)          |
          v              |              v
      +---+--------------+---+    +-----------+
      |       update()       |    |  Terminal  |
      |  &mut self + Msg ->  |    |  (screen)  |
      |      Command<Msg>    |    +-----------+
      +-----------+----------+
                  |
                  v
            Model state
```

1. **Model** -- your application state, a plain Rust struct.
2. **Message** -- an enum describing every event your app handles.
3. **update** -- a pure-ish function: take the current state and a message, mutate state, optionally return a `Command`.
4. **view** -- render the current state to a ratatui `Frame`. Called after every update.
5. **subscriptions** -- declare which long-lived event sources are active. The runtime diffs them each cycle, starting and stopping as needed.

## Widget Showcase

| Widget | Description |
|---|---|
| `text_input` | Single-line text input with cursor, placeholder, and character limit |
| `text_area` | Multi-line text editor with line wrapping and scroll |
| `list` | Scrollable list with filtering, selection, and status bar |
| `table` | Column-based data table with sortable headers and row selection |
| `select` | Dropdown-style option picker with keyboard navigation |
| `tabs` | Horizontal tab bar with keyboard switching |
| `spinner` | Animated spinner with configurable frame sets |
| `progress` | Animated progress bar backed by a subscription |
| `timer` | Countdown timer with start, stop, and reset |
| `stopwatch` | Elapsed-time stopwatch with start, stop, and lap |
| `filepicker` | File-system browser with directory navigation |
| `viewport` | Scrollable content viewport with keyboard and scrollbar support |
| `paginator` | Page indicator (dots or fraction) for paged content |
| `help` | Overlay bubble showing keybinding help |
| `cursor` | Blinking cursor component for text widgets |
| `focus` | Focus group utility for routing input between components |
| `key` | Key binding definitions and matching helpers |
| `runeutil` | Unicode width, sanitization, and ANSI-to-ratatui parsing utilities |

## Examples

| Example | Description | Command |
|---|---|---|
| `counter` | Minimal counter demonstrating the core loop | `cargo run --example counter` |
| `input_form` | Multi-field form with text inputs and focus management | `cargo run --example input_form` |
| `async_http` | Async HTTP request via `Command::future` | `cargo run --example async_http` |
| `full_app` | Full application combining multiple widgets | `cargo run --example full_app` |
| `file_browser` | File-system browser using the filepicker widget | `cargo run --example file_browser` |

All examples live in the `examples/` directory and are compiled through the `boba` umbrella crate.

## Crate Structure

```
boba/                          workspace root
  crates/
    boba/                      umbrella crate -- re-exports core + widgets + ratatui + crossterm
    boba-core/                 runtime, Model & Component traits, Command, Subscription, TestProgram
    boba-widgets/              14+ reusable widget components
  examples/                    runnable example applications
```

- **boba** -- the crate end-users add to `Cargo.toml`. Re-exports everything needed to build an application.
- **boba-core** -- the runtime engine: event loop, terminal setup/teardown, subscription diffing, the `Model` and `Component` traits, and `TestProgram` for headless testing.
- **boba-widgets** -- a library of pre-built `Component` implementations (text input, list, table, spinner, etc.) that can be composed into any `Model`.

## Comparison with Bubble Tea

boba is directly inspired by [Bubble Tea](https://github.com/charmbracelet/bubbletea) for Go. Key differences:

- **Rust's type system enforces correctness** -- `Message` is a concrete enum, so the compiler catches unhandled variants. No `interface{}` casting.
- **Rendering targets ratatui instead of lipgloss** -- you get ratatui's layout engine (`Constraint`, `Layout`, `Flex`) and its full widget ecosystem.
- **The `Component` trait enables nested composition** -- sub-models render into a caller-provided `Rect`, making it straightforward to build complex layouts from reusable pieces.
- **Async is powered by tokio** -- `Command::future` spawns a tokio task, and subscriptions are backed by tokio streams, giving you the full async Rust ecosystem.

## License

MIT
