//! Shared selectable-list navigation state.
//!
//! `SelectionState` tracks a cursor position and scroll offset for a
//! collection of selectable items, providing wrapping move_up/move_down,
//! page navigation, and home/end operations.

/// Tracks cursor position and scroll offset for a selectable collection.
pub struct SelectionState {
    cursor: usize,
    offset: usize,
    count: usize,
    visible: usize,
}

impl SelectionState {
    pub fn new(count: usize, visible: usize) -> Self {
        Self {
            cursor: 0,
            offset: 0,
            count,
            visible,
        }
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }
    pub fn offset(&self) -> usize {
        self.offset
    }
    pub fn count(&self) -> usize {
        self.count
    }
    pub fn visible(&self) -> usize {
        self.visible
    }

    pub fn set_count(&mut self, count: usize) {
        self.count = count;
        if self.count == 0 {
            self.cursor = 0;
            self.offset = 0;
        } else if self.cursor >= self.count {
            self.cursor = self.count - 1;
        }
        self.ensure_visible();
    }

    pub fn set_visible(&mut self, visible: usize) {
        self.visible = visible;
        self.ensure_visible();
    }

    pub fn move_up(&mut self) {
        if self.count == 0 {
            return;
        }
        self.cursor = if self.cursor == 0 {
            self.count - 1
        } else {
            self.cursor - 1
        };
        self.ensure_visible();
    }

    pub fn move_down(&mut self) {
        if self.count == 0 {
            return;
        }
        self.cursor = if self.cursor + 1 >= self.count {
            0
        } else {
            self.cursor + 1
        };
        self.ensure_visible();
    }

    pub fn page_up(&mut self) {
        if self.count == 0 {
            return;
        }
        self.cursor = self.cursor.saturating_sub(self.visible);
        self.ensure_visible();
    }

    pub fn page_down(&mut self) {
        if self.count == 0 {
            return;
        }
        self.cursor = (self.cursor + self.visible).min(self.count - 1);
        self.ensure_visible();
    }

    pub fn half_page_up(&mut self) {
        if self.count == 0 {
            return;
        }
        let half = self.visible / 2;
        self.cursor = self.cursor.saturating_sub(half);
        self.ensure_visible();
    }

    pub fn half_page_down(&mut self) {
        if self.count == 0 {
            return;
        }
        let half = self.visible / 2;
        self.cursor = (self.cursor + half).min(self.count - 1);
        self.ensure_visible();
    }

    pub fn home(&mut self) {
        self.cursor = 0;
        self.ensure_visible();
    }

    pub fn end(&mut self) {
        if self.count > 0 {
            self.cursor = self.count - 1;
        }
        self.ensure_visible();
    }

    pub fn select(&mut self, index: usize) {
        if self.count == 0 {
            return;
        }
        self.cursor = index.min(self.count - 1);
        self.ensure_visible();
    }

    fn ensure_visible(&mut self) {
        if self.count == 0 || self.visible == 0 {
            return;
        }
        if self.cursor < self.offset {
            self.offset = self.cursor;
        } else if self.cursor >= self.offset + self.visible {
            self.offset = self.cursor + 1 - self.visible;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_with_items() {
        let s = SelectionState::new(5, 3);
        assert_eq!(s.cursor(), 0);
        assert_eq!(s.offset(), 0);
    }

    #[test]
    fn move_down_wraps() {
        let mut s = SelectionState::new(3, 10);
        s.move_down(); // 0 -> 1
        assert_eq!(s.cursor(), 1);
        s.move_down(); // 1 -> 2
        assert_eq!(s.cursor(), 2);
        s.move_down(); // 2 -> 0 (wrap)
        assert_eq!(s.cursor(), 0);
    }

    #[test]
    fn move_up_wraps() {
        let mut s = SelectionState::new(3, 10);
        s.move_up(); // 0 -> 2 (wrap)
        assert_eq!(s.cursor(), 2);
        s.move_up(); // 2 -> 1
        assert_eq!(s.cursor(), 1);
    }

    #[test]
    fn page_down_clamps() {
        let mut s = SelectionState::new(20, 5);
        s.page_down(); // 0 -> 5
        assert_eq!(s.cursor(), 5);
        s.select(18);
        s.page_down(); // 18 -> 19 (clamped)
        assert_eq!(s.cursor(), 19);
    }

    #[test]
    fn page_up_clamps() {
        let mut s = SelectionState::new(20, 5);
        s.select(10);
        s.page_up(); // 10 -> 5
        assert_eq!(s.cursor(), 5);
        s.select(2);
        s.page_up(); // 2 -> 0 (clamped)
        assert_eq!(s.cursor(), 0);
    }

    #[test]
    fn half_page() {
        let mut s = SelectionState::new(20, 10);
        s.half_page_down(); // 0 -> 5
        assert_eq!(s.cursor(), 5);
        s.half_page_up(); // 5 -> 0
        assert_eq!(s.cursor(), 0);
    }

    #[test]
    fn home_end() {
        let mut s = SelectionState::new(10, 5);
        s.end(); // -> 9
        assert_eq!(s.cursor(), 9);
        s.home(); // -> 0
        assert_eq!(s.cursor(), 0);
    }

    #[test]
    fn select_clamps() {
        let mut s = SelectionState::new(5, 3);
        s.select(100); // clamped to 4
        assert_eq!(s.cursor(), 4);
    }

    #[test]
    fn empty_count_is_noop() {
        let mut s = SelectionState::new(0, 5);
        s.move_down();
        assert_eq!(s.cursor(), 0);
        s.move_up();
        assert_eq!(s.cursor(), 0);
    }

    #[test]
    fn ensure_visible_adjusts_offset() {
        let mut s = SelectionState::new(20, 5);
        s.select(10); // cursor at 10, offset should adjust
        assert!(s.offset() <= 10);
        assert!(s.offset() + 5 > 10);
    }

    #[test]
    fn set_count_clamps_cursor() {
        let mut s = SelectionState::new(10, 5);
        s.select(8);
        s.set_count(5); // cursor should clamp to 4
        assert_eq!(s.cursor(), 4);
    }
}
