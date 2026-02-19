# boba

A Bubble Tea-inspired TUI framework for Rust, built on [ratatui](https://ratatui.rs).

boba brings the [Elm Architecture](https://guide.elm-lang.org/architecture/) to the terminal. Define your state, describe how it updates, and render it -- boba handles the event loop, subscriptions, and screen drawing for you.

```rust
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
            Msg::Quit      => return Command::quit(),
        }
        Command::none()
    }

    fn view(&self, frame: &mut Frame) {
        let p = Paragraph::new(format!("Count: {}", self.count));
        frame.render_widget(p, frame.area());
    }
}
```

## Features

- **Elm Architecture** -- `Model` trait with `init`, `update`, `view`, and `subscriptions` keeps logic and rendering cleanly separated.
- **Async commands via tokio** -- return a `Command` from `update` to spawn futures, emit messages, batch work, or quit.
- **Subscription system with auto-diffing** -- declare active subscriptions each cycle; boba starts new ones and stops removed ones automatically.
- **18 ready-made widgets** -- text input, text area, list, table, dropdown, select, tabs, spinner, progress, timer, stopwatch, file picker, viewport, modal, paginator, search, status bar, and more.
- **Composable by design** -- every widget is borderless by default and renders into a caller-supplied `Rect`. You control the chrome.
- **Component trait for nesting** -- embed reusable sub-models that manage their own state and message types.
- **TestProgram for headless testing** -- send messages, drain queues, and assert against rendered buffers without a real terminal.
- **First-class ratatui integration** -- `view` receives a `ratatui::Frame` directly, so every ratatui widget and layout primitive works out of the box.

## Quick Start

```toml
[dependencies]
boba = "0.1"
```

```rust
use boba::crossterm::event::{KeyCode, KeyModifiers};
use boba::ratatui::widgets::Paragraph;
use boba::ratatui::Frame;
use boba::{terminal_events, Command, Model, Subscription, TerminalEvent};

struct Counter { count: i64 }

#[derive(Debug)]
enum Msg { Increment, Decrement, Quit, Noop }

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
            Msg::Quit      => return Command::quit(),
            Msg::Noop      => {}
        }
        Command::none()
    }

    fn view(&self, frame: &mut Frame) {
        let p = Paragraph::new(format!("Count: {}", self.count));
        frame.render_widget(p, frame.area());
    }

    fn subscriptions(&self) -> Vec<Subscription<Msg>> {
        vec![terminal_events(|ev| match ev {
            TerminalEvent::Key(key) => match key.code {
                KeyCode::Up   => Some(Msg::Increment),
                KeyCode::Down => Some(Msg::Decrement),
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

boba follows the Elm Architecture (Model-View-Update):

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
3. **update** -- take the current state and a message, mutate state, optionally return a `Command`.
4. **view** -- render the current state to a ratatui `Frame`. Called after every update.
5. **subscriptions** -- declare which long-lived event sources are active. The runtime diffs them each cycle, starting and stopping as needed.

## Coming from Bubble Tea

If you've used [Bubble Tea](https://github.com/charmbracelet/bubbletea) in Go, boba will feel familiar. Here's what maps to what, and what's different.

### Concept mapping

| Bubble Tea (Go) | boba (Rust) | Notes |
|---|---|---|
| `tea.Model` interface | `Model` trait | Same idea: `Init`, `Update`, `View` |
| `tea.Msg` (any / interface{}) | `type Message = MyEnum` | Rust enum -- the compiler catches unhandled variants |
| `tea.Cmd` | `Command<Msg>` | `Command::none()`, `Command::message()`, `Command::quit()`, `Command::perform()` |
| `tea.Batch(...)` | `Command::batch([...])` | Also `Command::sequence([...])` for ordered execution |
| `Cmd` returning a `Msg` | `Command::perform(future, map_fn)` | Backed by tokio -- full async Rust ecosystem |
| `tea.Program` | `boba::run::<MyModel>(flags)` | One function call, returns the final model |
| Bubble (sub-model) | `Component` trait | Renders into a `Rect`, has its own `Message` type, composed with `.map()` |
| lipgloss styling | ratatui `Style`, `Block`, `Layout` | ratatui's layout engine replaces lipgloss entirely |
| `tea.WithAltScreen()` | Enabled by default | Alt screen, raw mode, mouse capture are all automatic |

### Key differences

**No `interface{}` casting.** In Bubble Tea, messages are `interface{}` and you type-switch on them. In boba, `Message` is a concrete enum. If you forget to handle a variant, the compiler tells you.

**Widgets are borderless by default.** In Bubble Tea, most Bubbles render their own borders. In boba, widgets render "naked" into their full area. You wrap them with a `Block` when you want chrome:

```rust
// Boba: you control the border
let block = focus_block("Input", self.focused);
text_input.with_block(block).view(frame, area);
```

**Rendering uses ratatui, not lipgloss.** You get `Layout`, `Constraint`, `Flex`, and every ratatui widget. There's no string-based rendering -- everything is structured cells on a grid.

**Composition uses `Command::map`.** When a child component returns `Command<ChildMsg>`, you convert it to the parent's message type:

```rust
// In parent update():
Msg::Input(m) => self.input.update(m).map(Msg::Input),
```

This is the same pattern as Bubble Tea's `tea.Batch` + message wrapping, but type-safe.

**Subscriptions are declared, not started.** You return a `Vec<Subscription>` from `subscriptions()`. The runtime diffs the list and manages lifecycle. In Bubble Tea you manually send `tea.Tick` commands.

### Quick mental model

Think of boba as: **Bubble Tea + ratatui, with Rust's type system doing the work that Go's convention-based approach leaves to the developer.**

## For Rust Developers

### If you know ratatui

boba adds structure on top of ratatui. Instead of a manual `loop { terminal.draw(|f| ...) }`, you get:

- **A message loop** -- events become typed enum variants, update logic is centralized, and the framework calls `view()` for you.
- **Subscriptions** -- terminal events, timers, and custom streams are automatically managed. No manual `crossterm::read()`.
- **Commands** -- side effects (async work, quit, emit messages) are returned from `update()`, not executed inline.
- **Pre-built widgets** -- `TextInput`, `List`, `Table`, `Viewport`, etc. all implement the `Component` trait and compose with `.map()`.

You still write ratatui code in `view()`. Layouts, constraints, widgets, styles -- all the same. boba just gives you the architecture around it.

### If you know Elm / Iced / Dioxus

boba follows the Elm Architecture closely. If you've used Elm, Iced, or Dioxus, the Model-View-Update pattern is identical. Key Rust-specific things:

- `update` takes `&mut self` (mutable borrow, not a new model). No clone overhead.
- `Command::perform` takes a `Future` and a mapping function -- standard async Rust.
- `Component` is boba's nested TEA -- each component has its own `Message` type, and the parent maps with `Command::map`.
- ratatui rendering is immediate-mode: you write to a `Frame` each cycle, no retained widget tree.

### Shared primitives

If you're building custom widgets, these utilities save work:

| Utility | Purpose |
|---|---|
| `SelectionState` | Cursor + scroll offset management for any list-like widget. Wrapping navigation, page/half-page jumps, home/end. |
| `TextEditState` | Single-line text editing: char buffer, cursor, word movement, kill-line, undo/redo (100 entries). |
| `overlay::centered_rect` | Compute a centered sub-rect by percentage. |
| `overlay::centered_fixed` | Compute a centered sub-rect by fixed dimensions. |
| `overlay::render_overlay` | Clear background + optional block border for overlays. |
| `chrome::focus_block` | Bordered block with cyan (focused) / dark gray (unfocused) styling. |
| `FocusGroup` | Round-robin focus management across N components. |
| `runeutil` | Unicode display width, truncation, ANSI-to-ratatui style parsing. |

## Widgets

### Interactive components

| Widget | Description |
|---|---|
| `text_input` | Single-line input with cursor, placeholder, char limit, validation, autocomplete, undo/redo |
| `text_area` | Multi-line editor with line wrapping, selection, copy/cut/paste, word-case ops, undo/redo |
| `list` | Scrollable, filterable list with custom `Item` trait, delegated rendering, search |
| `table` | Row/column data table with column navigation, CSV import, row styling |
| `dropdown` | Overlay selector with scroll, position (above/below), and keyboard navigation |
| `select` | One-line trigger + dropdown overlay (thin wrapper around `dropdown`) |
| `filepicker` | File system browser with directory navigation and preview |
| `viewport` | Scrollable content pane with keyboard and scrollbar support |
| `modal` | Centered dialog overlay with title, body, and action buttons |
| `search` | Inline search bar with match count and navigation |

### Display components

| Widget | Description |
|---|---|
| `tabs` | Horizontal tab bar with keyboard switching |
| `spinner` | Animated spinner with configurable frame sets |
| `progress` | Determinate progress bar with gradient support |
| `timer` | Countdown timer with start, stop, reset |
| `stopwatch` | Elapsed-time stopwatch with start, stop, lap |
| `paginator` | Page indicator (dots or fraction) |
| `status_bar` | Three-section status line (left / center / right) |
| `cursor` | Blinking cursor helper for text widgets |

### Feature-gated

| Widget | Feature flag | Description |
|---|---|---|
| `code_block` | `syntax-highlighting` | Syntax-highlighted code block via syntect |
| `markdown` | `markdown` | CommonMark renderer with syntax highlighting |

### Formatting utilities

| Utility | Description |
|---|---|
| `help` | Key-binding formatter -- `short_help_line()` for status bars, `full_help_view()` for grouped overlay |

## Examples

| Example | Description | Run |
|---|---|---|
| `counter` | Minimal counter -- the "hello world" of boba | `cargo run --example counter` |
| `input_form` | Multi-field form with text inputs and focus | `cargo run --example input_form` |
| `async_http` | Async HTTP request via `Command::perform` | `cargo run --example async_http` |
| `full_app` | Multi-widget app: tabs, list, viewport, help overlay | `cargo run --example full_app` |
| `file_browser` | File system browser with preview pane | `cargo run --example file_browser` |
| `autocomplete` | TextInput + Dropdown composition pattern | `cargo run --example autocomplete` |
| `chat` | Chat UI composed from Viewport + custom rendering | `cargo run --example chat` |
| `wizard` | Multi-step wizard from Progress + state machine | `cargo run --example wizard` |

## Crate Structure

```
boba/
  crates/
    boba/            umbrella crate (add this to Cargo.toml)
    boba-core/       runtime, Model, Component, Command, Subscription, TestProgram
    boba-widgets/    18 widget components + utilities
  examples/          8 runnable examples
```

Most users only need `boba` as a dependency. It re-exports `boba_core`, `boba_widgets`, `ratatui`, `crossterm`, and `tokio`.

## License

MIT
