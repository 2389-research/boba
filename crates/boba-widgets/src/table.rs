//! Data table component with row and column navigation, sorting, per-row
//! styling, and CSV parsing.

use boba_core::command::Command;
use boba_core::component::Component;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{
    Block, Borders, Cell as RatatuiCell, Row, Table as RatatuiTable, TableState,
};
use ratatui::Frame;
use std::cell::Cell as StdCell;

/// Messages for the table component.
#[derive(Debug, Clone)]
pub enum Message {
    /// A key press event forwarded to the table for handling.
    KeyPress(KeyEvent),
    /// The row at the given index was selected (cursor moved).
    SelectRow(usize),
    /// The row at the given index was confirmed (Enter pressed).
    Confirm(usize),
}

/// A data table with column/row navigation.
///
/// Renders column headers and data rows with keyboard-driven selection.
/// Supports per-row styling, column-level cell navigation, and simple
/// CSV parsing.
///
/// Configurable key bindings for the table component.
pub struct TableKeyBindings {
    /// Move selection up. Default: Up, k
    pub up: crate::key::Binding,
    /// Move selection down. Default: Down, j
    pub down: crate::key::Binding,
    /// Move column left. Default: Left, h
    pub col_left: crate::key::Binding,
    /// Move column right. Default: Right, l
    pub col_right: crate::key::Binding,
    /// Next column (wrapping). Default: Tab
    pub col_next: crate::key::Binding,
    /// Move to first row. Default: Home
    pub first: crate::key::Binding,
    /// Move to last row. Default: End, G
    pub last: crate::key::Binding,
    /// Page up. Default: PageUp
    pub page_up: crate::key::Binding,
    /// Page down. Default: PageDown
    pub page_down: crate::key::Binding,
    /// Half page down. Default: Ctrl+D
    pub half_down: crate::key::Binding,
    /// Half page up. Default: Ctrl+U
    pub half_up: crate::key::Binding,
    /// Confirm selection. Default: Enter
    pub confirm: crate::key::Binding,
}

impl Default for TableKeyBindings {
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
            col_left: Binding::with_keys(
                vec![
                    KeyCombination::new(KeyCode::Left),
                    KeyCombination::new(KeyCode::Char('h')),
                ],
                "Column left",
            ),
            col_right: Binding::with_keys(
                vec![
                    KeyCombination::new(KeyCode::Right),
                    KeyCombination::new(KeyCode::Char('l')),
                ],
                "Column right",
            ),
            col_next: Binding::new(KeyCombination::new(KeyCode::Tab), "Next column"),
            first: Binding::new(KeyCombination::new(KeyCode::Home), "First"),
            last: Binding::with_keys(
                vec![
                    KeyCombination::new(KeyCode::End),
                    KeyCombination::new(KeyCode::Char('G')),
                    KeyCombination::shift(KeyCode::Char('G')),
                ],
                "Last",
            ),
            page_up: Binding::new(KeyCombination::new(KeyCode::PageUp), "Page up"),
            page_down: Binding::new(KeyCombination::new(KeyCode::PageDown), "Page down"),
            half_down: Binding::new(KeyCombination::ctrl(KeyCode::Char('d')), "Half page down"),
            half_up: Binding::new(KeyCombination::ctrl(KeyCode::Char('u')), "Half page up"),
            confirm: Binding::new(KeyCombination::new(KeyCode::Enter), "Confirm"),
        }
    }
}

impl crate::key::KeyMap for TableKeyBindings {
    fn short_help(&self) -> Vec<&crate::key::Binding> {
        vec![&self.up, &self.down, &self.confirm]
    }

    fn full_help(&self) -> Vec<Vec<&crate::key::Binding>> {
        vec![
            vec![&self.up, &self.down, &self.first, &self.last],
            vec![&self.col_left, &self.col_right, &self.col_next],
            vec![
                &self.page_up,
                &self.page_down,
                &self.half_up,
                &self.half_down,
            ],
            vec![&self.confirm],
        ]
    }
}

/// # Example
///
/// ```ignore
/// let headers = vec!["Name".into(), "Age".into()];
/// let rows = vec![
///     vec!["Alice".into(), "30".into()],
///     vec!["Bob".into(), "25".into()],
/// ];
/// let mut table = Table::new(headers, rows).with_title("People");
/// table.focus();
/// ```
pub struct Table {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    widths: Vec<Constraint>,
    state: TableState,
    focus: bool,
    style: TableStyle,
    title: String,
    visible_height: StdCell<usize>,
    selected_col: Option<usize>,
    row_style_fn: Option<RowStyleFn>,
    key_seq: boba_core::key_sequence::KeySequenceTracker,
    key_bindings: TableKeyBindings,
}

type RowStyleFn = Box<dyn Fn(usize, &[String]) -> Style + Send>;

/// Style configuration for the table.
#[derive(Debug, Clone)]
pub struct TableStyle {
    /// Style applied to column header cells.
    pub header: Style,
    /// Base style for unselected data rows.
    pub normal: Style,
    /// Style applied to the currently highlighted row.
    pub selected: Style,
    /// Border style when the table has focus.
    pub focused_border: Style,
    /// Border style when the table does not have focus.
    pub unfocused_border: Style,
    /// Symbol rendered to the left of the selected row (e.g. "▸ ").
    pub highlight_symbol: String,
    /// Style applied to the active cell when column navigation is enabled.
    pub active_cell: Style,
}

impl Default for TableStyle {
    fn default() -> Self {
        Self {
            header: Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            normal: Style::default(),
            selected: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            focused_border: Style::default().fg(Color::Cyan),
            unfocused_border: Style::default().fg(Color::DarkGray),
            highlight_symbol: "▸ ".to_string(),
            active_cell: Style::default()
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::UNDERLINED),
        }
    }
}

impl Table {
    /// Create a table with the given column headers and rows.
    ///
    /// Column widths default to equal percentages. The first row is selected
    /// automatically when `rows` is non-empty.
    pub fn new(headers: Vec<String>, rows: Vec<Vec<String>>) -> Self {
        let col_count = headers.len();
        let widths = if col_count == 0 {
            Vec::new()
        } else {
            vec![Constraint::Percentage(100 / col_count as u16); col_count]
        };
        let mut state = TableState::default();
        if !rows.is_empty() {
            state.select(Some(0));
        }
        Self {
            headers,
            rows,
            widths,
            state,
            focus: false,
            style: TableStyle::default(),
            title: String::new(),
            visible_height: StdCell::new(10),
            selected_col: None,
            row_style_fn: None,
            key_seq: boba_core::key_sequence::KeySequenceTracker::new(),
            key_bindings: TableKeyBindings::default(),
        }
    }

    /// Set custom key bindings for the table.
    pub fn with_key_bindings(mut self, bindings: TableKeyBindings) -> Self {
        self.key_bindings = bindings;
        self
    }

    /// Get a reference to the current key bindings.
    pub fn key_bindings(&self) -> &TableKeyBindings {
        &self.key_bindings
    }

    /// Create a table from simple CSV data.
    ///
    /// The first non-empty line is treated as column headers. Remaining non-empty
    /// lines become data rows. Fields are split by comma and trimmed. Quoted fields
    /// and escaped commas are not handled.
    pub fn from_csv(data: &str) -> Self {
        let lines: Vec<&str> = data.lines().filter(|l| !l.trim().is_empty()).collect();
        if lines.is_empty() {
            return Self::new(Vec::new(), Vec::new());
        }

        let headers: Vec<String> = lines[0].split(',').map(|s| s.trim().to_string()).collect();
        let rows: Vec<Vec<String>> = lines[1..]
            .iter()
            .map(|line| line.split(',').map(|s| s.trim().to_string()).collect())
            .collect();

        Self::new(headers, rows)
    }

    /// Override the column width constraints.
    pub fn with_widths(mut self, widths: Vec<Constraint>) -> Self {
        self.widths = widths;
        self
    }

    /// Set the table border title.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Set the table style configuration.
    pub fn with_style(mut self, style: TableStyle) -> Self {
        self.style = style;
        self
    }

    /// Set a per-row styling function. The function receives the row index and
    /// the row data, and returns a `Style` to use as the base style for that row.
    /// This enables alternating row colors, conditional highlighting, etc.
    pub fn with_row_style(
        mut self,
        f: impl Fn(usize, &[String]) -> Style + Send + 'static,
    ) -> Self {
        self.row_style_fn = Some(Box::new(f));
        self
    }

    /// Give focus to the table, enabling keyboard navigation.
    pub fn focus(&mut self) {
        self.focus = true;
    }

    /// Remove focus from the table.
    pub fn blur(&mut self) {
        self.focus = false;
    }

    /// Return the index of the currently selected row, if any.
    pub fn selected(&self) -> Option<usize> {
        self.state.selected()
    }

    /// Get the currently selected column index, if column navigation is active.
    pub fn selected_column(&self) -> Option<usize> {
        self.selected_col
    }

    /// Set the selected column. Pass `None` to disable column navigation.
    pub fn set_selected_column(&mut self, col: Option<usize>) {
        self.selected_col = col;
    }

    /// Replace the data rows, clamping the selection to the new bounds.
    pub fn set_rows(&mut self, rows: Vec<Vec<String>>) {
        self.rows = rows;
        if self.rows.is_empty() {
            self.state.select(None);
        } else if let Some(i) = self.state.selected() {
            if i >= self.rows.len() {
                self.state.select(Some(self.rows.len() - 1));
            }
        }
    }

    fn select_next(&mut self) {
        if self.rows.is_empty() {
            return;
        }
        let i = self.state.selected().unwrap_or(0);
        let next = if i + 1 >= self.rows.len() { 0 } else { i + 1 };
        self.state.select(Some(next));
    }

    fn select_prev(&mut self) {
        if self.rows.is_empty() {
            return;
        }
        let i = self.state.selected().unwrap_or(0);
        let prev = if i == 0 {
            self.rows.len().saturating_sub(1)
        } else {
            i - 1
        };
        self.state.select(Some(prev));
    }

    /// Move the cursor up by `n` rows, clamped to the first row.
    pub fn move_up(&mut self, n: usize) {
        if self.rows.is_empty() {
            return;
        }
        let i = self.state.selected().unwrap_or(0);
        self.state.select(Some(i.saturating_sub(n)));
    }

    /// Move the cursor down by `n` rows, clamped to the last row.
    pub fn move_down(&mut self, n: usize) {
        if self.rows.is_empty() {
            return;
        }
        let i = self.state.selected().unwrap_or(0);
        let last = self.rows.len().saturating_sub(1);
        self.state.select(Some((i + n).min(last)));
    }

    /// Jump to a specific row index, clamped to valid range.
    pub fn set_cursor(&mut self, n: usize) {
        if self.rows.is_empty() {
            return;
        }
        let last = self.rows.len().saturating_sub(1);
        self.state.select(Some(n.min(last)));
    }

    /// Alias for `selected()`.
    pub fn cursor(&self) -> Option<usize> {
        self.selected()
    }

    /// Update columns dynamically.
    pub fn set_columns(&mut self, headers: Vec<String>, widths: Vec<Constraint>) {
        self.headers = headers;
        self.widths = widths;
    }

    /// Get the number of rows.
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Move the selected column left.
    fn move_col_left(&mut self) {
        if self.headers.is_empty() {
            return;
        }
        match self.selected_col {
            Some(c) => self.selected_col = Some(c.saturating_sub(1)),
            None => self.selected_col = Some(0),
        }
    }

    /// Move the selected column right.
    fn move_col_right(&mut self) {
        if self.headers.is_empty() {
            return;
        }
        let max_col = self.headers.len().saturating_sub(1);
        match self.selected_col {
            Some(c) => self.selected_col = Some((c + 1).min(max_col)),
            None => self.selected_col = Some(0),
        }
    }

    /// Move to the next column, wrapping around to the first.
    fn move_col_next_wrap(&mut self) {
        if self.headers.is_empty() {
            return;
        }
        match self.selected_col {
            Some(c) => {
                self.selected_col = Some((c + 1) % self.headers.len());
            }
            None => self.selected_col = Some(0),
        }
    }
}

impl Component for Table {
    type Message = Message;

    fn update(&mut self, msg: Message) -> Command<Message> {
        match msg {
            Message::KeyPress(key) if self.focus => {
                // Check for gg sequence (vim go-to-first)
                if key.code == KeyCode::Char('g') && !key.modifiers.contains(KeyModifiers::SHIFT) {
                    if let Some(KeyCode::Char('g')) =
                        self.key_seq.completes_sequence(KeyCode::Char('g'))
                    {
                        self.set_cursor(0);
                        if let Some(i) = self.selected() {
                            return Command::message(Message::SelectRow(i));
                        }
                        return Command::none();
                    } else {
                        self.key_seq.set_pending(KeyCode::Char('g'));
                        return Command::none();
                    }
                }
                self.key_seq.clear();
                if self.key_bindings.up.matches(&key) {
                    self.select_prev();
                    if let Some(i) = self.selected() {
                        return Command::message(Message::SelectRow(i));
                    }
                    Command::none()
                } else if self.key_bindings.down.matches(&key) {
                    self.select_next();
                    if let Some(i) = self.selected() {
                        return Command::message(Message::SelectRow(i));
                    }
                    Command::none()
                } else if self.key_bindings.col_left.matches(&key) {
                    self.move_col_left();
                    Command::none()
                } else if self.key_bindings.col_right.matches(&key) {
                    self.move_col_right();
                    Command::none()
                } else if self.key_bindings.col_next.matches(&key) {
                    self.move_col_next_wrap();
                    Command::none()
                } else if self.key_bindings.page_up.matches(&key) {
                    self.move_up(self.visible_height.get());
                    if let Some(i) = self.selected() {
                        return Command::message(Message::SelectRow(i));
                    }
                    Command::none()
                } else if self.key_bindings.page_down.matches(&key) {
                    self.move_down(self.visible_height.get());
                    if let Some(i) = self.selected() {
                        return Command::message(Message::SelectRow(i));
                    }
                    Command::none()
                } else if self.key_bindings.half_down.matches(&key) {
                    self.move_down(self.visible_height.get() / 2);
                    if let Some(i) = self.selected() {
                        return Command::message(Message::SelectRow(i));
                    }
                    Command::none()
                } else if self.key_bindings.half_up.matches(&key) {
                    self.move_up(self.visible_height.get() / 2);
                    if let Some(i) = self.selected() {
                        return Command::message(Message::SelectRow(i));
                    }
                    Command::none()
                } else if self.key_bindings.first.matches(&key) {
                    self.set_cursor(0);
                    if let Some(i) = self.selected() {
                        return Command::message(Message::SelectRow(i));
                    }
                    Command::none()
                } else if self.key_bindings.last.matches(&key) {
                    if !self.rows.is_empty() {
                        self.set_cursor(self.rows.len() - 1);
                        if let Some(i) = self.selected() {
                            return Command::message(Message::SelectRow(i));
                        }
                    }
                    Command::none()
                } else if self.key_bindings.confirm.matches(&key) {
                    if let Some(i) = self.selected() {
                        return Command::message(Message::Confirm(i));
                    }
                    Command::none()
                } else {
                    Command::none()
                }
            }
            Message::SelectRow(i) => {
                if i < self.rows.len() {
                    self.state.select(Some(i));
                }
                Command::none()
            }
            _ => Command::none(),
        }
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        let border_style = if self.focus {
            self.style.focused_border
        } else {
            self.style.unfocused_border
        };

        let mut block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style);

        if !self.title.is_empty() {
            block = block.title(self.title.as_str());
        }

        // Track visible height for page navigation (border top/bottom + header + margin).
        let inner_height = block.inner(area).height as usize;
        // header row (1) + bottom margin (1) = 2 rows consumed by header
        let data_height = inner_height.saturating_sub(2);
        self.visible_height
            .set(if data_height > 0 { data_height } else { 10 });

        let header_cells: Vec<RatatuiCell> = self
            .headers
            .iter()
            .map(|h| RatatuiCell::from(h.as_str()).style(self.style.header))
            .collect();
        let header = Row::new(header_cells).height(1).bottom_margin(1);

        let selected_row = self.state.selected();

        let rows: Vec<Row> = self
            .rows
            .iter()
            .enumerate()
            .map(|(row_idx, row)| {
                let base_style = if let Some(ref style_fn) = self.row_style_fn {
                    style_fn(row_idx, row)
                } else {
                    self.style.normal
                };

                let is_selected_row = selected_row == Some(row_idx);

                let cells: Vec<RatatuiCell> = row
                    .iter()
                    .enumerate()
                    .map(|(col_idx, c)| {
                        let cell = RatatuiCell::from(c.as_str());
                        // If this is the active cell (selected row + selected col),
                        // apply the active cell style.
                        if is_selected_row {
                            if let Some(sel_col) = self.selected_col {
                                if col_idx == sel_col {
                                    return cell.style(self.style.active_cell);
                                }
                            }
                        }
                        cell
                    })
                    .collect();

                Row::new(cells).style(base_style)
            })
            .collect();

        let table = RatatuiTable::new(rows, &self.widths)
            .header(header)
            .block(block)
            .row_highlight_style(self.style.selected)
            .highlight_symbol(self.style.highlight_symbol.as_str());

        frame.render_stateful_widget(table, area, &mut self.state.clone());
    }

    fn focused(&self) -> bool {
        self.focus
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key_event(code: KeyCode) -> Message {
        Message::KeyPress(KeyEvent::new(code, KeyModifiers::NONE))
    }

    fn sample_table() -> Table {
        let headers = vec!["Name".into(), "Age".into(), "City".into()];
        let rows = vec![
            vec!["Alice".into(), "30".into(), "NYC".into()],
            vec!["Bob".into(), "25".into(), "LA".into()],
            vec!["Carol".into(), "35".into(), "SF".into()],
        ];
        let mut t = Table::new(headers, rows);
        t.focus();
        t
    }

    // ── Column navigation tests ──

    #[test]
    fn col_nav_right_from_none_sets_zero() {
        let mut t = sample_table();
        assert_eq!(t.selected_column(), None);
        t.update(key_event(KeyCode::Right));
        assert_eq!(t.selected_column(), Some(0));
    }

    #[test]
    fn col_nav_left_from_none_sets_zero() {
        let mut t = sample_table();
        assert_eq!(t.selected_column(), None);
        t.update(key_event(KeyCode::Left));
        assert_eq!(t.selected_column(), Some(0));
    }

    #[test]
    fn col_nav_right_increments() {
        let mut t = sample_table();
        t.set_selected_column(Some(0));
        t.update(key_event(KeyCode::Right));
        assert_eq!(t.selected_column(), Some(1));
        t.update(key_event(KeyCode::Right));
        assert_eq!(t.selected_column(), Some(2));
    }

    #[test]
    fn col_nav_right_capped_at_last() {
        let mut t = sample_table();
        t.set_selected_column(Some(2)); // last column (3 columns: 0,1,2)
        t.update(key_event(KeyCode::Right));
        assert_eq!(t.selected_column(), Some(2));
    }

    #[test]
    fn col_nav_left_decrements() {
        let mut t = sample_table();
        t.set_selected_column(Some(2));
        t.update(key_event(KeyCode::Left));
        assert_eq!(t.selected_column(), Some(1));
    }

    #[test]
    fn col_nav_left_saturates_at_zero() {
        let mut t = sample_table();
        t.set_selected_column(Some(0));
        t.update(key_event(KeyCode::Left));
        assert_eq!(t.selected_column(), Some(0));
    }

    #[test]
    fn col_nav_tab_wraps() {
        let mut t = sample_table();
        t.set_selected_column(Some(2)); // last column
        t.update(key_event(KeyCode::Tab));
        assert_eq!(t.selected_column(), Some(0)); // wraps to first
    }

    #[test]
    fn col_nav_tab_from_none_sets_zero() {
        let mut t = sample_table();
        t.update(key_event(KeyCode::Tab));
        assert_eq!(t.selected_column(), Some(0));
    }

    #[test]
    fn col_nav_tab_advances() {
        let mut t = sample_table();
        t.set_selected_column(Some(0));
        t.update(key_event(KeyCode::Tab));
        assert_eq!(t.selected_column(), Some(1));
    }

    // ── CSV parsing tests ──

    #[test]
    fn from_csv_simple() {
        let csv = "Name, Age, City\nAlice, 30, NYC\nBob, 25, LA\n";
        let t = Table::from_csv(csv);
        assert_eq!(t.headers, vec!["Name", "Age", "City"]);
        assert_eq!(t.rows.len(), 2);
        assert_eq!(t.rows[0], vec!["Alice", "30", "NYC"]);
        assert_eq!(t.rows[1], vec!["Bob", "25", "LA"]);
    }

    #[test]
    fn from_csv_empty_input() {
        let t = Table::from_csv("");
        assert!(t.headers.is_empty());
        assert!(t.rows.is_empty());
    }

    #[test]
    fn from_csv_only_headers() {
        let t = Table::from_csv("A,B,C\n");
        assert_eq!(t.headers, vec!["A", "B", "C"]);
        assert!(t.rows.is_empty());
    }

    #[test]
    fn from_csv_blank_lines_ignored() {
        let csv = "\n\nName,Age\n\nAlice,30\n\n";
        let t = Table::from_csv(csv);
        assert_eq!(t.headers, vec!["Name", "Age"]);
        assert_eq!(t.rows.len(), 1);
        assert_eq!(t.rows[0], vec!["Alice", "30"]);
    }

    #[test]
    fn from_csv_trims_whitespace() {
        let csv = "  Name  ,  Age  \n  Alice  ,  30  \n";
        let t = Table::from_csv(csv);
        assert_eq!(t.headers, vec!["Name", "Age"]);
        assert_eq!(t.rows[0], vec!["Alice", "30"]);
    }

    // ── Per-row styling tests ──

    #[test]
    fn row_style_fn_stored_correctly() {
        let t = Table::new(vec!["A".into()], vec![vec!["1".into()], vec!["2".into()]])
            .with_row_style(|idx, _row| {
                if idx % 2 == 0 {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::Red)
                }
            });
        assert!(t.row_style_fn.is_some());
    }

    #[test]
    fn row_style_fn_returns_expected_styles() {
        let style_fn = |idx: usize, _row: &[String]| -> Style {
            if idx % 2 == 0 {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Red)
            }
        };
        let t = Table::new(vec!["A".into()], vec![vec!["1".into()], vec!["2".into()]])
            .with_row_style(style_fn);

        // Verify the function returns the expected styles when called directly.
        let f = t.row_style_fn.as_ref().unwrap();
        assert_eq!(f(0, &["1".to_string()]), Style::default().fg(Color::Green));
        assert_eq!(f(1, &["2".to_string()]), Style::default().fg(Color::Red));
    }

    // ── selected_column getter/setter ──

    #[test]
    fn selected_column_getter_setter() {
        let mut t = sample_table();
        assert_eq!(t.selected_column(), None);
        t.set_selected_column(Some(1));
        assert_eq!(t.selected_column(), Some(1));
        t.set_selected_column(None);
        assert_eq!(t.selected_column(), None);
    }
}
