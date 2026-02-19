# Widget Composability Restructure

## Problem

Boba widgets are "finished rooms" — every complex widget forces `Borders::ALL`, hardcodes internal layouts, and duplicates selection/editing logic. This prevents composition: you can't nest a TextInput inside a Modal without double borders, and five widgets independently reimplement identical scroll navigation.

Bubble Tea's bubbles are "naked" by default — `textinput.View()` returns just the text, `spinner.View()` returns just the spinning character. The caller wraps with lipgloss for chrome. This makes them composable building blocks.

We should follow ratatui's own conventions: `ratatui::widgets::List`, `Table`, `Paragraph` all accept an optional `.block(Block)`. No block = naked rendering. Boba widgets should work the same way.

## Strategy

Bottom-up, layered approach. Each layer is independently shippable and testable.

## Layer 0: Borderless Defaults

Every widget that hardcodes `Block::default().borders(Borders::ALL)` gets:
- A `block: Option<Block<'static>>` field, defaulting to `None`
- A `with_block(Block<'static>) -> Self` builder
- View renders naked by default; renders the block + uses `block.inner(area)` only when set

**Affected widgets (15):**
- TextInput, TextArea, List, Table, Viewport, Select, FilePicker
- Help (overlay), Chat, Tabs, Progress, Modal, Dropdown, Wizard
- Autocomplete (dropdown portion)

**Style struct cleanup:**
Remove `focused_border` and `unfocused_border` fields from all XyzStyle structs. Border styling belongs to the Block, not the content style.

**New `chrome.rs` utility module:**
```rust
pub fn focus_block(title: &str, focused: bool) -> Block<'_> {
    let color = if focused { Color::Cyan } else { Color::DarkGray };
    Block::bordered()
        .title(title)
        .border_style(Style::default().fg(color))
}
```

**Example migration — before:**
```rust
// Widget forces borders internally
self.input.view(frame, area);
```

**After:**
```rust
// Naked (composable)
self.input.view(frame, area);

// With borders (caller-controlled)
let block = focus_block("Name", self.input.focused());
let inner = block.inner(area);
frame.render_widget(block, area);
self.input.view(frame, inner);
```

## Layer 1: Extract Shared Primitives

Two internal modules to eliminate duplicated logic:

### `selection.rs` — Shared navigation state

Tracks cursor position, scroll offset, total count, visible height. Provides: `move_up()`, `move_down()`, `page_up()`, `page_down()`, `home()`, `end()`, `half_page_up()`, `half_page_down()`.

Currently duplicated across: List, Table, Select, Dropdown, Autocomplete, FilePicker (identical scroll math in each).

### `text_edit.rs` — Shared single-line text editing

Extracted from TextInput's existing implementation. Tracks char buffer, cursor position, undo/redo stacks. Provides: `insert_char()`, `delete_back()`, `delete_forward()`, `move_left()`, `move_right()`, `word_left()`, `word_right()`, `kill_to_start()`, `kill_to_end()`, `undo()`, `redo()`.

Used by: TextInput (canonical), Autocomplete (currently duplicates ~500 lines), Search (reimplements basic editing). TextArea keeps its own multi-line implementation.

Both are internal modules (not public widgets). They reduce ~800 lines of duplicated code.

## Layer 2: Merge/Kill Redundant Widgets

### Kill `autocomplete.rs`

Replace with TextInput + Dropdown composition. TextInput gets a `with_dropdown(Dropdown)` mode that opens a filtered dropdown below the input. Uses shared `text_edit.rs` (already canonical in TextInput) and shared `selection.rs` for dropdown navigation. Eliminates ~520 lines of duplicated text editing logic.

### Merge `select.rs` + `dropdown.rs`

Select is a Dropdown with a trigger label. Dropdown becomes the canonical overlay selector. Select becomes a thin wrapper: one-line trigger display + Dropdown in popup mode. One navigation implementation instead of two.

### Move `chat.rs` to examples

Chat is an application pattern, not a building block. It hardcodes role labels, separator strings, streaming animation, markdown detection, spinner integration. Replace with `examples/chat.rs` showing how to compose Viewport + custom message rendering for a chat UI.

### Move `wizard.rs` to examples

Wizard is a full-page layout engine (progress bar + step content + navigation hints). Replace with `examples/wizard.rs` showing how to compose Progress + step state machine + key handling.

## Layer 3: Split Help

### `help.rs` becomes a pure formatter

No overlay, no scroll state, no Component impl:
```rust
pub fn format_short_help(bindings: &[HelpBinding], max_width: u16) -> Line
pub fn format_full_help(bindings: &[HelpBinding]) -> Vec<Line>
```
HelpBinding and HelpStyle structs stay. The formatting logic stays. The overlay behavior leaves.

### New `overlay.rs` utility

Generic centered overlay container:
```rust
pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect
pub fn render_overlay(frame: &mut Frame, area: Rect, block: Option<Block>)
```
Used by Modal, and any user who wants an overlay (help popup, confirmation dialog, etc.). Replaces duplicated centered-rect + Clear logic in Modal, Help, Wizard.

## Layer 4: Update Examples

All examples updated to demonstrate new composition patterns:
- `counter.rs` — naked widgets, caller-controlled borders
- `input_form.rs` — multiple TextInputs with focus_block() helper
- `full_app.rs` — List + Viewport + Tabs composed with caller blocks
- `file_browser.rs` — FilePicker + Viewport
- New `examples/chat.rs` — Viewport + custom message rendering (replaces widget)
- New `examples/wizard.rs` — Progress + step state machine (replaces widget)

## Net Impact

| Metric | Before | After |
|--------|--------|-------|
| Widget files | ~28 | ~22 |
| Widgets forcing borders | 15 | 0 |
| Duplicated selection logic | 6 copies | 1 shared module |
| Duplicated text editing | 4 copies | 1 shared module |
| Lines of code (est.) | ~13,100 | ~12,200 |

## Verification

After each layer:
1. `cargo build` — compiles cleanly
2. `cargo test` — all tests pass
3. `cargo clippy --all-targets` — zero warnings
4. `cargo doc --no-deps` — docs build
5. `cargo build --examples` — all examples compile
6. Manual spot-check: run 2-3 examples to verify visual correctness
