# Widget Composability Restructure — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make all boba widgets composable building blocks by removing forced borders, extracting shared primitives, and eliminating redundant widgets.

**Architecture:** Bottom-up refactor in 5 layers. Each layer is independently shippable. Layer 0 (borderless defaults) is the highest-impact change. Subsequent layers extract shared code and remove duplication.

**Tech Stack:** Rust, ratatui 0.30, crossterm 0.28, tokio

---

## Layer 0: Borderless Defaults

### Task 1: Add `chrome.rs` utility module

**Files:**
- Create: `crates/boba-widgets/src/chrome.rs`
- Modify: `crates/boba-widgets/src/lib.rs`

**Step 1: Create chrome.rs**

```rust
//! Convenience helpers for common widget chrome patterns.

use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders};

/// Create a bordered block with focus-dependent border color.
///
/// Uses cyan when focused, dark gray when unfocused. Suitable as a
/// default chrome for any widget.
pub fn focus_block(title: &str, focused: bool) -> Block<'_> {
    let color = if focused { Color::Cyan } else { Color::DarkGray };
    Block::new()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(color))
}
```

**Step 2: Add `pub mod chrome;` to lib.rs**

In `crates/boba-widgets/src/lib.rs`, add `pub mod chrome;` alongside other module declarations.

**Step 3: Run `cargo build` — expect clean**

**Step 4: Commit** — `feat: add chrome utility module with focus_block helper`

---

### Task 2: Make TextInput borderless

**Files:**
- Modify: `crates/boba-widgets/src/text_input.rs`

**Step 1: Remove border fields from TextInputStyle**

In the `TextInputStyle` struct (lines 29-46), remove:
- `focused_border: Style`
- `unfocused_border: Style`
- `error_border: Style`

Remove those same fields from `Default for TextInputStyle`.

**Step 2: Add `block` field to TextInput struct**

Add `block: Option<Block<'static>>` to the TextInput struct fields. Initialize to `None` in `new()`.

**Step 3: Add `with_block` builder**

```rust
pub fn with_block(mut self, block: Block<'static>) -> Self {
    self.block = Some(block);
    self
}
```

**Step 4: Update view() block logic**

Replace the block creation in view() (lines 676-678) — currently:
```rust
let block = Block::default()
    .borders(Borders::ALL)
    .border_style(border_style);
```

With:
```rust
let inner = if let Some(ref block) = self.block {
    let inner = block.inner(area);
    frame.render_widget(block.clone(), area);
    inner
} else {
    area
};
```

Remove the old `border_style` variable computation and the separate `frame.render_widget(block, area)` call. Use `inner` for all subsequent rendering.

**Step 5: Fix any tests that reference the removed style fields**

Search for `focused_border`, `unfocused_border`, `error_border` in text_input.rs tests and remove/update.

**Step 6: Run `cargo test -p boba-widgets` — expect all pass**

**Step 7: Commit** — `refactor: make TextInput borderless by default`

---

### Task 3: Make TextArea borderless

**Files:**
- Modify: `crates/boba-widgets/src/text_area.rs`

Same pattern as Task 2. TextAreaStyle (lines 69-82) — remove `focused_border` and `unfocused_border`. Add `block: Option<Block<'static>>` field to TextArea, `with_block()` builder. Update view() (lines 976-980) to use optional block pattern.

**Commit** — `refactor: make TextArea borderless by default`

---

### Task 4: Make List borderless

**Files:**
- Modify: `crates/boba-widgets/src/list.rs`

ListStyle (lines 280-291) — remove `focused_border` and `unfocused_border`. Add `block: Option<Block<'static>>` to List struct. Add `with_block()`. Update view() (lines 811-813) to use optional block pattern. The inner area computation feeds into the layout constraints for spinner/filter/items/status sections.

**Commit** — `refactor: make List borderless by default`

---

### Task 5: Make Table borderless

**Files:**
- Modify: `crates/boba-widgets/src/table.rs`

TableStyle (lines 161-176) — remove `focused_border` and `unfocused_border`. Add `block: Option<Block<'static>>` to Table struct. Add `with_block()`. Update view() (lines 537-543). Note: Table currently passes the block to `ratatui::widgets::Table::block()` — instead, render the block ourselves and pass content to inner area, OR pass `self.block.clone()` to the ratatui Table. The latter is simpler since ratatui's Table already handles optional blocks natively.

**Commit** — `refactor: make Table borderless by default`

---

### Task 6: Make Viewport borderless

**Files:**
- Modify: `crates/boba-widgets/src/viewport.rs`

ViewportStyle (lines 164-171) — remove `border` and `focused_border` (keep `scrollbar`). Add `block: Option<Block<'static>>` to Viewport struct. Add `with_block()`. Update view() (lines 474-478).

**Commit** — `refactor: make Viewport borderless by default`

---

### Task 7: Make Select borderless

**Files:**
- Modify: `crates/boba-widgets/src/select.rs`

SelectStyle (lines 42-55) — remove `focused_border`, `unfocused_border`, `dropdown_border`. Add `block: Option<Block<'static>>` and `dropdown_block: Option<Block<'static>>` to Select. Add `with_block()` and `with_dropdown_block()`. Update view() — trigger block (lines 196-202) and dropdown block (lines 240-244).

**Commit** — `refactor: make Select borderless by default`

---

### Task 8: Make remaining widgets borderless

Apply the same pattern to each. One commit per widget:

- **FilePicker** (lines 388-394) — remove `border` and `focused_border` from FilePickerStyle
- **Help overlay** (lines 334-338) — remove `border` from HelpStyle, add optional block for the overlay panel
- **Tabs** (lines 147-151, uses `Borders::BOTTOM`) — remove `border` from TabsStyle
- **Progress** (lines 283-290) — remove `border` from ProgressStyle. Note: Progress passes block to Gauge — same pattern as Table, pass `self.block.clone()` to the ratatui Gauge
- **Modal** (lines 339-343) — remove `border` from ModalStyle, add optional block
- **Dropdown** (lines 268-275) — remove `border` from DropdownStyle, add optional block
- **Chat** (lines 461-463) — remove `focused_border`, `unfocused_border`, `locked_border` from ChatStyle
- **Wizard** (lines 261-265) — remove `border` from WizardStyle
- **Autocomplete dropdown** (lines 343-347) — remove `dropdown_border` from AutocompleteStyle

**Commit each** — `refactor: make <Widget> borderless by default`

---

### Task 9: Update all examples

**Files:**
- Modify: `examples/counter.rs`
- Modify: `examples/input_form.rs`
- Modify: `examples/full_app.rs`
- Modify: `examples/file_browser.rs`
- Modify: `examples/async_http.rs`

Every example that uses a bordered widget needs to call `with_block()` or use `focus_block()`. The pattern:

```rust
use boba::widgets::chrome::focus_block;

// In view():
let block = focus_block("Items", self.list.focused());
let inner = block.inner(area);
frame.render_widget(block, area);
self.list.view(frame, inner);
```

**Run all examples**: `cargo build --examples` — expect clean.

**Commit** — `refactor: update examples for borderless widgets`

---

### Task 10: Layer 0 verification

Run full verification suite:
```
cargo build
cargo test
cargo clippy --all-targets
cargo doc --no-deps
cargo build --examples
```

All must pass cleanly. Commit any fixups.

---

## Layer 1: Extract Shared Primitives

### Task 11: Extract `selection.rs`

**Files:**
- Create: `crates/boba-widgets/src/selection.rs`
- Modify: `crates/boba-widgets/src/lib.rs`

Extract the navigation logic duplicated across List (lines 565-639), Table (lines 329-427), Select, Dropdown, and Autocomplete.

```rust
//! Shared selectable-list navigation state.

/// Tracks cursor position and scroll offset for a selectable collection.
pub struct SelectionState {
    cursor: usize,
    offset: usize,
    count: usize,
    visible: usize,
}

impl SelectionState {
    pub fn new(count: usize, visible: usize) -> Self { ... }
    pub fn cursor(&self) -> usize { self.cursor }
    pub fn offset(&self) -> usize { self.offset }
    pub fn set_count(&mut self, count: usize) { ... }
    pub fn set_visible(&mut self, visible: usize) { ... }
    pub fn move_up(&mut self) { ... }       // wraps
    pub fn move_down(&mut self) { ... }     // wraps
    pub fn page_up(&mut self) { ... }
    pub fn page_down(&mut self) { ... }
    pub fn half_page_up(&mut self) { ... }
    pub fn half_page_down(&mut self) { ... }
    pub fn home(&mut self) { ... }
    pub fn end(&mut self) { ... }
    pub fn select(&mut self, index: usize) { ... }
    fn ensure_visible(&mut self) { ... }    // adjusts offset
}
```

**Step 1: Write tests first**

Test each method: move_up wraps from 0, move_down wraps from last, page_up/down clamps, home/end go to extremes, ensure_visible keeps cursor in viewport.

**Step 2: Implement SelectionState**

**Step 3: Run tests — expect pass**

**Step 4: Commit** — `feat: add shared SelectionState for list navigation`

---

### Task 12: Migrate List to use SelectionState

**Files:**
- Modify: `crates/boba-widgets/src/list.rs`

Replace `select_next()`, `select_prev()`, `select_first()`, `select_last()`, `select_page_down()`, `select_page_up()`, `select_half_page_down()`, `select_half_page_up()` (lines 565-639) with calls to a `SelectionState` field. The `filtered_indices` mapping still happens in List — SelectionState tracks the position within the filtered view.

**Run `cargo test -p boba-widgets` — expect all existing List tests pass**

**Commit** — `refactor: use SelectionState in List`

---

### Task 13: Migrate Table, Select, Dropdown to SelectionState

Same pattern as Task 12 for each widget. One commit per widget.

- Table: Replace navigation methods (lines 329-427) — note Table has column navigation too, which stays widget-specific
- Select: Replace Up/Down handling (lines 136-150)
- Dropdown: Replace navigation + scroll offset (lines 182-238)

**Commit each** — `refactor: use SelectionState in <Widget>`

---

### Task 14: Extract `text_edit.rs`

**Files:**
- Create: `crates/boba-widgets/src/text_edit.rs`
- Modify: `crates/boba-widgets/src/lib.rs`

Extract the single-line text editing core from TextInput. The canonical implementation is already in TextInput (lines 265-441). Extract into a standalone struct:

```rust
//! Shared single-line text editing state.

use std::collections::VecDeque;

pub struct TextEditState {
    chars: Vec<char>,
    cursor: usize,
    undo_stack: VecDeque<(Vec<char>, usize)>,
    redo_stack: VecDeque<(Vec<char>, usize)>,
}

impl TextEditState {
    pub fn new() -> Self { ... }
    pub fn value(&self) -> String { ... }
    pub fn set_value(&mut self, s: &str) { ... }
    pub fn cursor(&self) -> usize { ... }
    pub fn len(&self) -> usize { ... }
    pub fn is_empty(&self) -> bool { ... }
    pub fn insert_char(&mut self, c: char) { ... }
    pub fn delete_back(&mut self) { ... }
    pub fn delete_forward(&mut self) { ... }
    pub fn move_left(&mut self) { ... }
    pub fn move_right(&mut self) { ... }
    pub fn move_home(&mut self) { ... }
    pub fn move_end(&mut self) { ... }
    pub fn word_left(&mut self) { ... }
    pub fn word_right(&mut self) { ... }
    pub fn delete_word_back(&mut self) { ... }
    pub fn delete_word_forward(&mut self) { ... }
    pub fn kill_to_start(&mut self) { ... }
    pub fn kill_to_end(&mut self) { ... }
    pub fn undo(&mut self) { ... }
    pub fn redo(&mut self) { ... }
    pub fn reset(&mut self) { ... }
    fn push_undo(&mut self) { ... }
}
```

**Step 1: Write tests** — port existing TextInput unit tests that exercise cursor/editing/undo
**Step 2: Implement** — extract from TextInput's methods
**Step 3: Run tests — expect pass**
**Step 4: Commit** — `feat: add shared TextEditState for single-line editing`

---

### Task 15: Migrate TextInput to use TextEditState

**Files:**
- Modify: `crates/boba-widgets/src/text_input.rs`

Replace internal `value: Vec<char>`, `cursor: usize`, `undo_stack`, `redo_stack` fields with a single `editor: TextEditState`. Delegate all cursor/editing methods. Keep TextInput-specific features (suggestions, validation, echo mode, history) in TextInput.

**Run `cargo test -p boba-widgets` — expect all TextInput tests pass**

**Commit** — `refactor: use TextEditState in TextInput`

---

### Task 16: Migrate Autocomplete and Search to TextEditState

**Files:**
- Modify: `crates/boba-widgets/src/autocomplete.rs`
- Modify: `crates/boba-widgets/src/search.rs`

Replace the hand-rolled cursor/editing logic in both files with `TextEditState`. Autocomplete (lines 195-286) — replace `value`, `cursor_pos`, `byte_offset()`, `char_len()` with `editor: TextEditState`. Search (lines 191-275) — same.

**Commit each** — `refactor: use TextEditState in <Widget>`

---

### Task 17: Layer 1 verification

Full verification: `cargo build && cargo test && cargo clippy --all-targets && cargo doc --no-deps && cargo build --examples`

---

## Layer 2: Merge/Kill Redundant Widgets

### Task 18: Kill autocomplete.rs — replace with TextInput + Dropdown

**Files:**
- Delete: `crates/boba-widgets/src/autocomplete.rs`
- Modify: `crates/boba-widgets/src/text_input.rs`
- Modify: `crates/boba-widgets/src/lib.rs`

TextInput gains a `with_dropdown(bool)` mode. When enabled and suggestions are set, typing opens a Dropdown below the input. TextInput already has suggestion support — this wires it to the Dropdown widget for rendering.

Move Autocomplete's tests into TextInput tests covering the dropdown behavior.

Remove `pub mod autocomplete;` from lib.rs.

**Commit** — `refactor: replace Autocomplete with TextInput dropdown mode`

---

### Task 19: Merge select.rs + dropdown.rs

**Files:**
- Delete: `crates/boba-widgets/src/select.rs`
- Modify: `crates/boba-widgets/src/dropdown.rs`
- Modify: `crates/boba-widgets/src/lib.rs`

Dropdown gains a `with_trigger(bool)` mode. When trigger mode is on, the widget renders as a one-line display (showing selected value) that opens on Enter/Space. This replaces Select's functionality.

Alternatively: keep Select as a thin composition wrapper around Dropdown. Choose whichever is simpler — the key is one selection/navigation implementation.

Remove `pub mod select;` from lib.rs (or keep it as a re-export type alias).

Port Select's tests to Dropdown or the merged module.

**Commit** — `refactor: merge Select into Dropdown`

---

### Task 20: Move chat.rs to examples

**Files:**
- Delete: `crates/boba-widgets/src/chat.rs`
- Create: `examples/chat.rs`
- Modify: `crates/boba-widgets/src/lib.rs`

Write `examples/chat.rs` that demonstrates composing a chat UI from Viewport + custom message rendering. Show the pattern without being a reusable widget.

Remove `pub mod chat;` from lib.rs.

**Commit** — `refactor: move Chat widget to examples as composition pattern`

---

### Task 21: Move wizard.rs to examples

**Files:**
- Delete: `crates/boba-widgets/src/wizard.rs`
- Create: `examples/wizard.rs`
- Modify: `crates/boba-widgets/src/lib.rs`

Write `examples/wizard.rs` showing Progress + step state machine + key handling.

Remove `pub mod wizard;` from lib.rs.

**Commit** — `refactor: move Wizard widget to examples as composition pattern`

---

### Task 22: Layer 2 verification

Full verification suite. Ensure no public API references to deleted modules remain.

---

## Layer 3: Split Help + Add Overlay Utility

### Task 23: Extract overlay.rs utility

**Files:**
- Create: `crates/boba-widgets/src/overlay.rs`
- Modify: `crates/boba-widgets/src/lib.rs`

Extract the centered-rect + Clear rendering logic currently duplicated in Modal (lines 226-254) and Help (lines 307-314):

```rust
//! Overlay positioning and rendering utilities.

/// Compute a centered sub-rect within `area` using percentage dimensions.
pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect { ... }

/// Compute a centered sub-rect with fixed dimensions, clamped to `area`.
pub fn centered_fixed(width: u16, height: u16, area: Rect) -> Rect { ... }

/// Clear and optionally frame an overlay area.
pub fn render_overlay(frame: &mut Frame, area: Rect, block: Option<&Block>) -> Rect { ... }
```

**Step 1: Write tests** for centered_rect and centered_fixed
**Step 2: Implement**
**Step 3: Commit** — `feat: add overlay utility module`

---

### Task 24: Refactor Help into formatter only

**Files:**
- Modify: `crates/boba-widgets/src/help.rs`

Keep `HelpBinding`, `HelpStyle`, and the formatting methods (`short_help_line`, `format_full_help`). Remove the `Component` impl, scroll state, and overlay rendering. Help becomes a pure rendering utility — no state, no update loop.

The overlay behavior moves to examples showing how to compose `help::format_full_help()` + `overlay::render_overlay()` + Viewport.

**Commit** — `refactor: simplify Help to pure formatting utility`

---

### Task 25: Migrate Modal to use overlay.rs

**Files:**
- Modify: `crates/boba-widgets/src/modal.rs`

Replace Modal's `centered_rect()` method (lines 226-254) with calls to `overlay::centered_rect()` or `overlay::centered_fixed()`. Replace its Clear + block rendering with `overlay::render_overlay()`.

**Commit** — `refactor: use overlay utility in Modal`

---

### Task 26: Layer 3 verification

Full verification suite.

---

## Layer 4: Update Examples + Final Polish

### Task 27: Update all examples for new patterns

**Files:**
- Modify: all `examples/*.rs`

Ensure every example demonstrates:
- Naked widgets with caller-controlled blocks
- `focus_block()` helper usage
- Composition patterns (multiple widgets in one view)

### Task 28: Final verification

```
cargo build
cargo test
cargo clippy --all-targets
cargo doc --no-deps
cargo build --examples
```

### Task 29: Final commit + push

Commit any remaining fixups. Push to remote.

---

## Verification Checklist (After Each Layer)

- [ ] `cargo build` — clean
- [ ] `cargo test` — all pass
- [ ] `cargo clippy --all-targets` — zero warnings
- [ ] `cargo doc --no-deps` — builds
- [ ] `cargo build --examples` — all compile
- [ ] Spot-check 2-3 examples visually
