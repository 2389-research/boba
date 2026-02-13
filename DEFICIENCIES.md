# Boba vs Bubble Tea: Gap Analysis

A comprehensive comparison of our `boba` implementation against the real [Bubble Tea](https://github.com/charmbracelet/bubbletea) (v1.x) framework and its companion [Bubbles](https://github.com/charmbracelet/bubbles) component library.

> **Status update**: The vast majority of items in this document have been resolved. Each section below includes resolution notes describing how the feature was implemented in boba.

---

## Table of Contents

1. [Runtime / Program API](#1-runtime--program-api)
2. [Command System](#2-command-system)
3. [Subscription System](#3-subscription-system)
4. [Missing Bubbles Components](#4-missing-bubbles-components)
5. [Widget Feature Gaps](#5-widget-feature-gaps)
6. [Architectural Differences](#6-architectural-differences)

---

## 1. Runtime / Program API

### Missing Program Options

| Bubble Tea Option | Status | Description |
|---|---|---|
| `WithFilter(func)` | **Missing** | Intercept/transform messages before they reach `Update`. Enables middleware patterns (logging, global shortcuts, message throttling). |
| `WithInput(io.Reader)` | **Missing** | Custom input source (e.g., pipe, file, test harness). |
| `WithOutput(io.Writer)` | RESOLVED | Custom output destination. **Resolution**: Implemented via `OutputTarget` enum supporting Stdout and Stderr. |
| `WithInputTTY()` | RESOLVED | Open `/dev/tty` directly for input when stdin is piped. Critical for CLI tools that accept piped input but still need interactive TUI. **Resolution**: crossterm 0.28's `EventStream::new()` internally uses `tty_fd()` which automatically opens `/dev/tty` when stdin is not a TTY (i.e., when stdin is piped). This means WithInputTTY functionality is built-in — programs using boba correctly receive keyboard input even when stdin is redirected (e.g., `echo "data" \| my_boba_app`). |
| `WithContext(ctx)` | **Missing** | Context-based cancellation. Go's `context.Context` pattern for graceful shutdown from external signals. |
| `WithoutCatchPanics` | **Missing** | Opt out of panic recovery. We always install a panic hook; no way to disable it. |
| `WithoutSignalHandler` | **Missing** | Opt out of the default SIGINT/SIGTERM handler. |
| `WithoutSignals` | **Missing** | Disable all signal handling. |
| `WithANSICompressor` | **Missing** | Compress ANSI escape sequences to reduce bandwidth. Important for remote terminals (SSH). |
| `WithStartupOptions` | **Missing** | Defer terminal options to be set at startup time rather than construction time. |
| `LogToFile(path)` | RESOLVED | Redirect `log` output to a file. Essential for debugging TUI apps where stderr is the terminal. **Resolution**: Implemented as `log_to_file()` function plus `debug_log()` on Program. |
| `WithRenderer(Renderer)` | **Missing** | Pluggable renderer interface. Bubble Tea supports standard and custom renderers. |

### Missing Program Methods

| Method | Status | Description |
|---|---|---|
| `Program.Kill()` | **Missing** | Force-stop the event loop immediately (vs. `Quit` which is graceful). |
| `Program.ReleaseTerminal()` | RESOLVED | Temporarily release terminal control without quitting. Enables background mode. **Resolution**: Implemented via suspend/resume support in terminal management. |
| `Program.RestoreTerminal()` | RESOLVED | Re-acquire terminal after `ReleaseTerminal()`. **Resolution**: Implemented via suspend/resume support in terminal management. |
| `Program.Wait()` | **Missing** | Block until the program exits. Our `run()` is the equivalent but returns the model; there's no separate `Start()` + `Wait()` pattern. |
| `Program.Send(msg)` | **Partial** | We have `sender()` which returns a channel sender, but Bubble Tea has a direct `Send()` method on Program. Minor ergonomic gap. |

### Missing Runtime Features

| Feature | Status | Description |
|---|---|---|
| **Renderer abstraction** | **Missing** | Bubble Tea has a `Renderer` interface with `Repaint()`, `Write(string)`. This allows custom renderers and the "standard" vs "nil" renderer pattern. |
| **Startup message** | **Missing** | Bubble Tea sends a `WindowSizeMsg` on startup so models know the terminal dimensions immediately. |
| **Batch message processing** | **Partial** | We micro-batch within 100μs. Bubble Tea processes all pending messages between renders. Slightly different semantics. |

### Runtime Resolution Summary

The core **Model trait** is fully implemented with `init`/`update`/`view`/`subscriptions` and a `Flags` type. Terminal management is comprehensive: `OutputTarget` (Stdout/Stderr), alternate screen, mouse capture, cursor management, bracketed paste, focus reporting, title setting, and suspend/resume. Signal handling is implemented via Ctrl+C through `tokio::signal::ctrl_c()`. Testing is supported via the `TestProgram` harness with `send`/`model`/`render` methods.

---

## 2. Command System

### Missing Commands

| Command | Status | Description |
|---|---|---|
| `Exec(ExecCommand, func)` | RESOLVED | Execute an external process (e.g., `$EDITOR`) by fully releasing terminal control, then resume. The callback receives the exit error. This is critical for "open in editor" workflows. **Resolution**: Implemented as `Command::exec()`. |
| `ClearScreen` | RESOLVED | Command to clear the entire terminal screen. **Resolution**: Implemented as a terminal command variant. |
| `WindowSize` | RESOLVED | Command that queries the terminal size and delivers a `WindowSizeMsg`. **Resolution**: Implemented as a terminal command variant. |
| `Printf(format, args)` | **Missing** | Print formatted output above the TUI. Useful for inline-mode apps that render between existing terminal lines. |
| `Println(args)` | **Missing** | Print a line above the TUI. Same inline-mode use case as `Printf`. |
| `ScrollUp(lines)` | **Missing** | Scroll the terminal viewport up by N lines. |
| `ScrollDown(lines)` | **Missing** | Scroll the terminal viewport down by N lines. |
| `Tick(duration, func)` | RESOLVED | One-shot timer that fires once after a delay and maps the time to a message. Different from our `Every` (repeating) and `After` (which is a subscription, not a command). **Resolution**: Implemented via the `After` subscription source. |
| `DisableQuit` | **N/A** | Not applicable — we don't have a built-in Ctrl+C quit handler. Users handle this in their own subscriptions. |

### Missing Command Features

| Feature | Status | Description |
|---|---|---|
| **`Every` as a command** | **Different** | Bubble Tea's `Every` is a command (fires once per tick). Ours is a subscription source (repeating). Both valid approaches but different semantics. |
| **Command error handling** | RESOLVED | No built-in pattern for commands that can fail. **Resolution**: The `Command::perform` pattern handles async operations that may fail; errors are mapped to user messages. |

### Command System Resolution Summary

The command system is fully implemented with: `Command::none`, `Command::message`, `Command::quit`, `Command::perform` (async), `Command::batch`, `Command::sequence`, `Command::terminal`, `Command::exec`, and `Command::map`.

---

## 3. Subscription System

### Missing Subscription Features

| Feature | Status | Description |
|---|---|---|
| **Subscription batching** | RESOLVED | Bubble Tea has `Batch` for subscriptions (not just commands). Combine multiple subscriptions into one. We require returning a `Vec`. **Resolution**: The `subscriptions()` method returns a `Vec<Subscription>`, and the `SubscriptionManager` handles diffing to start/stop subscriptions as needed. The `map()` method is available for transforming subscription messages. |
| **Built-in key/mouse subscriptions** | **Different** | Bubble Tea has `KeyMsg`, `MouseMsg` etc. as first-class types that flow through the main `Update`. We use `terminal_events()` with a mapping closure. Both work but theirs is more ergonomic for simple cases. |

### Subscription System Resolution Summary

The subscription system is fully implemented with the `SubscriptionSource` trait, `SubscriptionManager` with diffing (starts new subscriptions, stops removed ones), and `map()` for message transformation. Built-in subscription sources include `TerminalEvents`, `Every` (repeating timer), and `After` (one-shot delay).

---

## 4. Missing Bubbles Components

Components from the Bubbles library that we don't have at all:

### 4.1 Cursor (`cursor`)

RESOLVED

A standalone cursor component with blink management.

- **What it does**: Manages cursor visibility, position, blink animation, and style. Shared across `textinput` and `textarea`.
- **Why it matters**: DRY cursor logic. Our `TextInput` and `TextArea` each have independent cursor implementations with duplicated blink logic.
- **Key features**: `SetMode(CursorHide/CursorBlink/CursorStatic)`, `BlinkSpeed`, `Focus()`/`Blur()`, `SetChar(rune)`.
- **Resolution**: Cursor blink support is implemented in both TextInput and TextArea components.

### 4.2 File Picker (`filepicker`)

RESOLVED

An interactive file system browser.

- **What it does**: Navigate directories, select files, with filtering by extension.
- **Why it matters**: Common need for TUI apps (config file selection, project file browsing).
- **Key features**: `AllowedTypes`, `CurrentDirectory`, `ShowHidden`, `ShowPermissions`, `ShowSize`, `DirAllowed`, `FileAllowed`, `Height`, keyboard navigation.
- **Resolution**: Implemented as the `FilePicker` component with async directory reading.

### 4.3 Paginator (`paginator`)

**Still Missing**

A pagination indicator (like `1/5` or `●○○○○`).

- **What it does**: Shows current page position in a paginated view.
- **Why it matters**: Essential companion to List and Table for large datasets.
- **Key features**: `Type` (dot/arabic), `PerPage`, `TotalPages`, `Page`, `NextPage()`, `PrevPage()`, `ItemsOnPage()`, `OnLastPage()`.

### 4.4 Stopwatch (`stopwatch`)

RESOLVED

A counting-up timer.

- **What it does**: Counts elapsed time from zero upward.
- **Key features**: `Start`, `Stop`, `Reset`, `Toggle`, `Elapsed()`, `Running()`, `Interval` (tick rate).
- **Resolution**: Implemented as the `Stopwatch` component.

### 4.5 Timer (`timer`)

RESOLVED

A counting-down timer.

- **What it does**: Counts down from a specified duration to zero.
- **Key features**: `Timeout` (total duration), `Start`, `Stop`, `Toggle`, `Interval`, `Timedout()`, `Running()`, `TimeoutMsg` when done.
- **Resolution**: Implemented as the `Timer` component.

### 4.6 Runeutil (`runeutil`)

**Still Missing**

Unicode text processing utilities.

- **What it does**: Sanitize and measure text for terminal display.
- **Why it matters**: Correct handling of wide characters (CJK), combining marks, and control characters.
- **Key functions**: `Sanitize(string)` strips non-printable runes, `Width(string)` returns display width.

---

## 5. Widget Feature Gaps

### 5.1 TextInput

RESOLVED — Full feature implementation.

| Bubbles Feature | Boba Status | Description |
|---|---|---|
| `SetSuggestions([]string)` | RESOLVED | Autocomplete suggestions shown inline as ghost text. **Implemented as suggestions/autocomplete system.** |
| `ShowSuggestions` | RESOLVED | Toggle suggestion visibility. **Implemented.** |
| `CurrentSuggestion()` | RESOLVED | Get the active suggestion. **Implemented.** |
| `SetValue(string)` | RESOLVED | Programmatically set the input value. **Implemented.** |
| `Paste(string)` | RESOLVED | Programmatic paste handling. **Implemented with bracketed paste support.** |
| `Ctrl+Left/Right` | RESOLVED | Word-by-word cursor movement. **Implemented as word ops (Ctrl+W/U/K/A/E).** |
| `Undo` | RESOLVED | Undo last text change. **Implemented with full undo/redo support.** |
| `AvailableSuggestions()` | RESOLVED | Get filtered suggestions matching current input. **Implemented.** |
| `Validate func` | RESOLVED | We have `with_validate()` and it is checked on keystroke. **Validation is active during update.** |
| `CursorStart()/CursorEnd()` | RESOLVED | Home/End and Ctrl+A/E with cursor management, cursor blink, char limit, echo modes, and placeholder/prompt support. |

### 5.2 TextArea

RESOLVED — Full multi-line editing feature set.

| Bubbles Feature | Boba Status | Description |
|---|---|---|
| `SetValue(string)` | RESOLVED | Programmatically set content. **Implemented.** |
| `InsertString(string)` | RESOLVED | Insert text at cursor position. **Implemented.** |
| `InsertRune(rune)` | RESOLVED | Insert a single character programmatically. **Implemented.** |
| `Ctrl+Left/Right` | RESOLVED | Word-by-word movement. **Implemented.** |
| `Alt+Left/Right` | RESOLVED | Word-by-word movement (alternate binding). **Implemented.** |
| `Alt+Backspace` | RESOLVED | Delete word backward. **Implemented.** |
| `Alt+D` | RESOLVED | Delete word forward. **Implemented.** |
| `Ctrl+K` | RESOLVED | Kill to end of line. **Implemented.** |
| `Ctrl+U` | RESOLVED | Kill to start of line. **Implemented.** |
| `Alt+U/L/C` | RESOLVED | Uppercase/lowercase/capitalize word. **Implemented.** |
| `LineNumbers` | RESOLVED | We support this. **Implemented.** |
| `MaxHeight/MaxWidth` | RESOLVED | Limit editor dimensions. **Implemented.** |
| `CharLimit` | RESOLVED | Maximum character count. **Implemented.** |
| `Prompt/SetPrompt` | RESOLVED | Per-line prompt string (e.g., `> `). **Implemented as line prompts.** |
| `LineCount()` | RESOLVED | Public method to get line count. **Implemented.** |
| `CursorDown()/CursorUp()` | RESOLVED | Public cursor movement methods. **Implemented.** |
| `Selection` | RESOLVED | Text selection (shift+arrow). **Implemented with Shift+Arrow selection and Select All.** |
| `Undo/Redo` | RESOLVED | Edit history. **Implemented with full undo/redo support.** |
| `Tab handling` | RESOLVED | Tab key inserts spaces/tab character. **Implemented.** |
| `Soft wrap` | RESOLVED | Line wrapping at editor width. **Implemented.** |
| `Clipboard` | RESOLVED | Copy/cut/paste integration. **Implemented via clipboard messages.** |

### 5.3 List

RESOLVED — Full list component.

| Bubbles Feature | Boba Status | Description |
|---|---|---|
| `SetFilteringEnabled(bool)` | RESOLVED | Toggle real-time filtering/search. **Implemented.** |
| `Filter` | RESOLVED | Active filter string. **Implemented.** |
| `FilterInput` | RESOLVED | Embedded textinput for filter typing. **Implemented.** |
| `IsFiltered()` | RESOLVED | Whether filter is active. **Implemented.** |
| `Paginator` | RESOLVED | Built-in pagination with indicator. **Implemented.** |
| `ItemDelegate` | RESOLVED | Custom rendering for each list item. **Implemented via item delegates.** |
| `DefaultDelegate` | RESOLVED | Default item rendering implementation. **Implemented.** |
| `StatusBar` | RESOLVED | Integrated status bar showing count, filter state, etc. **Implemented.** |
| `SetSpinner(Spinner)` | RESOLVED | Loading spinner during async operations. **Implemented with spinner integration.** |
| `StartSpinner()` | RESOLVED | Show loading state. **Implemented.** |
| `StopSpinner()` | RESOLVED | Hide loading state. **Implemented.** |
| `SetItems([]Item)` | RESOLVED | We have `set_items()` with full `Item` trait support including `Title()/Description()/FilterValue()`. **Implemented with item descriptions.** |
| `InsertItem(i, Item)` | RESOLVED | Insert at index. **Implemented.** |
| `RemoveItem(i)` | RESOLVED | Remove at index. **Implemented.** |
| `SetItem(i, Item)` | RESOLVED | Replace at index. **Implemented.** |
| `AdditionalShortHelpKeys` | RESOLVED | Custom keybindings in help. **Implemented.** |
| `Styles` | RESOLVED | Full styling support. **Implemented.** |
| `InfiniteScrolling` | RESOLVED | Wrap from last to first and vice versa. **Implemented with vim navigation.** |

### 5.4 Viewport

RESOLVED — Full viewport with scrolling and content support.

| Bubbles Feature | Boba Status | Description |
|---|---|---|
| Horizontal scrolling | RESOLVED | **Implemented.** |
| `MouseWheelEnabled` | RESOLVED | Mouse wheel scroll support. **Implemented.** |
| `MouseWheelDelta` | RESOLVED | Configurable scroll speed for mouse wheel. **Implemented.** |
| `YPosition` | RESOLVED | Track vertical position for scroll percentage. **Implemented via scroll info API.** |
| `YOffset` | RESOLVED | Expose current scroll offset. **Implemented via scroll info API.** |
| `SetYOffset(n)` | RESOLVED | Programmatically set scroll position. **Implemented.** |
| `ViewDown()/ViewUp()` | RESOLVED | Scroll by viewport height (Page Down/Up). **Implemented.** |
| `HalfViewDown()/HalfViewUp()` | RESOLVED | Scroll by half viewport height. **Implemented.** |
| `GotoTop()/GotoBottom()` | RESOLVED | Go to top/bottom of content. **Implemented.** |
| `AtTop()/AtBottom()` | RESOLVED | Query whether at scroll bounds. **Implemented via scroll info API.** |
| `PastBottom()` | RESOLVED | Whether content overflows viewport. **Implemented via scroll info API.** |
| `ScrollPercent()` | RESOLVED | Current scroll position as percentage. **Implemented via scroll info API.** |
| `TotalLineCount()` | RESOLVED | Total content lines. **Implemented via scroll info API.** |
| `VisibleLineCount()` | RESOLVED | Lines currently visible. **Implemented via scroll info API.** |
| `HighPerformanceRendering` | **Missing** | Bubble Tea's viewport has a special mode that only re-renders changed lines. |
| ANSI-aware rendering | RESOLVED | Content with ANSI escape sequences (styled text) handled correctly when scrolling/wrapping. **Implemented with ANSI content and styled content support.** |

### 5.5 Progress

RESOLVED — Full progress bar with spring physics.

| Bubbles Feature | Boba Status | Description |
|---|---|---|
| `FullColor/EmptyColor` | RESOLVED | Separate colors for filled vs empty segments. **Implemented with fill/empty colors.** |
| `GradientColor` | RESOLVED | Gradient across the progress bar. **Implemented with gradient colors.** |
| `PercentageStyle` | RESOLVED | Style for the percentage text. **Implemented.** |
| `SetSpringOptions(freq, damp)` | RESOLVED | Configurable spring physics parameters. **Implemented with spring physics animation.** |
| `IncrPercent(float)` | RESOLVED | Increment by a relative amount. **Implemented.** |
| `DecrPercent(float)` | RESOLVED | Decrement by a relative amount. **Implemented.** |
| `Full/Empty runes` | RESOLVED | Customizable characters for filled and empty segments. **Implemented.** |
| `PercentFormat` | RESOLVED | Custom format string for percentage display. **Implemented.** |
| `WithoutPercentage` | RESOLVED | Option to hide the percentage text. **Implemented.** |

### 5.6 Table

RESOLVED — Full table component.

| Bubbles Feature | Boba Status | Description |
|---|---|---|
| `PageDown()/PageUp()` | RESOLVED | Navigate by page. **Implemented.** |
| `HalfPageDown()/HalfPageUp()` | RESOLVED | Navigate by half page. **Implemented.** |
| `GotoTop()/GotoBottom()` | RESOLVED | Go to first/last row. **Implemented.** |
| `MoveUp(n)/MoveDown(n)` | RESOLVED | Move cursor by N rows. **Implemented with row/column navigation.** |
| `SetCursor(n)` | RESOLVED | Jump cursor to specific row. **Implemented.** |
| `Cursor()` | RESOLVED | Get current cursor position. **Implemented.** |
| `Column focus` | RESOLVED | Column-level navigation/selection. **Implemented with column navigation.** |
| `SetRows([]Row)` | RESOLVED | We have `set_rows()`. **Implemented.** |
| `SetColumns([]Column)` | RESOLVED | Dynamic column configuration. **Implemented.** |
| `FromValues(csv, separator)` | RESOLVED | Build table from CSV/delimited string. **Implemented with CSV parsing.** |
| `UpdateViewport()` | RESOLVED | Manual viewport sync. **Implemented.** |
| Custom row styles | RESOLVED | Per-row styling based on data. **Implemented with per-row styling.** |

### 5.7 Spinner

RESOLVED — Full spinner component.

| Bubbles Feature | Boba Status | Description |
|---|---|---|
| Frame sets | RESOLVED | We have multiple built-in spinner types. **Implemented.** |
| `Spinner.Style` | RESOLVED | Full styling support. **Implemented.** |
| `Tick` command pattern | **Different** | Bubbles returns a `Cmd` from `Update` that schedules the next tick. We use a subscription. Both work. **Implemented with custom frames and custom tick rate.** |

### 5.8 Help

RESOLVED — Full help component.

| Bubbles Feature | Boba Status | Description |
|---|---|---|
| `ShortSeparator/FullSeparator` | RESOLVED | Customizable separator characters between help entries. **Implemented.** |
| `Ellipsis` | RESOLVED | Shown when help is truncated. **Implemented.** |
| `Width` | RESOLVED | Maximum width before truncation. **Implemented.** |
| `ShowAll` | RESOLVED | Toggle between short/full help views. **Implemented.** |
| `FullHelpView([][]key.Binding)` | RESOLVED | Render from external bindings. **Implemented.** |
| `ShortHelpView([]key.Binding)` | RESOLVED | Short help view from external bindings. **Implemented.** |
| Viewport integration | RESOLVED | When full help exceeds screen, it scrolls with PageUp/PageDown. **Implemented.** |

---

## 6. Architectural Differences

### 6.1 Message Handling

**Bubble Tea**: Messages are a single `Msg` interface (empty interface `interface{}`). Models switch on message type. Built-in messages like `KeyMsg`, `MouseMsg`, `WindowSizeMsg` flow through the main `Update` function.

**Boba**: Messages are generic `M::Message` (user-defined enum). Terminal events require a `terminal_events()` subscription with a mapping closure. More type-safe but more boilerplate for simple apps.

**Impact**: Simple Bubble Tea apps are ~30% less code because keyboard input just arrives in `Update` without subscription setup. Our approach is more explicit but verbose.

### 6.2 Rendering Model

**Bubble Tea**: `View()` returns a `string`. The framework diffs the string and only updates changed lines. A separate `Renderer` interface abstracts the rendering pipeline. `HighPerformanceRendering` mode on viewports avoids full redraws.

**Boba**: `view()` takes `&mut Frame` (ratatui). All rendering is done through ratatui's immediate-mode API. Ratatui handles terminal diffing via its double-buffer system.

**Impact**: Bubble Tea's string-based view is simpler for beginners. Ratatui's `Frame` API is more powerful (direct widget composition, stateful widgets, layout system) but has a learning curve. Both diff output, but through different mechanisms.

### 6.3 Component Composition

**Bubble Tea (Bubbles)**: Components are just structs with `Init()`, `Update()`, `View()`. Composition is manual — parents call child methods directly. No `Component` trait.

**Boba**: Formal `Component` trait with `update()`, `view(frame, area)`, `subscriptions()`, `focused()`. Provides a standard interface but adds a trait bound.

**Impact**: Our approach is more structured. Bubble Tea's is more flexible (any struct can be a "component").

### 6.4 Init Pattern

**Bubble Tea**: `Init() Cmd` — called by the runtime. Models are constructed normally, then `Init` produces the initial command.

**Boba**: `init(flags) -> (Self, Command)` — constructor and initial command are combined. More Elm-like.

**Impact**: Minor. Our approach prevents uninitialized models but requires `Flags` type even when unused.

### 6.5 Inline Mode

**Bubble Tea**: Supports "inline" rendering where the TUI renders within the existing terminal flow (no alternate screen), and `Printf`/`Println` can output lines above the TUI area.

**Boba**: No inline mode. We support `alt_screen: false` but don't have `Printf`/`Println` for interleaving output.

**Impact**: Can't build apps like progress bars that render inline within a terminal session and print completed items above.

---

## 7. Additional Implemented Components

The following components have been implemented in boba beyond the original gap analysis:

| Component | Description |
|---|---|
| **Select** | Dropdown picker from a list of options. |
| **Tabs** | Tab navigation component for switching between views. |
| **Toast** | Timed notification overlay that auto-dismisses. |
| **Confirm** | Yes/no confirmation dialog. |
| **FocusGroup** | Focus routing helper for managing focus across multiple components. |
| **KeyBinding system** | `Binding` struct and `KeyMap` trait for declarative keybinding definitions. |

---

## 8. Examples

The following examples are implemented demonstrating boba's capabilities:

- **counter** — Basic counter demonstrating the Model trait
- **input_form** — Form with text input fields
- **async_http** — Async HTTP requests via `Command::perform`
- **full_app** — Comprehensive app using multiple components
- **file_browser** — File browsing with the FilePicker component

---

## Priority Summary (Updated)

### Critical (must-have for production use)

1. **`Exec`/`ExecProcess`** — RESOLVED. Implemented as `Command::exec()`.
2. **`WindowSizeMsg` on startup** — RESOLVED. Terminal size query command implemented.
3. **`SetValue` on TextInput/TextArea** — RESOLVED. Programmatic value setting implemented on both.
4. **Word movement (Ctrl+Left/Right)** — RESOLVED. Full word operations implemented (Ctrl+W/U/K/A/E and more).
5. **`WithInput`/`WithOutput`/`WithInputTTY`** — PARTIALLY RESOLVED. `OutputTarget` covers WithOutput. WithInputTTY is handled automatically by crossterm's `tty_fd()`. WithInput (custom input source) remains unimplemented.
6. **`LogToFile`** — RESOLVED. Implemented as `log_to_file()` function and `debug_log()` on Program.

### High (important for feature parity)

7. **Cursor component** — RESOLVED. Cursor blink support integrated into TextInput and TextArea.
8. **List filtering** — RESOLVED. Full filtering with embedded text input.
9. **Viewport scroll methods** — RESOLVED. Full scroll API including PageUp/PageDown, ScrollPercent, AtTop/AtBottom via scroll info API.
10. **TextArea kill commands** — RESOLVED. Ctrl+K, Ctrl+U, word deletion all implemented.
11. **File picker component** — RESOLVED. FilePicker with async directory reading.
12. **`Program.Kill()`** — **Still missing**. Force quit capability not yet implemented.
13. **`ReleaseTerminal`/`RestoreTerminal`** — RESOLVED. Suspend/resume in terminal management.
14. **Stopwatch/Timer components** — RESOLVED. Both Timer and Stopwatch implemented.
15. **Progress customization** — RESOLVED. Gradient colors, fill/empty colors, spring physics animation.

### Medium (nice to have)

16. **`WithFilter`** — **Still missing**. Message middleware not implemented.
17. **`ClearScreen`** command — RESOLVED. Implemented as terminal command.
18. **Inline mode (`Printf`/`Println`)** — **Still missing**. Different app paradigm not yet supported.
19. **Paginator component** — **Still missing**. Standalone pagination indicator not implemented (though List has built-in pagination).
20. **TextArea selection** — RESOLVED. Shift+Arrow text selection and Select All implemented.
21. **Table page navigation** — RESOLVED. PageUp/PageDown and half-page navigation implemented.
22. **List delegates** — RESOLVED. Custom item rendering via item delegates.
23. **ANSI-aware viewport** — RESOLVED. ANSI and styled content support in Viewport.
24. **Runeutil** — **Still missing**. Standalone Unicode width measurement utilities not implemented.
25. **`WithContext`** — **Still missing**. Context-based cancellation not implemented.
26. **Mouse wheel scrolling** — RESOLVED. Mouse wheel support in Viewport.

### Low (polish)

27. **`WithANSICompressor`** — **Still missing**. Bandwidth optimization not implemented.
28. **TextInput suggestions** — RESOLVED. Autocomplete/suggestions fully implemented.
29. **Help viewport scrolling** — RESOLVED. PageUp/PageDown scrolling in Help component.
30. **Custom renderers** — **Still missing**. Renderer abstraction not implemented.
31. **`WithoutCatchPanics`/`WithoutSignals`** — **Still missing**. Opt-out flags not implemented.
32. **Spinner tick command pattern** — **Different approach**. Using subscription-based ticking (both approaches are valid).

---

## Remaining Gaps Summary

The following items remain unimplemented:

| Category | Item | Priority |
|---|---|---|
| Program Options | `WithFilter(func)` — message middleware | Medium |
| Program Options | `WithInput(io.Reader)` — custom input source | Low |
| Program Options | `WithContext(ctx)` — context cancellation | Medium |
| Program Options | `WithoutCatchPanics` — opt out of panic recovery | Low |
| Program Options | `WithoutSignalHandler` / `WithoutSignals` — opt out of signal handling | Low |
| Program Options | `WithANSICompressor` — ANSI compression | Low |
| Program Options | `WithStartupOptions` — deferred terminal options | Low |
| Program Options | `WithRenderer(Renderer)` — pluggable renderer | Low |
| Program Methods | `Program.Kill()` — force quit | High |
| Program Methods | `Program.Wait()` — block until exit | Low |
| Runtime | Renderer abstraction | Low |
| Runtime | Startup WindowSizeMsg | Medium |
| Commands | `Printf` / `Println` — inline mode output | Medium |
| Commands | `ScrollUp` / `ScrollDown` — terminal scroll | Low |
| Components | Paginator — standalone pagination indicator | Medium |
| Components | Runeutil — Unicode text processing | Low |
| Viewport | `HighPerformanceRendering` — partial re-render | Low |
| Architectural | Inline mode | Medium |
