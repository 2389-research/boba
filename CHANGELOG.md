# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2025-01-01

Initial release of boba.

### Added

- `Model` trait implementing the Elm Architecture (`init`, `update`, `view`, `subscriptions`).
- `Component` trait for composable sub-models that render into a caller-provided `Rect`.
- Async runtime powered by tokio with `Command::future`, `Command::message`, `Command::batch`, `Command::sequence`, and `Command::quit`.
- Subscription system with automatic diffing -- subscriptions are started and stopped based on declarative diffs each update cycle.
- Built-in `terminal_events` subscription for keyboard, mouse, and resize events.
- Built-in `Every` and `After` timer subscriptions.
- `TestProgram` harness for headless model testing without a real terminal.
- `ProgramOptions` for configuring output target, mouse mode, and cursor style.
- Terminal management (alternate screen, raw mode) handled automatically by the runtime.
- 14+ widget components: text input, text area, list, table, select, tabs, spinner, progress bar, timer, stopwatch, file picker, viewport, paginator, and help bubble.
- Utility modules: cursor (blinking cursor), focus (focus group routing), key (key binding definitions), and runeutil (Unicode width and ANSI parsing).
- 5 example applications: counter, input form, async HTTP, full app, and file browser.
- Umbrella `boba` crate re-exporting core, widgets, ratatui, and crossterm.
