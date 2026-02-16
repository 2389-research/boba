//! Multi-step wizard (form) component with step navigation.
//!
//! The wizard manages a sequence of steps, each defined by a trait object
//! that handles its own rendering and key handling.  The wizard provides
//! a progress indicator, back/next navigation, and validation gating.
//!
//! # Example
//!
//! ```ignore
//! use boba_widgets::wizard::{Wizard, WizardStep, StepTransition};
//!
//! struct NameStep { name: String }
//!
//! impl WizardStep for NameStep {
//!     fn title(&self) -> &str { "Your Name" }
//!     fn is_valid(&self) -> bool { !self.name.is_empty() }
//!     fn handle_key(&mut self, key: KeyEvent) -> StepTransition {
//!         // handle input...
//!         StepTransition::Stay
//!     }
//!     fn render(&self, frame: &mut Frame, area: Rect) {
//!         // render step UI...
//!     }
//! }
//! ```

use boba_core::command::Command;
use boba_core::component::Component;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

/// Transition returned by a step's key handler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepTransition {
    /// Stay on the current step.
    Stay,
    /// Move to the next step (if valid).
    Next,
    /// Go back to the previous step.
    Back,
    /// Complete the wizard.
    Complete,
    /// Cancel the wizard.
    Cancel,
}

/// Trait for individual wizard steps.
///
/// Each step owns its own state (text fields, selections, etc.) and
/// handles its own key events and rendering.
pub trait WizardStep: Send {
    /// The step title shown in the progress bar.
    fn title(&self) -> &str;

    /// Whether the step's data is valid (gates forward navigation).
    fn is_valid(&self) -> bool {
        true
    }

    /// Handle a key event and return a transition.
    fn handle_key(&mut self, key: KeyEvent) -> StepTransition;

    /// Render the step content into the given area.
    fn render(&self, frame: &mut Frame, area: Rect);

    /// Handle periodic tick (for async operations).
    /// Return `Some(transition)` when the async work completes.
    fn handle_tick(&mut self) -> Option<StepTransition> {
        None
    }
}

/// Messages for the wizard component.
#[derive(Debug, Clone)]
pub enum Message {
    /// A key press event.
    KeyPress(KeyEvent),
    /// A tick event (for async step operations).
    Tick,
    /// The wizard was completed.
    Completed,
    /// The wizard was cancelled.
    Cancelled,
}

/// Style configuration for the wizard.
#[derive(Debug, Clone)]
pub struct WizardStyle {
    /// Border style.
    pub border: Style,
    /// Title style.
    pub title: Style,
    /// Progress bar filled style.
    pub progress_filled: Style,
    /// Progress bar empty style.
    pub progress_empty: Style,
    /// Step counter text style.
    pub step_counter: Style,
    /// Navigation hint style.
    pub nav_hint: Style,
}

impl Default for WizardStyle {
    fn default() -> Self {
        Self {
            border: Style::default().fg(Color::Cyan),
            title: Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            progress_filled: Style::default().fg(Color::Cyan),
            progress_empty: Style::default().fg(Color::DarkGray),
            step_counter: Style::default().fg(Color::DarkGray),
            nav_hint: Style::default().fg(Color::DarkGray),
        }
    }
}

/// Multi-step wizard component.
pub struct Wizard {
    steps: Vec<Box<dyn WizardStep>>,
    current: usize,
    style: WizardStyle,
    /// Width as a percentage of the terminal (0-100).
    width_percent: u16,
    /// Height as a percentage of the terminal (0-100).
    height_percent: u16,
}

impl Wizard {
    /// Create a new wizard with the given steps.
    pub fn new(steps: Vec<Box<dyn WizardStep>>) -> Self {
        Self {
            steps,
            current: 0,
            style: WizardStyle::default(),
            width_percent: 70,
            height_percent: 60,
        }
    }

    /// Set the style.
    pub fn with_style(mut self, style: WizardStyle) -> Self {
        self.style = style;
        self
    }

    /// Set the wizard size as a percentage of the terminal.
    pub fn with_size(mut self, width: u16, height: u16) -> Self {
        self.width_percent = width.min(100);
        self.height_percent = height.min(100);
        self
    }

    /// Get the current step index (0-based).
    pub fn current_step(&self) -> usize {
        self.current
    }

    /// Get the total number of steps.
    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    /// Whether we're on the last step.
    pub fn is_last_step(&self) -> bool {
        self.current + 1 >= self.steps.len()
    }

    /// Whether we're on the first step.
    pub fn is_first_step(&self) -> bool {
        self.current == 0
    }

    /// Get the current step title.
    pub fn current_title(&self) -> &str {
        self.steps
            .get(self.current)
            .map(|s| s.title())
            .unwrap_or("")
    }

    fn centered_rect(&self, area: Rect) -> Rect {
        let v_margin = ((100 - self.height_percent) / 2).max(1);
        let h_margin = ((100 - self.width_percent) / 2).max(1);
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(v_margin),
                Constraint::Percentage(self.height_percent),
                Constraint::Percentage(v_margin),
            ])
            .split(area);
        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(h_margin),
                Constraint::Percentage(self.width_percent),
                Constraint::Percentage(h_margin),
            ])
            .split(vertical[1]);
        horizontal[1]
    }
}

impl Component for Wizard {
    type Message = Message;

    fn update(&mut self, msg: Message) -> Command<Message> {
        match msg {
            Message::KeyPress(key) => {
                if self.steps.is_empty() {
                    return Command::none();
                }

                // Global wizard keys
                match (key.code, key.modifiers) {
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                        return Command::message(Message::Cancelled);
                    }
                    (KeyCode::Esc, _) => {
                        if self.is_first_step() {
                            return Command::message(Message::Cancelled);
                        } else {
                            self.current -= 1;
                            return Command::none();
                        }
                    }
                    _ => {}
                }

                // Delegate to current step
                let transition = self.steps[self.current].handle_key(key);
                self.apply_transition(transition)
            }
            Message::Tick => {
                if self.steps.is_empty() {
                    return Command::none();
                }
                if let Some(transition) = self.steps[self.current].handle_tick() {
                    self.apply_transition(transition)
                } else {
                    Command::none()
                }
            }
            Message::Completed | Message::Cancelled => Command::none(),
        }
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        if self.steps.is_empty() {
            return;
        }

        let wizard_area = self.centered_rect(area);
        frame.render_widget(Clear, wizard_area);

        let block = Block::default()
            .title(self.current_title())
            .title_style(self.style.title)
            .borders(Borders::ALL)
            .border_style(self.style.border);

        let inner = block.inner(wizard_area);
        frame.render_widget(block, wizard_area);

        // Layout: progress (1 line) + gap + content + gap + nav hints (1 line)
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Progress bar
                Constraint::Length(1), // Gap
                Constraint::Min(1),    // Step content
                Constraint::Length(1), // Gap
                Constraint::Length(1), // Navigation hints
            ])
            .split(inner);

        // Progress bar
        self.render_progress(frame, chunks[0]);

        // Step content
        self.steps[self.current].render(frame, chunks[2]);

        // Navigation hints
        self.render_nav_hints(frame, chunks[4]);
    }

    fn focused(&self) -> bool {
        true // Wizards capture all input
    }
}

impl Wizard {
    fn apply_transition(&mut self, transition: StepTransition) -> Command<Message> {
        match transition {
            StepTransition::Stay => Command::none(),
            StepTransition::Next => {
                if self.steps[self.current].is_valid() {
                    if self.is_last_step() {
                        Command::message(Message::Completed)
                    } else {
                        self.current += 1;
                        Command::none()
                    }
                } else {
                    Command::none()
                }
            }
            StepTransition::Back => {
                if self.current > 0 {
                    self.current -= 1;
                }
                Command::none()
            }
            StepTransition::Complete => Command::message(Message::Completed),
            StepTransition::Cancel => Command::message(Message::Cancelled),
        }
    }

    fn render_progress(&self, frame: &mut Frame, area: Rect) {
        if area.width < 10 {
            return;
        }
        let total = self.steps.len();
        let current = self.current + 1;

        // Progress bar: [████░░░░] Step 2/5
        let bar_width = (area.width as usize).saturating_sub(12).min(20);
        let filled = if total > 0 {
            (current * bar_width) / total
        } else {
            0
        };
        let empty = bar_width.saturating_sub(filled);

        let spans = vec![
            Span::raw("["),
            Span::styled("\u{2588}".repeat(filled), self.style.progress_filled),
            Span::styled("\u{2591}".repeat(empty), self.style.progress_empty),
            Span::raw("] "),
            Span::styled(
                format!("Step {}/{}", current, total),
                self.style.step_counter,
            ),
        ];

        let paragraph = Paragraph::new(Line::from(spans));
        frame.render_widget(paragraph, area);
    }

    fn render_nav_hints(&self, frame: &mut Frame, area: Rect) {
        let mut hints = Vec::new();

        if !self.is_first_step() {
            hints.push("Esc: Back");
        }

        if self.steps[self.current].is_valid() {
            if self.is_last_step() {
                hints.push("Enter: Finish");
            } else {
                hints.push("Enter: Next");
            }
        }

        hints.push("Ctrl+C: Cancel");

        let text = hints.join("  \u{2502}  ");
        let paragraph = Paragraph::new(Line::from(Span::styled(text, self.style.nav_hint)))
            .alignment(Alignment::Center);
        frame.render_widget(paragraph, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEventKind, KeyEventState};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn ctrl_key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    /// A simple test step that always validates.
    struct TestStep {
        title: String,
        valid: bool,
        next_on_enter: bool,
    }

    impl TestStep {
        fn new(title: &str) -> Self {
            Self {
                title: title.to_string(),
                valid: true,
                next_on_enter: true,
            }
        }

        fn invalid(title: &str) -> Self {
            Self {
                title: title.to_string(),
                valid: false,
                next_on_enter: true,
            }
        }
    }

    impl WizardStep for TestStep {
        fn title(&self) -> &str {
            &self.title
        }

        fn is_valid(&self) -> bool {
            self.valid
        }

        fn handle_key(&mut self, key: KeyEvent) -> StepTransition {
            match key.code {
                KeyCode::Enter if self.next_on_enter => StepTransition::Next,
                KeyCode::Backspace => StepTransition::Back,
                _ => StepTransition::Stay,
            }
        }

        fn render(&self, _frame: &mut Frame, _area: Rect) {
            // No-op for tests
        }
    }

    fn make_wizard(count: usize) -> Wizard {
        let steps: Vec<Box<dyn WizardStep>> = (0..count)
            .map(|i| Box::new(TestStep::new(&format!("Step {}", i + 1))) as Box<dyn WizardStep>)
            .collect();
        Wizard::new(steps)
    }

    #[test]
    fn initial_state() {
        let wiz = make_wizard(3);
        assert_eq!(wiz.current_step(), 0);
        assert_eq!(wiz.step_count(), 3);
        assert!(wiz.is_first_step());
        assert!(!wiz.is_last_step());
        assert_eq!(wiz.current_title(), "Step 1");
    }

    #[test]
    fn navigate_forward() {
        let mut wiz = make_wizard(3);
        wiz.update(Message::KeyPress(key(KeyCode::Enter)));
        assert_eq!(wiz.current_step(), 1);
        wiz.update(Message::KeyPress(key(KeyCode::Enter)));
        assert_eq!(wiz.current_step(), 2);
        assert!(wiz.is_last_step());
    }

    #[test]
    fn navigate_back() {
        let mut wiz = make_wizard(3);
        wiz.update(Message::KeyPress(key(KeyCode::Enter))); // step 1
        wiz.update(Message::KeyPress(key(KeyCode::Enter))); // step 2
        wiz.update(Message::KeyPress(key(KeyCode::Backspace))); // back to step 1
        assert_eq!(wiz.current_step(), 1);
    }

    #[test]
    fn cannot_go_back_from_first() {
        let mut wiz = make_wizard(3);
        wiz.update(Message::KeyPress(key(KeyCode::Backspace)));
        assert_eq!(wiz.current_step(), 0);
    }

    #[test]
    fn complete_on_last_step() {
        let mut wiz = make_wizard(2);
        wiz.update(Message::KeyPress(key(KeyCode::Enter))); // step 1
        let cmd = wiz.update(Message::KeyPress(key(KeyCode::Enter))); // last step → complete
        match cmd.into_message() {
            Some(Message::Completed) => {}
            other => panic!(
                "Expected Completed, got {:?}",
                other.map(|m| format!("{:?}", m))
            ),
        }
    }

    #[test]
    fn cancel_with_ctrl_c() {
        let mut wiz = make_wizard(3);
        let cmd = wiz.update(Message::KeyPress(ctrl_key(KeyCode::Char('c'))));
        match cmd.into_message() {
            Some(Message::Cancelled) => {}
            other => panic!(
                "Expected Cancelled, got {:?}",
                other.map(|m| format!("{:?}", m))
            ),
        }
    }

    #[test]
    fn validation_gates_forward() {
        let steps: Vec<Box<dyn WizardStep>> = vec![
            Box::new(TestStep::invalid("Invalid Step")),
            Box::new(TestStep::new("Step 2")),
        ];
        let mut wiz = Wizard::new(steps);
        wiz.update(Message::KeyPress(key(KeyCode::Enter)));
        assert_eq!(wiz.current_step(), 0); // Didn't advance
    }

    #[test]
    fn tick_delegates_to_step() {
        // This tests the tick path — our TestStep returns None from handle_tick
        let mut wiz = make_wizard(2);
        let cmd = wiz.update(Message::Tick);
        assert!(cmd.is_none());
    }

    #[test]
    fn esc_cancels_on_first_step() {
        let mut wiz = make_wizard(2);
        let cmd = wiz.update(Message::KeyPress(key(KeyCode::Esc)));
        match cmd.into_message() {
            Some(Message::Cancelled) => {}
            other => panic!(
                "Expected Cancelled, got {:?}",
                other.map(|m| format!("{:?}", m))
            ),
        }
    }

    #[test]
    fn esc_goes_back_on_non_first_step() {
        let mut wiz = make_wizard(3);
        wiz.update(Message::KeyPress(key(KeyCode::Enter))); // advance to step 1
        assert_eq!(wiz.current_step(), 1);

        wiz.update(Message::KeyPress(key(KeyCode::Esc))); // should go back
        assert_eq!(wiz.current_step(), 0);
    }
}
