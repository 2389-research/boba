# TextArea Unification Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Merge all TextInput features into TextArea, making it the single text editing widget.

**Architecture:** Incremental feature migration — each task adds one capability to TextArea, with tests first. TextInput becomes a deprecated thin wrapper. Search is refactored to compose a single-line TextArea. TextEditState is deprecated.

**Tech Stack:** Rust, ratatui, crossterm, boba-core Component trait

**Test command:** `cargo test -p boba-widgets`

**Key files:**
- Primary: `crates/boba-widgets/src/text_area.rs`
- Deprecate: `crates/boba-widgets/src/text_input.rs`
- Deprecate: `crates/boba-widgets/src/text_edit.rs`
- Refactor: `crates/boba-widgets/src/search.rs`
- Update: `crates/boba-widgets/src/lib.rs`
- Update: `examples/autocomplete.rs`, `examples/input_form.rs`, `examples/wizard.rs`

---

## Task 1: Single-Line Mode + Submit Binding

Add `SubmitBinding` enum, `single_line` field, and `submit_binding` field.
In single-line mode: Enter submits, paste strips newlines, Up/Down always go
to history. `with_submit()` works in any mode.

**Files:**
- Modify: `crates/boba-widgets/src/text_area.rs`

**Step 1: Add SubmitBinding enum and Message::Submit variant**

Add before the `TextArea` struct definition:

```rust
/// Controls which key combination triggers a submit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SubmitBinding {
    /// Enter key submits.
    #[default]
    Enter,
    /// Shift+Enter submits.
    ShiftEnter,
    /// Ctrl+Enter submits.
    CtrlEnter,
    /// No submit key; all Enter variants insert newlines.
    None,
}
```

Add `Submit(String)` to the `Message` enum.

Add to the `TextArea` struct:

```rust
single_line: bool,
submit_binding: SubmitBinding,
```

Default `single_line: false`, `submit_binding: SubmitBinding::None`.

Add builder methods:

```rust
pub fn with_single_line(mut self, single_line: bool) -> Self {
    self.single_line = single_line;
    if single_line && self.submit_binding == SubmitBinding::None {
        self.submit_binding = SubmitBinding::Enter;
    }
    self
}

pub fn with_submit(mut self, binding: SubmitBinding) -> Self {
    self.submit_binding = binding;
    self
}
```

**Step 2: Write failing tests for single-line mode**

```rust
#[test]
fn single_line_enter_submits() {
    let mut ta = TextArea::new().with_single_line(true);
    ta.focus();
    ta.update(Message::KeyPress(key(KeyCode::Char('h'))));
    ta.update(Message::KeyPress(key(KeyCode::Char('i'))));
    let cmd = ta.update(Message::KeyPress(key(KeyCode::Enter)));
    match cmd.into_message() {
        Some(Message::Submit(s)) => assert_eq!(s, "hi"),
        other => panic!("Expected Submit(\"hi\"), got {:?}", other),
    }
    // Should NOT have inserted a newline
    assert_eq!(ta.line_count(), 1);
}

#[test]
fn single_line_paste_strips_newlines() {
    let mut ta = TextArea::new().with_single_line(true);
    ta.focus();
    ta.update(Message::Paste("hello\nworld\nfoo".into()));
    assert_eq!(ta.value(), "helloworldfoo");
    assert_eq!(ta.line_count(), 1);
}

#[test]
fn single_line_up_down_browse_history() {
    let mut ta = TextArea::new()
        .with_single_line(true)
        .with_history(10);
    ta.focus();
    ta.push_history("first");
    ta.push_history("second");
    // Up should browse history
    ta.update(Message::KeyPress(key(KeyCode::Up)));
    assert_eq!(ta.value(), "second");
    ta.update(Message::KeyPress(key(KeyCode::Up)));
    assert_eq!(ta.value(), "first");
    // Down should go forward
    ta.update(Message::KeyPress(key(KeyCode::Down)));
    assert_eq!(ta.value(), "second");
}

#[test]
fn multiline_submit_with_ctrl_enter() {
    let mut ta = TextArea::new()
        .with_submit(SubmitBinding::CtrlEnter);
    ta.focus();
    ta.update(Message::KeyPress(key(KeyCode::Char('h'))));
    ta.update(Message::KeyPress(key(KeyCode::Char('i'))));
    // Plain Enter should insert newline
    ta.update(Message::KeyPress(key(KeyCode::Enter)));
    assert_eq!(ta.line_count(), 2);
    // Ctrl+Enter should submit
    let cmd = ta.update(Message::KeyPress(ctrl_key(KeyCode::Enter)));
    match cmd.into_message() {
        Some(Message::Submit(s)) => assert_eq!(s, "hi\n"),
        other => panic!("Expected Submit, got {:?}", other),
    }
}

#[test]
fn multiline_enter_submit_shift_enter_newline() {
    let mut ta = TextArea::new()
        .with_submit(SubmitBinding::Enter);
    ta.focus();
    ta.update(Message::KeyPress(key(KeyCode::Char('a'))));
    // Shift+Enter should insert newline
    ta.update(Message::KeyPress(shift_key(KeyCode::Enter)));
    assert_eq!(ta.line_count(), 2);
    // Enter should submit
    let cmd = ta.update(Message::KeyPress(key(KeyCode::Enter)));
    match cmd.into_message() {
        Some(Message::Submit(_)) => {}
        other => panic!("Expected Submit, got {:?}", other),
    }
}

#[test]
fn multiline_no_submit_binding_all_enters_newline() {
    let mut ta = TextArea::new(); // default: SubmitBinding::None
    ta.focus();
    ta.update(Message::KeyPress(key(KeyCode::Enter)));
    assert_eq!(ta.line_count(), 2);
}
```

Note: you'll need to add `shift_key` and `ctrl_key` test helpers if they don't
exist in the test module already. Model them after the existing `key()` helper.

**Step 3: Run tests to verify they fail**

Run: `cargo test -p boba-widgets -- single_line`
Expected: FAIL — `SubmitBinding` not defined, `Submit` not a variant, etc.

**Step 4: Implement the single-line mode logic**

Modify `update()` Enter key handling (currently around line 833):

```rust
(KeyCode::Enter, m) => {
    // Check if this Enter variant matches the submit binding
    let is_submit = match self.submit_binding {
        SubmitBinding::Enter => m == KeyModifiers::NONE,
        SubmitBinding::ShiftEnter => m.contains(KeyModifiers::SHIFT),
        SubmitBinding::CtrlEnter => m.contains(KeyModifiers::CONTROL),
        SubmitBinding::None => false,
    };
    if is_submit {
        return Command::message(Message::Submit(self.value()));
    }
    if self.single_line {
        return Command::none();
    }
    // Existing newline insertion logic
    self.push_undo();
    self.delete_selection();
    let rest = self.lines[self.cursor_row].split_off(self.cursor_col);
    self.cursor_row += 1;
    self.cursor_col = 0;
    self.lines.insert(self.cursor_row, rest);
    Command::message(Message::Changed(self.value()))
}
```

Modify Up/Down handling: when `self.single_line`, always try history
(skip the `self.lines.len() == 1` check).

Modify `Paste` handling: when `self.single_line`, strip `\n` and `\r`
from pasted text before inserting.

**Step 5: Run tests to verify they pass**

Run: `cargo test -p boba-widgets`
Expected: All 228+ existing tests pass, plus new tests pass.

**Step 6: Commit**

```
feat(text_area): add single-line mode and submit binding
```

---

## Task 2: Horizontal Scroll + Overflow Indicator (Single-Line)

When `single_line` is true, track horizontal offset and show `…` at edges.

**Files:**
- Modify: `crates/boba-widgets/src/text_area.rs`

**Step 1: Write failing tests**

```rust
#[test]
fn single_line_horizontal_scroll_keeps_cursor_visible() {
    let mut ta = TextArea::new().with_single_line(true);
    ta.focus();
    // Type a long string
    for c in "abcdefghijklmnopqrstuvwxyz".chars() {
        ta.update(Message::KeyPress(key(KeyCode::Char(c))));
    }
    // cursor_col should be at 26
    assert_eq!(ta.cursor_col(), 26);
    // h_offset should have advanced (exact value depends on render width)
}
```

Note: Testing render output precisely is hard without a Frame. Focus tests
on the offset tracking logic. Visual verification via examples.

**Step 2: Implement horizontal offset tracking**

Add field: `h_offset: usize` (default 0).

In `view()`, when `single_line`:
- Calculate `available_width` from inner area
- Adjust `h_offset` so cursor is visible (same logic as TextInput lines 566-579)
- Slice the display string to `h_offset..h_offset+available_width`
- Render `…` at left edge if `h_offset > 0`
- Render `…` at right edge if content extends past visible width
- Skip line numbers, vertical scroll

**Step 3: Run tests, commit**

```
feat(text_area): add horizontal scroll with overflow indicator for single-line mode
```

---

## Task 3: Placeholder + Prompt

**Files:**
- Modify: `crates/boba-widgets/src/text_area.rs`

**Step 1: Write failing tests**

```rust
#[test]
fn placeholder_stored() {
    let ta = TextArea::new().with_placeholder("Type here...");
    assert_eq!(ta.value(), "");
    // Placeholder is a display concern — just verify it's stored
}

#[test]
fn prompt_stored() {
    let ta = TextArea::new().with_prompt("> ");
    // Prompt is a display concern — verify it's stored and doesn't
    // interfere with value()
    assert_eq!(ta.value(), "");
}
```

**Step 2: Implement**

Add fields:
```rust
placeholder: String,
prompt: String,
```

Add builders:
```rust
pub fn with_placeholder(mut self, text: impl Into<String>) -> Self {
    self.placeholder = text.into();
    self
}

pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
    self.prompt = prompt.into();
    // Unify with existing line_prompt
    self.line_prompt = Some(self.prompt.clone());
    self
}
```

In `view()`:
- When content is empty and not focused, render placeholder text with
  `self.style.placeholder` style instead of content.
- Prompt rendering: reuse existing `line_prompt` rendering path. In
  single-line mode, `prompt` is the prefix rendered once.

Add style fields: `prompt: Style` and `placeholder: Style` to
`TextAreaStyle` with sensible defaults (prompt = Cyan, placeholder = DarkGray).

**Step 3: Run tests, commit**

```
feat(text_area): add placeholder and prompt support
```

---

## Task 4: Echo Mode

**Files:**
- Modify: `crates/boba-widgets/src/text_area.rs`

**Step 1: Write failing tests**

```rust
#[test]
fn echo_mode_password_hides_text() {
    let mut ta = TextArea::new()
        .with_single_line(true)
        .with_echo_mode(EchoMode::Password('*'));
    ta.focus();
    ta.update(Message::KeyPress(key(KeyCode::Char('s'))));
    ta.update(Message::KeyPress(key(KeyCode::Char('e'))));
    ta.update(Message::KeyPress(key(KeyCode::Char('c'))));
    // value() returns real text
    assert_eq!(ta.value(), "sec");
    // display is a rendering concern — verify the mode is stored
    assert!(matches!(ta.echo_mode(), EchoMode::Password('*')));
}

#[test]
fn echo_mode_hidden_returns_empty_display() {
    let ta = TextArea::new()
        .with_single_line(true)
        .with_echo_mode(EchoMode::Hidden)
        .with_content("secret");
    assert_eq!(ta.value(), "secret");
    assert!(matches!(ta.echo_mode(), EchoMode::Hidden));
}
```

**Step 2: Implement**

Move `EchoMode` enum from `text_input.rs` into `text_area.rs`:

```rust
#[derive(Debug, Clone, Default)]
pub enum EchoMode {
    #[default]
    Normal,
    Password(char),
    Hidden,
}
```

Add field: `echo_mode: EchoMode` (default Normal).
Add builder: `with_echo_mode()`.
Add accessor: `echo_mode() -> &EchoMode`.

Add private method:

```rust
fn display_chars(&self) -> Vec<char> {
    match &self.echo_mode {
        EchoMode::Normal => self.lines[0].clone(),
        EchoMode::Password(c) => vec![*c; self.lines[0].len()],
        EchoMode::Hidden => Vec::new(),
    }
}
```

In `view()`, when `single_line` and echo mode is not Normal, use
`display_chars()` for rendering instead of actual content. Disable
selection display in Password/Hidden modes.

**Step 3: Run tests, commit**

```
feat(text_area): add echo mode for password and hidden input
```

---

## Task 5: Autocomplete / Suggestions

**Files:**
- Modify: `crates/boba-widgets/src/text_area.rs`

**Step 1: Write failing tests**

```rust
#[test]
fn suggestions_filter_as_user_types() {
    let mut ta = TextArea::new()
        .with_single_line(true)
        .with_suggestions(vec!["apple".into(), "banana".into(), "apricot".into()]);
    ta.focus();
    ta.update(Message::KeyPress(key(KeyCode::Char('a'))));
    let avail = ta.available_suggestions();
    assert_eq!(avail.len(), 2); // apple, apricot
    assert!(avail.contains(&"apple".to_string()));
    assert!(avail.contains(&"apricot".to_string()));
}

#[test]
fn tab_accepts_suggestion() {
    let mut ta = TextArea::new()
        .with_single_line(true)
        .with_suggestions(vec!["apple".into()]);
    ta.focus();
    ta.update(Message::KeyPress(key(KeyCode::Char('a'))));
    assert_eq!(ta.current_suggestion(), Some("apple"));
    ta.update(Message::KeyPress(key(KeyCode::Tab)));
    assert_eq!(ta.value(), "apple");
}

#[test]
fn suggestions_ignored_in_multiline() {
    let mut ta = TextArea::new()
        .with_suggestions(vec!["apple".into()]);
    ta.focus();
    ta.update(Message::KeyPress(key(KeyCode::Char('a'))));
    assert_eq!(ta.current_suggestion(), Option::<&str>::None);
}
```

**Step 2: Implement**

Port the suggestion fields and logic from TextInput (lines 97-101, 195-226,
307-327):

Add fields:
```rust
suggestions: Vec<String>,
filtered_suggestions: Vec<String>,
show_suggestions: bool,
suggestion_index: usize,
```

Port methods: `filter_suggestions()`, `accept_suggestion()`,
`set_suggestions()`, `with_suggestions()`, `current_suggestion()`,
`available_suggestions()`, `show_suggestions()`.

Guard all suggestion logic with `if !self.single_line { return; }`.

In `view()` for single-line mode, render ghost text after cursor using
`self.style.suggestion` style (port from TextInput lines 587-601).

Call `filter_suggestions()` after every content change in single-line mode.

**Step 3: Run tests, commit**

```
feat(text_area): add autocomplete suggestions for single-line mode
```

---

## Task 6: Validation

**Files:**
- Modify: `crates/boba-widgets/src/text_area.rs`

**Step 1: Write failing tests**

```rust
#[test]
fn validation_sets_error() {
    let mut ta = TextArea::new()
        .with_single_line(true)
        .with_validate(|s| {
            if s.len() > 3 {
                Err("Too long".into())
            } else {
                Ok(())
            }
        });
    ta.focus();
    ta.update(Message::KeyPress(key(KeyCode::Char('a'))));
    assert!(ta.err().is_none());
    ta.update(Message::KeyPress(key(KeyCode::Char('b'))));
    ta.update(Message::KeyPress(key(KeyCode::Char('c'))));
    ta.update(Message::KeyPress(key(KeyCode::Char('d'))));
    assert_eq!(ta.err(), Some("Too long"));
}

#[test]
fn validation_clears_on_valid() {
    let mut ta = TextArea::new()
        .with_single_line(true)
        .with_validate(|s| {
            if s.is_empty() { Err("Required".into()) } else { Ok(()) }
        });
    ta.focus();
    // Initially no error (validation runs on change, not construction)
    assert!(ta.err().is_none());
    ta.update(Message::KeyPress(key(KeyCode::Char('a'))));
    assert!(ta.err().is_none());
    ta.update(Message::KeyPress(key(KeyCode::Backspace)));
    assert_eq!(ta.err(), Some("Required"));
    ta.update(Message::KeyPress(key(KeyCode::Char('b'))));
    assert!(ta.err().is_none());
}
```

**Step 2: Implement**

Add fields:
```rust
validate: Option<Box<dyn Fn(&str) -> Result<(), String> + Send>>,
err: Option<String>,
```

Add builder and accessor:
```rust
pub fn with_validate(
    mut self,
    f: impl Fn(&str) -> Result<(), String> + Send + 'static,
) -> Self {
    self.validate = Some(Box::new(f));
    self
}

pub fn err(&self) -> Option<&str> {
    self.err.as_deref()
}
```

Add private method:
```rust
fn run_validate(&mut self) {
    if let Some(ref f) = self.validate {
        self.err = f(&self.value()).err();
    }
}
```

Call `run_validate()` after every `Changed` message is emitted, and
after paste operations.

**Step 3: Run tests, commit**

```
feat(text_area): add input validation
```

---

## Task 7: Visual Height + Max Visible Lines

**Files:**
- Modify: `crates/boba-widgets/src/text_area.rs`

**Step 1: Write failing tests**

```rust
#[test]
fn visual_height_single_line() {
    let ta = TextArea::new()
        .with_single_line(true)
        .with_content("hello");
    assert_eq!(ta.visual_height(80), 1);
}

#[test]
fn visual_height_multiline() {
    let ta = TextArea::new()
        .with_content("line1\nline2\nline3");
    assert_eq!(ta.visual_height(80), 3);
}

#[test]
fn visual_height_with_wrapping() {
    let ta = TextArea::new()
        .with_soft_wrap(true)
        .with_content("a]bcdefghij"); // 10 chars
    // At width 5, wraps to 2 lines
    assert_eq!(ta.visual_height(5), 2);
}

#[test]
fn visual_height_capped_by_max_visible_lines() {
    let ta = TextArea::new()
        .with_content("a\nb\nc\nd\ne")
        .with_max_visible_lines(3);
    assert_eq!(ta.visual_height(80), 3);
}

#[test]
fn max_visible_lines_scrolls_internally() {
    let mut ta = TextArea::new()
        .with_content("a\nb\nc\nd\ne")
        .with_max_visible_lines(3);
    ta.focus();
    // Cursor starts at (0,0), visible lines are a,b,c
    // Move down to line 4 (e)
    ta.cursor_down(); // b
    ta.cursor_down(); // c
    ta.cursor_down(); // d
    ta.cursor_down(); // e
    assert_eq!(ta.cursor_row(), 4);
    // scroll_offset should have adjusted to keep cursor visible
}
```

**Step 2: Implement**

Add field: `max_visible_lines: Option<u16>` (default None).

Add builder:
```rust
pub fn with_max_visible_lines(mut self, n: u16) -> Self {
    self.max_visible_lines = Some(n);
    self
}
```

Add public method:
```rust
pub fn visual_height(&self, width: u16) -> u16 {
    let raw = if self.single_line {
        1
    } else if self.soft_wrap && width > 0 {
        let w = width as usize;
        self.lines.iter().map(|line| {
            let len = line.len();
            if len == 0 { 1 } else { ((len + w - 1) / w) as u16 }
        }).sum()
    } else {
        self.lines.len() as u16
    };
    match self.max_visible_lines {
        Some(max) => raw.min(max),
        None => raw,
    }
}
```

In `view()`, when `max_visible_lines` is set, use it to cap the
visible height instead of using the full inner area height.

**Step 3: Run tests, commit**

```
feat(text_area): add visual_height() and max_visible_lines
```

---

## Task 8: Convenience Methods

Port remaining TextInput public API methods.

**Files:**
- Modify: `crates/boba-widgets/src/text_area.rs`

**Step 1: Write failing tests**

```rust
#[test]
fn is_empty_and_len() {
    let ta = TextArea::new();
    assert!(ta.is_empty());
    assert_eq!(ta.len(), 0);

    let ta = TextArea::new().with_content("hello");
    assert!(!ta.is_empty());
    assert_eq!(ta.len(), 5);
}

#[test]
fn cursor_position_flat_index() {
    let mut ta = TextArea::new().with_content("ab\ncd");
    ta.focus();
    ta.cursor_down();       // row 1
    ta.update(Message::KeyPress(key(KeyCode::Right))); // col 1
    // Flat index: 2 (for \n) + 1 = 3
    assert_eq!(ta.cursor_position(), 3);
}

#[test]
fn set_cursor_flat_index() {
    let mut ta = TextArea::new().with_content("ab\ncd");
    ta.set_cursor(3); // should land at row 1, col 0  ("c")
    assert_eq!(ta.cursor_row(), 1);
    assert_eq!(ta.cursor_col(), 0);
}

#[test]
fn reset_clears_everything() {
    let mut ta = TextArea::new().with_content("hello");
    ta.focus();
    ta.reset();
    assert!(ta.is_empty());
    assert_eq!(ta.cursor_row(), 0);
    assert_eq!(ta.cursor_col(), 0);
}
```

**Step 2: Implement**

```rust
pub fn is_empty(&self) -> bool {
    self.lines.len() == 1 && self.lines[0].is_empty()
}

pub fn len(&self) -> usize {
    let newlines = self.lines.len().saturating_sub(1);
    let chars: usize = self.lines.iter().map(|l| l.len()).sum();
    chars + newlines
}

pub fn cursor_position(&self) -> usize {
    let mut pos = 0;
    for row in 0..self.cursor_row {
        pos += self.lines[row].len() + 1; // +1 for newline
    }
    pos + self.cursor_col
}

pub fn set_cursor(&mut self, pos: usize) {
    let mut remaining = pos;
    for (row, line) in self.lines.iter().enumerate() {
        if remaining <= line.len() {
            self.cursor_row = row;
            self.cursor_col = remaining;
            return;
        }
        remaining -= line.len() + 1; // +1 for newline
    }
    // Past end — clamp to last position
    self.cursor_row = self.lines.len() - 1;
    self.cursor_col = self.lines[self.cursor_row].len();
}

pub fn reset(&mut self) {
    self.lines = vec![Vec::new()];
    self.cursor_row = 0;
    self.cursor_col = 0;
    self.scroll_offset = 0;
    self.selection_start = None;
    self.undo_stack.clear();
    self.redo_stack.clear();
}
```

**Step 3: Run tests, commit**

```
feat(text_area): add convenience methods (is_empty, len, cursor_position, set_cursor, reset)
```

---

## Task 9: Deprecated TextInput Wrapper

**Files:**
- Modify: `crates/boba-widgets/src/text_input.rs`
- Modify: `crates/boba-widgets/src/lib.rs`

**Step 1: Write a test that the wrapper preserves existing behavior**

```rust
#[test]
fn wrapper_enter_submits() {
    let mut input = TextInput::new("placeholder");
    input.focus();
    input.update(text_input::Message::KeyPress(key(KeyCode::Char('x'))));
    let cmd = input.update(text_input::Message::KeyPress(key(KeyCode::Enter)));
    match cmd.into_message() {
        Some(text_input::Message::Submit(s)) => assert_eq!(s, "x"),
        other => panic!("Expected Submit, got {:?}", other),
    }
}
```

**Step 2: Implement the wrapper**

Replace TextInput's internals with a TextArea in single-line mode. The
existing `text_input::Message` enum stays as-is for backward compatibility.
Internally, map between `text_input::Message` and `text_area::Message`.

Mark with `#[deprecated(since = "0.2.0", note = "Use TextArea with .with_single_line(true)")]`.

Keep the existing `text_input::Message` enum and `TextInputStyle` struct
for backward compatibility, but implement them as thin mappings.

**Step 3: Verify ALL existing TextInput tests still pass**

Run: `cargo test -p boba-widgets -- text_input`
Expected: All 94 existing tests pass.

**Step 4: Commit**

```
refactor(text_input): deprecate TextInput as thin wrapper over TextArea
```

---

## Task 10: Refactor Search to Compose TextArea

**Files:**
- Modify: `crates/boba-widgets/src/search.rs`

**Step 1: Verify existing search tests pass before refactoring**

Run: `cargo test -p boba-widgets -- search`
Expected: All existing search tests pass.

**Step 2: Replace TextEditState with single-line TextArea**

Replace `editor: TextEditState` with a `TextArea` in single-line mode.
Delegate key handling for the query input to the embedded TextArea.
Remove direct `TextEditState` usage.

The Search component still manages its own activation state, match
tracking, and Ctrl+N/Ctrl+P navigation — only the text editing part
is delegated.

**Step 3: Verify all search tests still pass**

Run: `cargo test -p boba-widgets -- search`
Expected: All existing tests pass unchanged.

**Step 4: Commit**

```
refactor(search): compose single-line TextArea instead of TextEditState
```

---

## Task 11: Deprecate TextEditState

**Files:**
- Modify: `crates/boba-widgets/src/text_edit.rs`
- Modify: `crates/boba-widgets/src/lib.rs`

**Step 1: Add deprecation attribute**

```rust
#[deprecated(
    since = "0.2.0",
    note = "Use TextArea with .with_single_line(true) instead"
)]
pub struct TextEditState { ... }
```

**Step 2: Update lib.rs doc comment**

Update the module table in `lib.rs` to note the deprecation.

**Step 3: Verify everything compiles and tests pass**

Run: `cargo test -p boba-widgets`
Expected: All tests pass. Deprecation warnings appear but don't fail.

**Step 4: Commit**

```
refactor(text_edit): deprecate TextEditState in favor of TextArea
```

---

## Task 12: Update Examples

**Files:**
- Modify: `examples/autocomplete.rs`
- Modify: `examples/input_form.rs`
- Modify: `examples/wizard.rs`

**Step 1: Update each example to use TextArea directly**

Replace `TextInput::new("placeholder")` with
`TextArea::new().with_single_line(true).with_placeholder("placeholder")`.

Update message enum variants from `text_input::Message` to
`text_area::Message`.

**Step 2: Verify examples compile**

Run: `cargo build --examples`
Expected: Compiles with no errors (deprecation warnings from old imports
are acceptable).

**Step 3: Commit**

```
refactor(examples): migrate from TextInput to TextArea
```
