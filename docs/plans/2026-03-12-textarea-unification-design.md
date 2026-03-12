# TextArea Unification Design

TextInput and TextArea are conceptually the same widget — a TextInput is just a
single-line TextArea. This design merges all TextInput features into TextArea,
deprecates TextInput and TextEditState, and refactors Search to compose a
single-line TextArea.

## Motivation

The two widgets duplicate significant logic (editing, undo/redo, history, focus,
styling, block support, char limits) while each having features the other lacks.
Users of boba (notably Jeff) have resorted to multiline workarounds on top of
TextInput because TextArea wasn't discoverable or didn't have the right
affordances. Unifying removes confusion about which widget to use.

## Approach: Incremental Feature Migration

Features are layered onto TextArea one commit at a time. Each commit is
independently testable. The implementation order is:

1. Single-line mode + submit binding + horizontal scroll + overflow indicator
2. Placeholder + prompt
3. Echo mode (password/hidden)
4. Autocomplete/suggestions
5. Validation
6. `visual_height()` + `max_visible_lines`
7. Deprecated TextInput wrapper
8. Refactor Search to compose TextArea
9. Deprecate TextEditState

## Single-Line Mode

```rust
pub enum SubmitBinding {
    Enter,
    ShiftEnter,
    CtrlEnter,
    None,
}
```

`with_single_line(true)` enables:

- Enter submits by default (configurable via `with_submit()`)
- Up/Down navigate history instead of moving the cursor
- Paste strips newlines
- Horizontal scroll instead of vertical
- Overflow indicator (`…`) at clipped edges

`with_submit(binding)` works in any mode. In multiline mode the default is
`None`. The submit key fires `Message::Submit(String)`; the complementary
Enter variant inserts a newline.

| Mode | Default | Enter | Shift+Enter | Ctrl+Enter |
|------|---------|-------|-------------|------------|
| Single-line | `Enter` | submit | n/a | n/a |
| Multiline | `None` | newline | newline | newline |
| Multiline + `Enter` | | submit | newline | newline |
| Multiline + `ShiftEnter` | | newline | submit | newline |
| Multiline + `CtrlEnter` | | newline | newline | submit |

## Overflow Handling

**Single-line:** horizontal `h_offset` follows cursor. `…` rendered at the
left edge when `h_offset > 0` and at the right edge when content extends past
visible width. The indicator replaces the first/last visible character to keep
width consistent.

**Multiline height control:**

- `with_max_visible_lines(n: u16)` caps rendering height; content scrolls
  internally beyond that.
- `visual_height(width: u16) -> u16` returns rendered height accounting for
  wrapping, line count, and `max_visible_lines`. Parents use this to
  dynamically size the widget area.

## Placeholder + Prompt

- `with_placeholder(text)` — displayed when empty and unfocused, using the
  `placeholder` style.
- `with_prompt(text)` — in single-line mode, a one-time prefix (e.g. `"> "`).
  In multiline mode, rendered per-line (unifies the existing `line_prompt`).

## Echo Mode

```rust
pub enum EchoMode {
    Normal,
    Password(char),
    Hidden,
}
```

`with_echo_mode(mode)` — only meaningful in single-line mode. Ignored in
multiline mode.

## Autocomplete / Suggestions

- `set_suggestions(Vec<String>)` / `with_suggestions(Vec<String>)`
- Auto-filters as user types; ghost text rendered after cursor; Tab accepts.
- Only active in single-line mode.

## Validation

- `with_validate(fn)` — runs on every change, stores error string.
- `err() -> Option<&str>` — query current validation error.
- Works in both single-line and multiline modes.

## Convenience Methods

Ported from TextInput:

- `is_empty() -> bool`
- `len() -> usize`
- `cursor_position() -> usize` (flat char index, useful in single-line mode)
- `set_cursor(pos: usize)` (flat char index)
- `reset()`

## Message Enum

```rust
pub enum Message {
    KeyPress(KeyEvent),
    Paste(String),
    Changed(String),
    Submit(String),
    Copy(String),
    Cut(String),
}
```

`Submit` fires in any mode when the configured submit key is pressed.

## Style

```rust
pub struct TextAreaStyle {
    pub text: Style,
    pub cursor: Style,
    pub line_number: Style,
    pub selection: Style,
    pub prompt: Style,
    pub placeholder: Style,
    pub suggestion: Style,
}
```

## Deprecations

- **TextInput** becomes a thin wrapper that constructs
  `TextArea::new().with_single_line(true).with_placeholder(placeholder)`.
  Marked `#[deprecated]`.
- **TextEditState** marked `#[deprecated]`; docs point to single-line TextArea.
- **Search** refactored to embed a single-line TextArea instead of using
  TextEditState directly.

## Backward Compatibility

Existing code using `TextInput` continues to compile via the deprecated wrapper.
The `text_input` module remains in the public API with a deprecation notice. The
wrapper maps `text_input::Message` variants to `text_area::Message` variants so
callers don't need to change their match arms immediately.
