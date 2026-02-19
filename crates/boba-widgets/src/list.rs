//! Selectable list component with filtering, multi-select, custom item
//! delegates, spinner integration, and item descriptions.

use boba_core::command::Command;
use boba_core::component::Component;
use boba_core::key_sequence::KeySequenceTracker;
use boba_core::subscription::Subscription;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::key::Binding;
use crate::selection::SelectionState;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, HighlightSpacing, List as RatatuiList, ListItem, ListState, Paragraph,
};
use ratatui::Frame;
use std::cell::Cell;

/// Trait for types that can be displayed in a list.
///
/// Any domain type can implement this trait to be used directly with [`List`]
/// without converting into an intermediate struct. This mirrors the Bubble Tea
/// `Item` interface where only `FilterValue() string` is required.
///
/// # Example
///
/// ```ignore
/// struct Task {
///     title: String,
///     priority: u8,
///     done: bool,
/// }
///
/// impl list::Item for Task {
///     fn filter_value(&self) -> &str {
///         &self.title
///     }
///     fn description(&self) -> Option<&str> {
///         None
///     }
/// }
///
/// // Use domain types directly — no conversion needed
/// let list = List::new(vec![
///     Task { title: "Fix bug".into(), priority: 1, done: false },
///     Task { title: "Write tests".into(), priority: 2, done: true },
/// ]);
/// ```
pub trait Item: Send + 'static {
    /// Text used for filtering and as the default display label.
    fn filter_value(&self) -> &str;

    /// Optional description shown below the label by the default delegate.
    /// Returns `None` by default.
    fn description(&self) -> Option<&str> {
        None
    }
}

impl Item for String {
    fn filter_value(&self) -> &str {
        self
    }
}

impl Item for &'static str {
    fn filter_value(&self) -> &str {
        self
    }
}

/// Trait for custom list item rendering.
///
/// The type parameter `I` is the concrete item type stored in the list,
/// giving the delegate full access to domain fields for rendering.
///
/// # Example
/// ```ignore
/// struct TaskDelegate;
/// impl ItemDelegate<Task> for TaskDelegate {
///     fn render<'a>(&'a self, task: &'a Task, _index: usize, selected: bool, _width: u16) -> Vec<Line<'a>> {
///         let check = if task.done { "✓" } else { " " };
///         let style = if selected {
///             Style::default().fg(Color::Cyan)
///         } else {
///             Style::default()
///         };
///         vec![Line::styled(format!("[{}] {}", check, task.title), style)]
///     }
/// }
/// ```
pub trait ItemDelegate<I: Item>: Send {
    /// Render a list item. Returns one or more Lines for display.
    /// - `item`: the domain item (full access to all fields)
    /// - `index`: original index in the items list
    /// - `selected`: whether this item is currently selected
    /// - `width`: available width in columns
    fn render<'a>(
        &'a self,
        item: &'a I,
        index: usize,
        selected: bool,
        width: u16,
    ) -> Vec<Line<'a>>;
}

/// Default delegate that renders [`Item::filter_value`] as the label
/// and [`Item::description`] as a dimmed second line when present.
pub struct DefaultDelegate;

impl<I: Item> ItemDelegate<I> for DefaultDelegate {
    fn render<'a>(
        &'a self,
        item: &'a I,
        _index: usize,
        _selected: bool,
        _width: u16,
    ) -> Vec<Line<'a>> {
        let mut lines = vec![Line::raw(item.filter_value())];
        if let Some(desc) = item.description() {
            lines.push(Line::styled(
                desc,
                Style::default().fg(Color::DarkGray),
            ));
        }
        lines
    }
}

/// Messages for the list component.
#[derive(Debug, Clone)]
pub enum Message {
    /// A key press event forwarded to the list for handling.
    KeyPress(KeyEvent),
    /// The item at the given original index was selected (cursor moved).
    Select(usize),
    /// The item at the given original index was confirmed (Enter pressed).
    Confirm(usize),
    /// The filter text changed to the given value.
    FilterChanged(String),
    /// The filter input was toggled on or off.
    ToggleFilter,
    /// Internal tick used to advance the loading spinner animation.
    SpinnerTick,
}

/// Configurable key bindings for the list component.
///
/// Each field is a [`Binding`](crate::key::Binding) that maps one or more
/// key combinations to an action. The defaults match vim-style navigation.
/// Override individual fields to customise keys:
///
/// ```ignore
/// use boba_widgets::list::ListKeyBindings;
/// use boba_widgets::key::{Binding, KeyCombination};
/// use crossterm::event::KeyCode;
///
/// let mut bindings = ListKeyBindings::default();
/// bindings.confirm = Binding::new(KeyCombination::new(KeyCode::Char(' ')), "Confirm");
/// ```
pub struct ListKeyBindings {
    /// Move selection up. Default: Up, k
    pub up: Binding,
    /// Move selection down. Default: Down, j
    pub down: Binding,
    /// Move to first item. Default: Home
    pub first: Binding,
    /// Move to last item. Default: End, G
    pub last: Binding,
    /// Page down. Default: PageDown
    pub page_down: Binding,
    /// Page up. Default: PageUp
    pub page_up: Binding,
    /// Half page down. Default: Ctrl+D
    pub half_down: Binding,
    /// Half page up. Default: Ctrl+U
    pub half_up: Binding,
    /// Confirm selection. Default: Enter
    pub confirm: Binding,
    /// Toggle filter. Default: /
    pub filter: Binding,
}

impl Default for ListKeyBindings {
    fn default() -> Self {
        use crate::key::{Binding, KeyCombination};
        Self {
            up: Binding::with_keys(
                vec![
                    KeyCombination::new(KeyCode::Up),
                    KeyCombination::new(KeyCode::Char('k')),
                ],
                "Up",
            ),
            down: Binding::with_keys(
                vec![
                    KeyCombination::new(KeyCode::Down),
                    KeyCombination::new(KeyCode::Char('j')),
                ],
                "Down",
            ),
            first: Binding::new(KeyCombination::new(KeyCode::Home), "First"),
            last: Binding::with_keys(
                vec![
                    KeyCombination::new(KeyCode::End),
                    KeyCombination::new(KeyCode::Char('G')),
                    KeyCombination::shift(KeyCode::Char('G')),
                ],
                "Last",
            ),
            page_down: Binding::new(KeyCombination::new(KeyCode::PageDown), "Page down"),
            page_up: Binding::new(KeyCombination::new(KeyCode::PageUp), "Page up"),
            half_down: Binding::new(KeyCombination::ctrl(KeyCode::Char('d')), "Half page down"),
            half_up: Binding::new(KeyCombination::ctrl(KeyCode::Char('u')), "Half page up"),
            confirm: Binding::new(KeyCombination::new(KeyCode::Enter), "Confirm"),
            filter: Binding::new(KeyCombination::new(KeyCode::Char('/')), "Filter"),
        }
    }
}

impl crate::key::KeyMap for ListKeyBindings {
    fn short_help(&self) -> Vec<&Binding> {
        vec![&self.up, &self.down, &self.confirm, &self.filter]
    }

    fn full_help(&self) -> Vec<Vec<&Binding>> {
        vec![
            vec![&self.up, &self.down, &self.first, &self.last],
            vec![
                &self.page_up,
                &self.page_down,
                &self.half_up,
                &self.half_down,
            ],
            vec![&self.confirm, &self.filter],
        ]
    }
}

/// A selectable list with vim-style navigation, filtering, and custom rendering.
///
/// The type parameter `I` is the item type stored in the list. Any type
/// implementing [`Item`] works — from plain `String`s to rich domain structs.
///
/// Items can be filtered interactively by pressing `/`. A loading spinner
/// is shown when `with_loading(true)` is set. Custom item rendering is
/// available through the [`ItemDelegate`] trait which receives `&I` directly,
/// giving full access to domain fields.
///
/// # Example
///
/// ```ignore
/// // Simple: plain strings
/// let list = List::new(vec!["Apple".to_string(), "Banana".to_string()]);
///
/// // Rich: domain types with custom delegate
/// let list = List::new(tasks).with_delegate(TaskDelegate);
/// ```
pub struct List<I: Item> {
    items: Vec<I>,
    state: ListState,
    focus: bool,
    style: ListStyle,
    block: Option<Block<'static>>,
    title: String,
    filter: Option<String>,
    filtering: bool,
    filtered_indices: Vec<usize>,
    selection: SelectionState,
    visible_height: Cell<usize>,
    status_message: Option<String>,
    delegate: Box<dyn ItemDelegate<I>>,
    loading: bool,
    spinner: Option<crate::spinner::Spinner>,
    key_seq: KeySequenceTracker,
    key_bindings: ListKeyBindings,
}

/// Style configuration for the list.
#[derive(Debug, Clone)]
pub struct ListStyle {
    /// Base style for unselected items.
    pub normal: Style,
    /// Style applied to the currently highlighted item.
    pub selected: Style,
    /// Symbol rendered to the left of the selected item (e.g. "▸ ").
    pub highlight_symbol: String,
}

impl Default for ListStyle {
    fn default() -> Self {
        Self {
            normal: Style::default(),
            selected: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            highlight_symbol: "▸ ".to_string(),
        }
    }
}

impl<I: Item> List<I> {
    /// Create a list from a vector of items.
    ///
    /// The first item is selected automatically when the list is non-empty.
    pub fn new(items: Vec<I>) -> Self {
        let mut state = ListState::default();
        if !items.is_empty() {
            state.select(Some(0));
        }
        let count = items.len();
        let filtered_indices = (0..count).collect();
        Self {
            items,
            state,
            focus: false,
            style: ListStyle::default(),
            block: None,
            title: String::new(),
            filter: None,
            filtering: false,
            filtered_indices,
            selection: SelectionState::new(count, 10),
            visible_height: Cell::new(10),
            status_message: None,
            delegate: Box::new(DefaultDelegate),
            loading: false,
            spinner: None,
            key_seq: KeySequenceTracker::new(),
            key_bindings: ListKeyBindings::default(),
        }
    }

    /// Set the list border title.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Set an optional block (border/chrome) around the list.
    ///
    /// When `None` (the default), the list renders borderless, using the
    /// full area. Callers who want a border can supply their own `Block`:
    ///
    /// ```ignore
    /// use ratatui::widgets::{Block, Borders};
    /// let list = List::new(items)
    ///     .with_block(Block::default().borders(Borders::ALL).title("My List"));
    /// ```
    pub fn with_block(mut self, block: Block<'static>) -> Self {
        self.block = Some(block);
        self
    }

    /// Set the list style configuration.
    pub fn with_style(mut self, style: ListStyle) -> Self {
        self.style = style;
        self
    }

    /// Set a custom item delegate for rendering list items.
    pub fn with_delegate(mut self, delegate: impl ItemDelegate<I> + 'static) -> Self {
        self.delegate = Box::new(delegate);
        self
    }

    /// Set custom key bindings for the list.
    pub fn with_key_bindings(mut self, bindings: ListKeyBindings) -> Self {
        self.key_bindings = bindings;
        self
    }

    /// Get a reference to the current key bindings.
    pub fn key_bindings(&self) -> &ListKeyBindings {
        &self.key_bindings
    }

    /// Set the loading state. When loading is true and a spinner is present,
    /// the spinner is rendered at the top of the list area.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        if loading && self.spinner.is_none() {
            self.spinner =
                Some(crate::spinner::Spinner::new("list-spinner").with_title("Loading..."));
        }
        self
    }

    /// Mutable setter for the loading state.
    pub fn set_loading(&mut self, loading: bool) {
        self.loading = loading;
        if loading && self.spinner.is_none() {
            self.spinner =
                Some(crate::spinner::Spinner::new("list-spinner").with_title("Loading..."));
        }
    }

    /// Give focus to the list, enabling keyboard navigation.
    pub fn focus(&mut self) {
        self.focus = true;
    }

    /// Remove focus from the list.
    pub fn blur(&mut self) {
        self.focus = false;
    }

    /// Returns the selected index in the original (unfiltered) items list.
    pub fn selected(&self) -> Option<usize> {
        if self.filtered_indices.is_empty() {
            None
        } else {
            self.filtered_indices.get(self.selection.cursor()).copied()
        }
    }

    /// Programmatically set the selected index (in the original items list).
    /// If the index is out of range, it will be clamped.
    pub fn set_selected(&mut self, index: usize) {
        if self.filtered_indices.is_empty() {
            self.state.select(None);
            return;
        }
        // Find the position of `index` in filtered_indices
        if let Some(pos) = self.filtered_indices.iter().position(|&i| i == index) {
            self.selection.select(pos);
        } else {
            // If not found in filtered view, select the closest or last
            let clamped = index.min(self.filtered_indices.len() - 1);
            self.selection.select(clamped);
        }
        self.sync_list_state();
    }

    /// Return a reference to the currently selected item, if any.
    pub fn selected_item(&self) -> Option<&I> {
        self.selected().and_then(|i| self.items.get(i))
    }

    /// Replace all items, rebuilding the filter and clamping the selection.
    pub fn set_items(&mut self, items: Vec<I>) {
        self.items = items;
        self.rebuild_filtered_indices();
        self.selection.set_count(self.filtered_indices.len());
        self.sync_list_state();
    }

    // --- Filtering ---

    /// Return whether a filter is currently active.
    pub fn is_filtered(&self) -> bool {
        self.filter.is_some()
    }

    /// Return the current filter string, or an empty string if no filter is active.
    pub fn filter_value(&self) -> &str {
        match &self.filter {
            Some(f) => f.as_str(),
            None => "",
        }
    }

    fn rebuild_filtered_indices(&mut self) {
        match &self.filter {
            Some(f) if !f.is_empty() => {
                let lower = f.to_lowercase();
                self.filtered_indices = self
                    .items
                    .iter()
                    .enumerate()
                    .filter(|(_, item)| item.filter_value().to_lowercase().contains(&lower))
                    .map(|(i, _)| i)
                    .collect();
            }
            _ => {
                self.filtered_indices = (0..self.items.len()).collect();
            }
        }
    }

    fn activate_filter(&mut self) {
        self.filtering = true;
        if self.filter.is_none() {
            self.filter = Some(String::new());
        }
    }

    fn deactivate_filter(&mut self) {
        self.filtering = false;
        self.filter = None;
        self.rebuild_filtered_indices();
        self.selection.set_count(self.filtered_indices.len());
        self.selection.home();
        self.sync_list_state();
    }

    fn apply_filter(&mut self, value: String) {
        self.filter = Some(value);
        self.rebuild_filtered_indices();
        self.selection.set_count(self.filtered_indices.len());
        self.selection.home();
        self.sync_list_state();
    }

    // --- Item manipulation ---

    /// Insert an item at the given index, rebuilding the filter.
    pub fn insert_item(&mut self, index: usize, item: I) {
        let index = index.min(self.items.len());
        self.items.insert(index, item);
        self.rebuild_filtered_indices();
        self.selection.set_count(self.filtered_indices.len());
        self.sync_list_state();
    }

    /// Remove and return the item at the given index, if it exists.
    pub fn remove_item(&mut self, index: usize) -> Option<I> {
        if index >= self.items.len() {
            return None;
        }
        let removed = self.items.remove(index);
        self.rebuild_filtered_indices();
        self.selection.set_count(self.filtered_indices.len());
        self.sync_list_state();
        Some(removed)
    }

    /// Replace the item at the given index, rebuilding the filter.
    pub fn set_item(&mut self, index: usize, item: I) {
        if index < self.items.len() {
            self.items[index] = item;
            self.rebuild_filtered_indices();
            self.selection.set_count(self.filtered_indices.len());
            self.sync_list_state();
        }
    }

    /// Return the total number of items (unfiltered).
    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    // --- Status message ---

    /// Set or clear the status message shown below the list.
    pub fn set_status(&mut self, msg: Option<String>) {
        self.status_message = msg;
    }

    // --- Navigation helpers ---

    fn select_next(&mut self) {
        self.sync_selection_visible();
        self.selection.move_down();
        self.sync_list_state();
    }

    fn select_prev(&mut self) {
        self.sync_selection_visible();
        self.selection.move_up();
        self.sync_list_state();
    }

    fn select_first(&mut self) {
        self.sync_selection_visible();
        self.selection.home();
        self.sync_list_state();
    }

    fn select_last(&mut self) {
        self.sync_selection_visible();
        self.selection.end();
        self.sync_list_state();
    }

    fn select_page_down(&mut self) {
        self.sync_selection_visible();
        self.selection.page_down();
        self.sync_list_state();
    }

    fn select_page_up(&mut self) {
        self.sync_selection_visible();
        self.selection.page_up();
        self.sync_list_state();
    }

    fn select_half_page_down(&mut self) {
        self.sync_selection_visible();
        self.selection.half_page_down();
        self.sync_list_state();
    }

    fn select_half_page_up(&mut self) {
        self.sync_selection_visible();
        self.selection.half_page_up();
        self.sync_list_state();
    }

    fn sync_selection_visible(&mut self) {
        let h = self.visible_height.get();
        if self.selection.visible() != h {
            self.selection.set_visible(h);
        }
    }

    fn sync_list_state(&mut self) {
        if self.filtered_indices.is_empty() {
            self.state.select(None);
        } else {
            self.state.select(Some(self.selection.cursor()));
        }
    }
}

impl<I: Item> Component for List<I> {
    type Message = Message;

    fn update(&mut self, msg: Message) -> Command<Message> {
        match msg {
            Message::SpinnerTick => {
                if self.loading {
                    if let Some(ref mut spinner) = self.spinner {
                        let _ = spinner.update(crate::spinner::Message::Tick);
                    }
                }
                Command::none()
            }
            Message::KeyPress(key) if self.focus && self.filtering => {
                match key.code {
                    KeyCode::Esc => {
                        self.deactivate_filter();
                        Command::message(Message::ToggleFilter)
                    }
                    KeyCode::Enter => {
                        // Stop filtering input but keep the filter active
                        self.filtering = false;
                        Command::none()
                    }
                    KeyCode::Backspace => {
                        if let Some(ref mut f) = self.filter {
                            f.pop();
                            let value = f.clone();
                            self.apply_filter(value.clone());
                            Command::message(Message::FilterChanged(value))
                        } else {
                            Command::none()
                        }
                    }
                    KeyCode::Char(c) => {
                        let value = {
                            let f = self.filter.get_or_insert_with(String::new);
                            f.push(c);
                            f.clone()
                        };
                        self.apply_filter(value.clone());
                        Command::message(Message::FilterChanged(value))
                    }
                    // Allow navigation while filtering
                    KeyCode::Up => {
                        self.select_prev();
                        if let Some(i) = self.selected() {
                            return Command::message(Message::Select(i));
                        }
                        Command::none()
                    }
                    KeyCode::Down => {
                        self.select_next();
                        if let Some(i) = self.selected() {
                            return Command::message(Message::Select(i));
                        }
                        Command::none()
                    }
                    _ => Command::none(),
                }
            }
            Message::KeyPress(key) if self.focus => {
                // Check for gg sequence (vim go-to-first)
                if key.code == KeyCode::Char('g') && key.modifiers == KeyModifiers::NONE {
                    if let Some(KeyCode::Char('g')) =
                        self.key_seq.completes_sequence(KeyCode::Char('g'))
                    {
                        self.select_first();
                        if let Some(i) = self.selected() {
                            return Command::message(Message::Select(i));
                        }
                        return Command::none();
                    } else {
                        self.key_seq.set_pending(KeyCode::Char('g'));
                        return Command::none();
                    }
                }
                // Any other key clears a pending sequence
                self.key_seq.clear();
                if self.key_bindings.up.matches(&key) {
                    self.select_prev();
                    if let Some(i) = self.selected() {
                        return Command::message(Message::Select(i));
                    }
                    Command::none()
                } else if self.key_bindings.down.matches(&key) {
                    self.select_next();
                    if let Some(i) = self.selected() {
                        return Command::message(Message::Select(i));
                    }
                    Command::none()
                } else if self.key_bindings.first.matches(&key) {
                    self.select_first();
                    if let Some(i) = self.selected() {
                        return Command::message(Message::Select(i));
                    }
                    Command::none()
                } else if self.key_bindings.last.matches(&key) {
                    self.select_last();
                    if let Some(i) = self.selected() {
                        return Command::message(Message::Select(i));
                    }
                    Command::none()
                } else if self.key_bindings.page_down.matches(&key) {
                    self.select_page_down();
                    if let Some(i) = self.selected() {
                        return Command::message(Message::Select(i));
                    }
                    Command::none()
                } else if self.key_bindings.page_up.matches(&key) {
                    self.select_page_up();
                    if let Some(i) = self.selected() {
                        return Command::message(Message::Select(i));
                    }
                    Command::none()
                } else if self.key_bindings.half_down.matches(&key) {
                    self.select_half_page_down();
                    if let Some(i) = self.selected() {
                        return Command::message(Message::Select(i));
                    }
                    Command::none()
                } else if self.key_bindings.half_up.matches(&key) {
                    self.select_half_page_up();
                    if let Some(i) = self.selected() {
                        return Command::message(Message::Select(i));
                    }
                    Command::none()
                } else if self.key_bindings.confirm.matches(&key) {
                    if let Some(i) = self.selected() {
                        return Command::message(Message::Confirm(i));
                    }
                    Command::none()
                } else if self.key_bindings.filter.matches(&key) {
                    self.activate_filter();
                    Command::message(Message::ToggleFilter)
                } else {
                    Command::none()
                }
            }
            Message::Select(i) => {
                if i < self.items.len() {
                    // Find the position in filtered_indices that maps to original index i
                    if let Some(pos) = self.filtered_indices.iter().position(|&idx| idx == i) {
                        self.selection.select(pos);
                        self.sync_list_state();
                    }
                }
                Command::none()
            }
            Message::FilterChanged(value) => {
                self.apply_filter(value);
                Command::none()
            }
            Message::ToggleFilter => {
                // Note: This message is emitted as a notification by internal key
                // handlers (/ key and Esc). When received externally, it acts as
                // a toggle. Internal dispatch already updates state before emitting.
                Command::none()
            }
            _ => Command::none(),
        }
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        let inner = if let Some(ref block) = self.block {
            let inner = block.inner(area);
            frame.render_widget(block.clone(), area);
            inner
        } else {
            area
        };

        // Determine layout sections
        let has_spinner_line = self.loading && self.spinner.is_some();
        let has_filter_line = self.filtering || self.filter.as_ref().is_some_and(|f| !f.is_empty());
        let has_status = self.status_message.is_some();
        // Show a "Filter: {text}" indicator at the bottom when a non-empty filter is active
        let has_filter_display =
            !self.filtering && self.filter.as_ref().is_some_and(|f| !f.is_empty());

        if inner.height == 0 || inner.width == 0 {
            return;
        }

        // Build constraints for sub-layout
        let mut constraints = Vec::new();
        if has_spinner_line {
            constraints.push(Constraint::Length(1));
        }
        if has_filter_line {
            constraints.push(Constraint::Length(1));
        }
        constraints.push(Constraint::Min(0));
        if has_filter_display {
            constraints.push(Constraint::Length(1));
        }
        if has_status {
            constraints.push(Constraint::Length(1));
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner);

        let mut chunk_idx = 0;

        // Render spinner line
        if has_spinner_line {
            if let Some(ref spinner) = self.spinner {
                spinner.view(frame, chunks[chunk_idx]);
            }
            chunk_idx += 1;
        }

        // Render filter input line
        if has_filter_line {
            let filter_text = self.filter.as_deref().unwrap_or("");
            let filter_line = Line::from(vec![
                Span::styled("/ ", Style::default().fg(Color::Yellow)),
                Span::raw(filter_text),
            ]);
            let filter_paragraph = Paragraph::new(filter_line);
            frame.render_widget(filter_paragraph, chunks[chunk_idx]);
            chunk_idx += 1;
        }

        // Render the list
        let list_area = chunks[chunk_idx];
        self.visible_height.set(if list_area.height > 0 {
            list_area.height as usize
        } else {
            10
        });
        chunk_idx += 1;

        let items: Vec<ListItem> = self
            .filtered_indices
            .iter()
            .enumerate()
            .map(|(pos, &i)| {
                let selected = !self.filtered_indices.is_empty()
                    && pos == self.selection.cursor();
                let lines = self
                    .delegate
                    .render(&self.items[i], i, selected, list_area.width);
                ListItem::new(lines)
            })
            .collect();

        let list = RatatuiList::new(items)
            .highlight_style(self.style.selected)
            .highlight_symbol(self.style.highlight_symbol.as_str())
            .highlight_spacing(HighlightSpacing::Always);

        frame.render_stateful_widget(list, list_area, &mut self.state.clone());

        // Render filter display indicator at bottom
        if has_filter_display {
            let filter_text = self.filter.as_deref().unwrap_or("");
            let display_line = Line::from(Span::styled(
                format!("Filter: {}", filter_text),
                Style::default().fg(Color::DarkGray),
            ));
            let display_paragraph = Paragraph::new(display_line);
            frame.render_widget(display_paragraph, chunks[chunk_idx]);
            chunk_idx += 1;
        }

        // Render status message
        if has_status {
            let status_text = self.status_message.as_deref().unwrap_or("");
            let status_line = Line::from(Span::styled(
                status_text,
                Style::default().fg(Color::DarkGray),
            ));
            let status_paragraph = Paragraph::new(status_line);
            frame.render_widget(status_paragraph, chunks[chunk_idx]);
        }
    }

    fn subscriptions(&self) -> Vec<Subscription<Message>> {
        if self.loading {
            if let Some(ref spinner) = self.spinner {
                return spinner
                    .subscriptions()
                    .into_iter()
                    .map(|sub| sub.map(|_| Message::SpinnerTick))
                    .collect();
            }
        }
        vec![]
    }

    fn focused(&self) -> bool {
        self.focus
    }
}
