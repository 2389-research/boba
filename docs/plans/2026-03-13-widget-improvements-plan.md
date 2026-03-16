# Widget Improvements Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add viewport padding, viewport follow mode, search content/match-strategy, list multi-select, and improve FocusGroup docs/examples.

**Architecture:** Each feature is an independent addition to an existing widget. All changes are additive builder methods following boba's existing patterns. `center_rect()` already exists in `overlay.rs` — no work needed.

**Tech Stack:** Rust, ratatui, crossterm, boba-core Component trait

---

### Task 1: Viewport Padding

**Files:**
- Modify: `crates/boba-widgets/src/viewport.rs`

**Context:** Viewport currently has `with_block()` for borders but no way to add inner padding between the border and content. Users want `with_padding(top, right, bottom, left)` to add whitespace inside the viewport area. Ratatui has a `Padding` type we can reuse.

**Step 1: Write the failing test**

Add to the `tests` module at the bottom of `viewport.rs`:

```rust
#[test]
fn padding_reduces_visible_area() {
    use ratatui::layout::Rect;

    let vp = Viewport::new("line1\nline2\nline3\nline4\nline5\nline6\nline7\nline8\nline9\nline10")
        .with_padding(1, 1, 1, 1);

    // After setting padding, total_line_count is unchanged (content is the same)
    assert_eq!(vp.total_line_count(), 10);

    // The padding field should be set
    assert_eq!(vp.padding.top, 1);
    assert_eq!(vp.padding.right, 1);
    assert_eq!(vp.padding.bottom, 1);
    assert_eq!(vp.padding.left, 1);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p boba-widgets padding_reduces_visible_area`
Expected: FAIL — `with_padding` method doesn't exist.

**Step 3: Implement padding**

In the `Viewport` struct (around line 146), add:

```rust
padding: ratatui::layout::Padding,
```

In `Viewport::new()` (around line 178), add to the initializer:

```rust
padding: ratatui::layout::Padding::zero(),
```

Add the builder method after `with_block()` (around line 254):

```rust
/// Set inner padding between the border (or area edge) and content.
///
/// Uses ratatui's `Padding` type. Content area shrinks by the padding
/// amounts on each side.
pub fn with_padding(mut self, top: u16, right: u16, bottom: u16, left: u16) -> Self {
    self.padding = ratatui::layout::Padding::new(left, right, top, bottom);
    self
}
```

In the `view()` method (around line 524-531), after computing `inner` from the block, apply padding:

```rust
let inner = {
    let r = if let Some(ref block) = self.block {
        let r = block.inner(area);
        frame.render_widget(block.clone(), area);
        r
    } else {
        area
    };
    // Apply padding inside the block/area boundary.
    Rect {
        x: r.x.saturating_add(self.padding.left),
        y: r.y.saturating_add(self.padding.top),
        width: r.width.saturating_sub(self.padding.left + self.padding.right),
        height: r.height.saturating_sub(self.padding.top + self.padding.bottom),
    }
};
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p boba-widgets padding_reduces_visible_area`
Expected: PASS

**Step 5: Run full suite**

Run: `cargo test -p boba-widgets`
Expected: All tests pass (no regressions).

**Step 6: Commit**

```bash
git add crates/boba-widgets/src/viewport.rs
git commit -m "feat(viewport): add with_padding() for inner content padding"
```

---

### Task 2: Viewport Follow / Auto-Scroll Mode

**Files:**
- Modify: `crates/boba-widgets/src/viewport.rs`

**Context:** When using Viewport for streaming content (e.g. chat, logs), users want the viewport to auto-scroll to the bottom whenever new content is set. Currently they must call `goto_bottom()` manually after every `set_styled_content()`. A `with_follow(true)` flag makes this automatic.

**Step 1: Write the failing tests**

```rust
#[test]
fn follow_mode_scrolls_to_bottom_on_set_content() {
    let mut vp = Viewport::new("line1").with_follow(true);
    vp.set_content("line1\nline2\nline3\nline4\nline5\nline6\nline7\nline8\nline9\nline10\nline11\nline12\nline13\nline14\nline15\nline16\nline17\nline18\nline19\nline20\nline21\nline22\nline23\nline24\nline25");
    // With follow mode, offset should be set to bottom (u16::MAX, clamped in view)
    assert_eq!(vp.y_offset(), u16::MAX);
}

#[test]
fn follow_mode_scrolls_to_bottom_on_set_styled_content() {
    use ratatui::text::Line;
    let mut vp = Viewport::new("").with_follow(true);
    let lines: Vec<Line<'static>> = (0..30).map(|i| Line::raw(format!("line {i}"))).collect();
    vp.set_styled_content(lines);
    assert_eq!(vp.y_offset(), u16::MAX);
}

#[test]
fn no_follow_preserves_offset() {
    let mut vp = Viewport::new("line1");
    // Default: follow is false
    vp.set_content("line1\nline2\nline3");
    // set_content resets offset to 0
    assert_eq!(vp.y_offset(), 0);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p boba-widgets follow_mode`
Expected: FAIL — `with_follow` method doesn't exist.

**Step 3: Implement follow mode**

Add to Viewport struct:

```rust
follow: bool,
```

Initialize in `new()`:

```rust
follow: false,
```

Add builder method after `with_word_wrap()`:

```rust
/// Enable follow mode: automatically scroll to the bottom whenever
/// content is updated via `set_content()`, `set_styled_content()`, or
/// `set_ansi_content()`.
pub fn with_follow(mut self, enabled: bool) -> Self {
    self.follow = enabled;
    self
}
```

In `set_content()`, after the existing reset logic, add:

```rust
if self.follow {
    self.offset = u16::MAX;
}
```

In `set_styled_content()`, after `self.content.clear();`, add:

```rust
if self.follow {
    self.offset = u16::MAX;
}
```

In `set_ansi_content()`, change the `self.offset = 0;` line:

```rust
if self.follow {
    self.offset = u16::MAX;
} else {
    self.offset = 0;
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p boba-widgets follow_mode`
Expected: PASS

**Step 5: Run full suite**

Run: `cargo test -p boba-widgets`
Expected: All tests pass.

**Step 6: Commit**

```bash
git add crates/boba-widgets/src/viewport.rs
git commit -m "feat(viewport): add with_follow() for auto-scroll to bottom on content update"
```

---

### Task 3: Search MatchStrategy Enum

**Files:**
- Modify: `crates/boba-widgets/src/search.rs`

**Context:** Search currently leaves all matching to the parent. Jeff wants the Search widget to optionally own the content and do matching itself, with configurable match strategy (case-sensitive, case-insensitive, etc.). We add a `MatchStrategy` enum and a `set_content()` method. When content is set, Search performs the matching internally on every query change.

**Step 1: Write the failing tests**

```rust
#[test]
fn set_content_performs_matching() {
    let mut search = Search::new();
    search.set_content(vec![
        "Apple".to_string(),
        "banana".to_string(),
        "CHERRY".to_string(),
        "apricot".to_string(),
    ]);
    search.activate();
    search.update(Message::KeyPress(key(KeyCode::Char('a'))));

    // Default strategy is CaseInsensitive: matches "Apple", "banana", "apricot"
    // Matches are by index: [0, 1, 3]
    assert_eq!(search.match_count(), 3);
    assert_eq!(search.matches(), &[0, 1, 3]);
}

#[test]
fn set_content_case_sensitive() {
    let mut search = Search::new()
        .with_match_strategy(MatchStrategy::CaseSensitive);
    search.set_content(vec![
        "Apple".to_string(),
        "banana".to_string(),
        "apricot".to_string(),
    ]);
    search.activate();
    search.update(Message::KeyPress(key(KeyCode::Char('a'))));

    // CaseSensitive: only "apricot" matches lowercase 'a'
    assert_eq!(search.match_count(), 1);
    assert_eq!(search.matches(), &[2]);
}

#[test]
fn set_content_updates_matches_on_query_change() {
    let mut search = Search::new();
    search.set_content(vec![
        "hello".to_string(),
        "help".to_string(),
        "world".to_string(),
    ]);
    search.activate();
    search.update(Message::KeyPress(key(KeyCode::Char('h'))));
    assert_eq!(search.match_count(), 2); // hello, help

    search.update(Message::KeyPress(key(KeyCode::Char('e'))));
    assert_eq!(search.match_count(), 2); // hello, help (both contain "he")

    search.update(Message::KeyPress(key(KeyCode::Char('l'))));
    search.update(Message::KeyPress(key(KeyCode::Char('l'))));
    assert_eq!(search.match_count(), 1); // only "hello"
}

#[test]
fn empty_query_clears_matches_with_content() {
    let mut search = Search::new();
    search.set_content(vec!["apple".to_string(), "banana".to_string()]);
    search.activate();
    search.update(Message::KeyPress(key(KeyCode::Char('a'))));
    assert_eq!(search.match_count(), 2);

    // Backspace to empty
    search.update(Message::KeyPress(key(KeyCode::Backspace)));
    assert_eq!(search.match_count(), 0);
}

#[test]
fn external_set_matches_still_works_without_content() {
    // When no content is set, Search behaves exactly as before:
    // parent calls set_matches() manually.
    let mut search = Search::new();
    search.activate();
    search.update(Message::KeyPress(key(KeyCode::Char('x'))));
    search.set_matches(vec![0, 5, 10]);
    assert_eq!(search.match_count(), 3);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p boba-widgets set_content_performs`
Expected: FAIL — `set_content` and `MatchStrategy` don't exist.

**Step 3: Implement MatchStrategy and set_content**

Add the `MatchStrategy` enum before the `Search` struct:

```rust
/// Strategy for matching search queries against content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MatchStrategy {
    /// Case-insensitive substring match (default).
    #[default]
    CaseInsensitive,
    /// Case-sensitive substring match.
    CaseSensitive,
}
```

Add fields to the `Search` struct:

```rust
content: Option<Vec<String>>,
match_strategy: MatchStrategy,
```

Initialize in `new()`:

```rust
content: None,
match_strategy: MatchStrategy::default(),
```

Add builder method and `set_content()`:

```rust
/// Set the match strategy (default: case-insensitive).
pub fn with_match_strategy(mut self, strategy: MatchStrategy) -> Self {
    self.match_strategy = strategy;
    self
}

/// Provide searchable content. When set, the search widget performs
/// matching internally on every query change instead of relying on
/// the parent to call `set_matches()`.
pub fn set_content(&mut self, content: Vec<String>) {
    self.content = Some(content);
}
```

Add a private helper method:

```rust
/// Re-run matching against stored content using the current query.
fn update_matches_from_content(&mut self) {
    if let Some(ref content) = self.content {
        let query = self.editor.value();
        if query.is_empty() {
            self.matches.clear();
            self.current_match = 0;
            return;
        }
        let matches: Vec<usize> = match self.match_strategy {
            MatchStrategy::CaseInsensitive => {
                let q = query.to_lowercase();
                content
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| s.to_lowercase().contains(&q))
                    .map(|(i, _)| i)
                    .collect()
            }
            MatchStrategy::CaseSensitive => {
                content
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| s.contains(&query))
                    .map(|(i, _)| i)
                    .collect()
            }
        };
        self.set_matches(matches);
    }
}
```

In the `update()` method, after the line `Command::message(Message::QueryChanged(new_value))` (where the query changed), insert a call to re-match. The modified section should look like:

```rust
if new_value != old_value {
    self.update_matches_from_content();
    Command::message(Message::QueryChanged(new_value))
} else {
    Command::none()
}
```

Also in `deactivate()`, clear the content matches:

No change needed — `deactivate()` already calls `self.matches.clear()`.

**Step 4: Run tests to verify they pass**

Run: `cargo test -p boba-widgets set_content`
Expected: PASS

**Step 5: Run full suite**

Run: `cargo test -p boba-widgets`
Expected: All tests pass.

**Step 6: Commit**

```bash
git add crates/boba-widgets/src/search.rs
git commit -m "feat(search): add MatchStrategy enum and set_content() for self-managed matching"
```

---

### Task 4: List Multi-Select

**Files:**
- Modify: `crates/boba-widgets/src/list.rs`

**Context:** List currently supports single selection only. Users want `with_multi_select(true)` to allow toggling multiple items. Space toggles the current item's selected state. The cursor (highlight) remains separate from the selection set. A new `Message::Toggled(usize, bool)` variant notifies the parent. `selected_items()` returns the set of selected indices.

**Step 1: Write the failing tests**

```rust
#[test]
fn multi_select_toggle() {
    let mut list = List::new(vec!["a".to_string(), "b".to_string(), "c".to_string()])
        .with_multi_select(true);
    list.focus();

    // Space toggles current item (index 0)
    let cmd = list.update(Message::KeyPress(key(KeyCode::Char(' '))));
    assert!(matches!(cmd.into_message(), Some(Message::Toggled(0, true))));
    assert_eq!(list.selected_items(), &std::collections::BTreeSet::from([0]));

    // Move down and toggle index 1
    list.update(Message::KeyPress(key(KeyCode::Down)));
    list.update(Message::KeyPress(key(KeyCode::Char(' '))));
    assert_eq!(list.selected_items(), &std::collections::BTreeSet::from([0, 1]));

    // Toggle index 0 off — move back up and space
    list.update(Message::KeyPress(key(KeyCode::Up)));
    let cmd = list.update(Message::KeyPress(key(KeyCode::Char(' '))));
    assert!(matches!(cmd.into_message(), Some(Message::Toggled(0, false))));
    assert_eq!(list.selected_items(), &std::collections::BTreeSet::from([1]));
}

#[test]
fn multi_select_disabled_by_default() {
    let list = List::new(vec!["a".to_string(), "b".to_string()]);
    assert!(list.selected_items().is_empty());
}

#[test]
fn multi_select_space_does_nothing_when_disabled() {
    let mut list = List::new(vec!["a".to_string(), "b".to_string()]);
    list.focus();

    let cmd = list.update(Message::KeyPress(key(KeyCode::Char(' '))));
    // Space should not emit Toggled when multi_select is false
    assert!(cmd.is_none());
    assert!(list.selected_items().is_empty());
}

#[test]
fn multi_select_clear_selections() {
    let mut list = List::new(vec!["a".to_string(), "b".to_string(), "c".to_string()])
        .with_multi_select(true);
    list.focus();

    list.update(Message::KeyPress(key(KeyCode::Char(' '))));
    list.update(Message::KeyPress(key(KeyCode::Down)));
    list.update(Message::KeyPress(key(KeyCode::Char(' '))));
    assert_eq!(list.selected_items().len(), 2);

    list.clear_selections();
    assert!(list.selected_items().is_empty());
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p boba-widgets multi_select`
Expected: FAIL — `with_multi_select`, `selected_items`, `Toggled` don't exist.

**Step 3: Implement multi-select**

Add `use std::collections::BTreeSet;` to the top of `list.rs`.

Add `Toggled` variant to the `Message` enum:

```rust
/// An item was toggled in multi-select mode (index, now_selected).
Toggled(usize, bool),
```

Add fields to the `List` struct:

```rust
multi_select: bool,
selected_set: BTreeSet<usize>,
```

Initialize in `new()`:

```rust
multi_select: false,
selected_set: BTreeSet::new(),
```

Add builder and accessor methods:

```rust
/// Enable multi-select mode. When enabled, Space toggles items and
/// `selected_items()` returns all toggled indices.
pub fn with_multi_select(mut self, enabled: bool) -> Self {
    self.multi_select = enabled;
    self
}

/// Get the set of selected item indices (multi-select mode).
/// Returns an empty set when multi-select is disabled.
pub fn selected_items(&self) -> &BTreeSet<usize> {
    &self.selected_set
}

/// Clear all multi-select selections.
pub fn clear_selections(&mut self) {
    self.selected_set.clear();
}
```

In the key handling section of `update()` (the `Message::KeyPress(key) if self.focus` arm), add a Space handler. Insert it before the `confirm` binding check (around where `self.key_bindings.confirm.matches(&key)` is checked):

```rust
} else if self.multi_select && key.code == KeyCode::Char(' ') && key.modifiers == KeyModifiers::NONE {
    if let Some(original_idx) = self.selected() {
        let toggled_on = if self.selected_set.contains(&original_idx) {
            self.selected_set.remove(&original_idx);
            false
        } else {
            self.selected_set.insert(original_idx);
            true
        };
        return Command::message(Message::Toggled(original_idx, toggled_on));
    }
    Command::none()
```

In `set_items()`, clear the selection set:

```rust
self.selected_set.clear();
```

In the `ListStyle` struct, add a style for multi-selected items:

```rust
/// Style for items in the selected set (multi-select mode).
pub multi_selected: Style,
```

Initialize in `ListStyle::default()`:

```rust
multi_selected: Style::default().fg(Color::Green),
```

In the `view()` method, when rendering items, apply `multi_selected` style to items in `selected_set`. Find the item rendering loop and wrap the item style:

Where the item style is computed (the selected vs normal style check), add:

```rust
let is_multi = self.multi_select && self.selected_set.contains(&original_idx);
```

And apply `self.style.multi_selected` when `is_multi` is true (merge it with the base style).

**Step 4: Run tests to verify they pass**

Run: `cargo test -p boba-widgets multi_select`
Expected: PASS

**Step 5: Run full suite**

Run: `cargo test -p boba-widgets`
Expected: All tests pass.

**Step 6: Commit**

```bash
git add crates/boba-widgets/src/list.rs
git commit -m "feat(list): add with_multi_select() for toggling multiple items"
```

---

### Task 5: FocusGroup Documentation and Example

**Files:**
- Modify: `crates/boba-widgets/src/focus.rs`
- Modify: `crates/boba-widgets/src/lib.rs` (doc table)
- Modify: `examples/input_form.rs` (already uses FocusGroup — improve/annotate)

**Context:** FocusGroup exists and works but is under-documented. The doc comment on the struct is a single line. We need a module-level doc, a usage example in the struct doc, and make sure the lib.rs widget table points users to it clearly. The `input_form` example already demonstrates FocusGroup usage but could use comments explaining the focus pattern.

**Step 1: Add ABOUTME and module-level docs to focus.rs**

At the top of `focus.rs`, before the existing `//!` line:

```rust
// ABOUTME: Focus management utility for cycling keyboard focus across N components.
// ABOUTME: Provides FocusGroup<N> with next/prev/direct focus and is_focused queries.
```

Expand the struct doc comment to include an example:

```rust
/// A utility to simplify the common pattern of routing keyboard input
/// to the focused component. `N` is the number of focusable slots.
///
/// # Example
///
/// ```ignore
/// use boba_widgets::focus::FocusGroup;
///
/// // Three focusable panes: sidebar, main content, detail panel
/// let mut focus = FocusGroup::<3>::new();
///
/// // Tab to cycle focus
/// focus.focus_next(); // now on slot 1 (main content)
///
/// // Route input based on focus
/// match focus.focused() {
///     0 => { /* send keys to sidebar */ }
///     1 => { /* send keys to main content */ }
///     2 => { /* send keys to detail panel */ }
///     _ => unreachable!(),
/// }
///
/// // Check specific slot
/// if focus.is_focused(1) {
///     // main content has focus
/// }
/// ```
```

**Step 2: Update lib.rs doc table**

In the widget reference table in `lib.rs`, ensure `FocusGroup` has a clear entry. If there's a table of widgets, verify the `focus` line mentions "cycle keyboard focus across N components with Tab/Shift+Tab".

**Step 3: Add comments to input_form.rs**

Read `examples/input_form.rs` and add brief comments explaining the FocusGroup pattern where it's used (focus routing in update, focus-aware rendering in view). Don't change any logic.

**Step 4: Run full suite**

Run: `cargo test -p boba-widgets`
Expected: All tests pass (no logic changes, only docs).

**Step 5: Commit**

```bash
git add crates/boba-widgets/src/focus.rs crates/boba-widgets/src/lib.rs examples/input_form.rs
git commit -m "docs(focus): improve FocusGroup documentation and example annotations"
```
