//! Example: Composing a multi-step wizard from Progress + state machine.
//!
//! Demonstrates how to build a wizard UI without a dedicated Wizard widget,
//! using Progress for the step indicator and enum-based step management.
//!
//! Run with: `cargo run --example wizard`

use boba::crossterm::event::{KeyCode, KeyModifiers};
use boba::ratatui::layout::{Alignment, Constraint, Layout, Rect};
use boba::ratatui::style::{Color, Modifier, Style};
use boba::ratatui::text::{Line, Span};
use boba::ratatui::widgets::{Clear, Paragraph};
use boba::ratatui::Frame;
use boba::widgets::chrome::focus_block;
use boba::widgets::progress::{self, Progress};
use boba::widgets::text_input::{self, TextInput};
use boba::{terminal_events, Command, Component, Model, Subscription, TerminalEvent};

// ---------------------------------------------------------------------------
// Step enum — each variant holds its own state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
enum Step {
    Welcome,
    Name,
    Confirm,
    Done,
    Cancelled,
}

const TOTAL_STEPS: usize = 3; // Welcome, Name, Confirm

impl Step {
    fn index(&self) -> usize {
        match self {
            Step::Welcome => 0,
            Step::Name => 1,
            Step::Confirm => 2,
            Step::Done | Step::Cancelled => TOTAL_STEPS,
        }
    }
}

// ---------------------------------------------------------------------------
// App model
// ---------------------------------------------------------------------------

struct WizardApp {
    step: Step,
    name_input: TextInput,
    progress: Progress,
}

#[derive(Debug)]
enum Msg {
    NextStep,
    PrevStep,
    Cancel,
    Quit,
    NameInput(text_input::Message),
    Progress(progress::Message),
}

impl WizardApp {
    /// Whether the current step's data is valid (gates forward navigation).
    fn current_step_valid(&self) -> bool {
        match self.step {
            Step::Welcome => true,
            Step::Name => !self.name_input.value().trim().is_empty(),
            Step::Confirm => true,
            Step::Done | Step::Cancelled => false,
        }
    }

    /// Advance to the next step if valid.
    fn go_next(&mut self) -> Command<Msg> {
        if !self.current_step_valid() {
            return Command::none();
        }
        match self.step {
            Step::Welcome => {
                self.step = Step::Name;
                self.name_input.focus();
            }
            Step::Name => {
                self.step = Step::Confirm;
            }
            Step::Confirm => {
                self.step = Step::Done;
            }
            _ => {}
        }
        self.sync_progress();
        Command::none()
    }

    /// Go back one step.
    fn go_back(&mut self) -> Command<Msg> {
        match self.step {
            Step::Name => {
                self.step = Step::Welcome;
            }
            Step::Confirm => {
                self.step = Step::Name;
                self.name_input.focus();
            }
            _ => {}
        }
        self.sync_progress();
        Command::none()
    }

    fn sync_progress(&mut self) {
        let ratio = (self.step.index() as f64) / (TOTAL_STEPS as f64);
        self.progress.set_progress(ratio);
    }

    /// Render a centered rectangle within `area`.
    fn centered_rect(area: Rect, width_pct: u16, height_pct: u16) -> Rect {
        let [_, v_center, _] = Layout::vertical([
            Constraint::Percentage((100 - height_pct) / 2),
            Constraint::Percentage(height_pct),
            Constraint::Percentage((100 - height_pct) / 2),
        ])
        .areas(area);
        let [_, h_center, _] = Layout::horizontal([
            Constraint::Percentage((100 - width_pct) / 2),
            Constraint::Percentage(width_pct),
            Constraint::Percentage((100 - width_pct) / 2),
        ])
        .areas(v_center);
        h_center
    }
}

// ---------------------------------------------------------------------------
// Model implementation
// ---------------------------------------------------------------------------

impl Model for WizardApp {
    type Message = Msg;
    type Flags = ();

    fn init(_: ()) -> (Self, Command<Msg>) {
        let progress = Progress::new("wizard-progress")
            .with_label("Progress")
            .with_fill_color(Color::Cyan);
        let name_input = TextInput::new("Enter your name...");

        (
            WizardApp {
                step: Step::Welcome,
                name_input,
                progress,
            },
            Command::none(),
        )
    }

    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::NextStep => self.go_next(),
            Msg::PrevStep => self.go_back(),
            Msg::Cancel => {
                self.step = Step::Cancelled;
                Command::none()
            }
            Msg::Quit => Command::quit(),
            Msg::NameInput(m) => self.name_input.update(m).map(Msg::NameInput),
            Msg::Progress(m) => {
                self.progress.update(m).map(Msg::Progress);
                Command::none()
            }
        }
    }

    fn view(&self, frame: &mut Frame) {
        let area = frame.area();

        // Terminal-exit screens
        match self.step {
            Step::Done => {
                let text = Paragraph::new(vec![
                    Line::from(Span::styled(
                        "Wizard Complete!",
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                    Line::from(format!("Hello, {}!", self.name_input.value())),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Press q to exit.",
                        Style::default().fg(Color::DarkGray),
                    )),
                ])
                .alignment(Alignment::Center);
                let centered = Self::centered_rect(area, 50, 30);
                frame.render_widget(text, centered);
                return;
            }
            Step::Cancelled => {
                let text = Paragraph::new(vec![
                    Line::from(Span::styled(
                        "Wizard Cancelled",
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Press q to exit.",
                        Style::default().fg(Color::DarkGray),
                    )),
                ])
                .alignment(Alignment::Center);
                let centered = Self::centered_rect(area, 50, 30);
                frame.render_widget(text, centered);
                return;
            }
            _ => {}
        }

        // Main wizard layout — centered card
        let card = Self::centered_rect(area, 60, 60);
        frame.render_widget(Clear, card);

        let block = focus_block("Setup Wizard", true);
        let inner = block.inner(card);
        frame.render_widget(block, card);

        let [progress_area, _gap1, title_area, content_area, _gap2, hint_area] =
            Layout::vertical([
                Constraint::Length(1), // Progress bar
                Constraint::Length(1), // Gap
                Constraint::Length(2), // Step title
                Constraint::Fill(1),   // Step content
                Constraint::Length(1), // Gap
                Constraint::Length(1), // Nav hints
            ])
            .areas(inner);

        // -- Progress bar --
        self.progress.view(frame, progress_area);

        // -- Step title --
        let step_title = match self.step {
            Step::Welcome => "Welcome",
            Step::Name => "Your Name",
            Step::Confirm => "Confirm",
            Step::Done | Step::Cancelled => "",
        };
        let title = Paragraph::new(Line::from(vec![
            Span::styled(
                step_title,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                format!("Step {}/{}", self.step.index() + 1, TOTAL_STEPS),
                Style::default().fg(Color::DarkGray),
            ),
        ]));
        frame.render_widget(title, title_area);

        // -- Step content --
        match self.step {
            Step::Welcome => {
                let welcome = Paragraph::new(vec![
                    Line::from("Welcome to the setup wizard!"),
                    Line::from(""),
                    Line::from("This example shows how to compose a multi-step"),
                    Line::from("wizard from Progress + TextInput + state machine."),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Press Enter to begin.",
                        Style::default().fg(Color::Cyan),
                    )),
                ]);
                frame.render_widget(welcome, content_area);
            }
            Step::Name => {
                let [prompt_area, _g, input_area, _rest] = Layout::vertical([
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(3),
                    Constraint::Fill(1),
                ])
                .areas(content_area);

                let prompt = Paragraph::new("Please enter your name:");
                frame.render_widget(prompt, prompt_area);

                let input_block = focus_block("Name", self.name_input.focused());
                let input_inner = input_block.inner(input_area);
                frame.render_widget(input_block, input_area);
                self.name_input.view(frame, input_inner);

                if self.name_input.value().trim().is_empty() {
                    let [_, warn_area] =
                        Layout::vertical([Constraint::Length(5), Constraint::Length(1)])
                            .areas(content_area);
                    let warn = Paragraph::new(Span::styled(
                        "Name is required to continue.",
                        Style::default().fg(Color::Yellow),
                    ));
                    frame.render_widget(warn, warn_area);
                }
            }
            Step::Confirm => {
                let confirm = Paragraph::new(vec![
                    Line::from(vec![
                        Span::raw("Name: "),
                        Span::styled(
                            self.name_input.value(),
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]),
                    Line::from(""),
                    Line::from("Press Enter to finish, or Esc to go back."),
                ]);
                frame.render_widget(confirm, content_area);
            }
            Step::Done | Step::Cancelled => {}
        }

        // -- Navigation hints --
        let mut hints = Vec::new();
        if self.step != Step::Welcome {
            hints.push(Span::styled("Esc", Style::default().fg(Color::DarkGray)));
            hints.push(Span::raw(": Back  "));
        }
        if self.current_step_valid() {
            hints.push(Span::styled("Enter", Style::default().fg(Color::DarkGray)));
            if self.step == Step::Confirm {
                hints.push(Span::raw(": Finish  "));
            } else {
                hints.push(Span::raw(": Next  "));
            }
        }
        hints.push(Span::styled("Ctrl+C", Style::default().fg(Color::DarkGray)));
        hints.push(Span::raw(": Cancel"));

        let nav = Paragraph::new(Line::from(hints)).alignment(Alignment::Center);
        frame.render_widget(nav, hint_area);
    }

    fn subscriptions(&self) -> Vec<Subscription<Msg>> {
        let step = self.step.clone();
        let valid = self.current_step_valid();

        let mut subs: Vec<Subscription<Msg>> = vec![terminal_events(move |ev| match ev {
            TerminalEvent::Key(key) => match (key.code, key.modifiers) {
                // Quit on Ctrl+C in terminal states, cancel otherwise
                (KeyCode::Char('c'), m) if m.contains(KeyModifiers::CONTROL) => {
                    if step == Step::Done || step == Step::Cancelled {
                        Some(Msg::Quit)
                    } else {
                        Some(Msg::Cancel)
                    }
                }
                // q to quit on terminal screens
                (KeyCode::Char('q'), _) if step == Step::Done || step == Step::Cancelled => {
                    Some(Msg::Quit)
                }
                // Enter to advance (only if valid)
                (KeyCode::Enter, _) if valid && step != Step::Done && step != Step::Cancelled => {
                    Some(Msg::NextStep)
                }
                // Esc to go back
                (KeyCode::Esc, _)
                    if step != Step::Welcome && step != Step::Done && step != Step::Cancelled =>
                {
                    Some(Msg::PrevStep)
                }
                // Esc on welcome = cancel
                (KeyCode::Esc, _) if step == Step::Welcome => Some(Msg::Cancel),
                // Forward everything else to TextInput on the Name step
                _ if step == Step::Name => Some(Msg::NameInput(text_input::Message::KeyPress(key))),
                _ => None,
            },
            _ => None,
        })];

        // Progress bar animation subscription
        for sub in self.progress.subscriptions() {
            subs.push(sub.map(Msg::Progress));
        }

        subs
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[boba::tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    boba::run::<WizardApp>(()).await?;
    Ok(())
}
