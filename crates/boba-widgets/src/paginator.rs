//! Page position indicator showing dot-style pagination.

use boba_core::command::Command;
use boba_core::component::Component;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// The type of pagination indicator to display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaginatorType {
    /// Render dots: ● for active, ○ for inactive.
    Dots,
    /// Render Arabic numerals: "1/5".
    Arabic,
}

/// Messages for the paginator component.
#[derive(Debug, Clone)]
pub enum Message {
    /// Move to the next page.
    NextPage,
    /// Move to the previous page.
    PrevPage,
    /// Jump to a specific page (zero-indexed).
    GotoPage(usize),
}

/// Style configuration for the paginator.
#[derive(Debug, Clone)]
pub struct PaginatorStyle {
    /// Style for the active (current page) dot.
    pub active_dot: Style,
    /// Style for inactive dots.
    pub inactive_dot: Style,
    /// Style for Arabic numeral text (e.g. "2/5").
    pub text: Style,
}

impl Default for PaginatorStyle {
    fn default() -> Self {
        Self {
            active_dot: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            inactive_dot: Style::default().fg(Color::DarkGray),
            text: Style::default(),
        }
    }
}

/// A pagination indicator component.
///
/// Displays the current page position either as a row of dots
/// or as an Arabic numeral fraction (e.g. "2/5").
pub struct Paginator {
    total_pages: usize,
    page: usize,
    per_page: usize,
    paginator_type: PaginatorType,
    style: PaginatorStyle,
}

impl Paginator {
    /// Create a new paginator with the given number of total pages.
    /// Defaults to `PaginatorType::Dots` and 10 items per page.
    pub fn new(total_pages: usize) -> Self {
        Self {
            total_pages: total_pages.max(1),
            page: 0,
            per_page: 10,
            paginator_type: PaginatorType::Dots,
            style: PaginatorStyle::default(),
        }
    }

    /// Set the paginator display type.
    pub fn with_type(mut self, t: PaginatorType) -> Self {
        self.paginator_type = t;
        self
    }

    /// Set the number of items per page.
    pub fn with_per_page(mut self, n: usize) -> Self {
        self.per_page = n.max(1);
        self
    }

    /// Set the paginator style.
    pub fn with_style(mut self, style: PaginatorStyle) -> Self {
        self.style = style;
        self
    }

    /// Get the current page (zero-indexed).
    pub fn page(&self) -> usize {
        self.page
    }

    /// Set the current page (zero-indexed). Clamped to valid range.
    pub fn set_page(&mut self, n: usize) {
        self.page = n.min(self.total_pages.saturating_sub(1));
    }

    /// Get the total number of pages.
    pub fn total_pages(&self) -> usize {
        self.total_pages
    }

    /// Set the total number of pages (minimum 1).
    pub fn set_total_pages(&mut self, n: usize) {
        self.total_pages = n.max(1);
        // Clamp current page if it exceeds new total
        if self.page >= self.total_pages {
            self.page = self.total_pages - 1;
        }
    }

    /// Advance to the next page if not on the last page.
    pub fn next_page(&mut self) {
        if !self.on_last_page() {
            self.page += 1;
        }
    }

    /// Go to the previous page if not on the first page.
    pub fn prev_page(&mut self) {
        if !self.on_first_page() {
            self.page -= 1;
        }
    }

    /// Whether we are on the first page.
    pub fn on_first_page(&self) -> bool {
        self.page == 0
    }

    /// Whether we are on the last page.
    pub fn on_last_page(&self) -> bool {
        self.page >= self.total_pages.saturating_sub(1)
    }

    /// Calculate how many items are on the current page given a total item count.
    ///
    /// For example, with 23 total items, 10 per page, and 3 pages:
    /// - Pages 0 and 1 have 10 items each.
    /// - Page 2 has 3 items.
    pub fn items_on_page(&self, total_items: usize) -> usize {
        if total_items == 0 || self.per_page == 0 {
            return 0;
        }
        let start = self.page * self.per_page;
        if start >= total_items {
            return 0;
        }
        let remaining = total_items - start;
        remaining.min(self.per_page)
    }
}

impl Component for Paginator {
    type Message = Message;

    fn update(&mut self, msg: Message) -> Command<Message> {
        match msg {
            Message::NextPage => {
                self.next_page();
                Command::none()
            }
            Message::PrevPage => {
                self.prev_page();
                Command::none()
            }
            Message::GotoPage(n) => {
                self.set_page(n);
                Command::none()
            }
        }
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        match self.paginator_type {
            PaginatorType::Dots => {
                let mut spans = Vec::with_capacity(self.total_pages * 2);
                for i in 0..self.total_pages {
                    if i > 0 {
                        spans.push(Span::raw(" "));
                    }
                    if i == self.page {
                        spans.push(Span::styled("●", self.style.active_dot));
                    } else {
                        spans.push(Span::styled("○", self.style.inactive_dot));
                    }
                }
                let line = Line::from(spans);
                let paragraph = Paragraph::new(line);
                frame.render_widget(paragraph, area);
            }
            PaginatorType::Arabic => {
                let text = format!("{}/{}", self.page + 1, self.total_pages);
                let span = Span::styled(text, self.style.text);
                let paragraph = Paragraph::new(span);
                frame.render_widget(paragraph, area);
            }
        }
    }
}
