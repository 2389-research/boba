//! # File Browser Example
//!
//! A two-panel file browser demonstrating:
//! - FilePicker component for directory navigation
//! - Viewport component for file preview
//! - Async file loading with `Command::perform`
//! - Focus toggling between panels
//!
//! Run with: `cargo run --example file_browser`

use boba::crossterm::event::{KeyCode, KeyModifiers};
use boba::ratatui::layout::{Constraint, Layout};
use boba::ratatui::style::{Color, Modifier, Style};
use boba::ratatui::text::{Line, Span};
use boba::ratatui::widgets::Paragraph;
use boba::ratatui::Frame;
use boba::widgets::filepicker::{self, FilePicker};
use boba::widgets::viewport::{self, Viewport};
use boba::{terminal_events, Command, Component, Model, Subscription, TerminalEvent};
use std::path::PathBuf;

/// Which panel currently has keyboard focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Focus {
    Picker,
    Preview,
}

/// Top-level application state.
struct FileBrowser {
    picker: FilePicker,
    preview: Viewport,
    focus: Focus,
    status_path: String,
}

/// Application message type.
#[derive(Debug)]
enum Msg {
    Picker(filepicker::Message),
    Preview(viewport::Message),
    FileLoaded(String),
    ToggleFocus,
    Quit,
}

impl Model for FileBrowser {
    type Message = Msg;
    type Flags = ();

    fn init(_: ()) -> (Self, Command<Msg>) {
        let cwd = std::env::current_dir().unwrap_or_default();
        let status_path = cwd.display().to_string();

        let mut picker = FilePicker::new(cwd)
            .with_show_size(true)
            .with_show_permissions(true);
        picker.focus();

        let preview = Viewport::new("Select a file to preview its contents.");

        (
            FileBrowser {
                picker,
                preview,
                focus: Focus::Picker,
                status_path,
            },
            Command::none(),
        )
    }

    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Picker(filepicker::Message::SelectFile(ref path)) => {
                // Async file loading: when a file is selected, kick off an
                // async read via Command::perform. Command::batch runs the
                // picker update and the file load concurrently.
                let path_clone = path.clone();
                self.status_path = path.display().to_string();
                let cmd_picker = self
                    .picker
                    .update(filepicker::Message::SelectFile(path.clone()))
                    .map(Msg::Picker);
                let cmd_load = Command::perform(
                    async move { load_file_contents(&path_clone).await },
                    Msg::FileLoaded,
                );
                Command::batch([cmd_picker, cmd_load])
            }
            Msg::Picker(filepicker::Message::EnterDir(ref path)) => {
                self.status_path = path.display().to_string();
                self.preview
                    .set_content("Select a file to preview its contents.");
                self.picker
                    .update(filepicker::Message::EnterDir(path.clone()))
                    .map(Msg::Picker)
            }
            Msg::Picker(filepicker::Message::GoUp) => {
                self.status_path = self.picker.current_dir().display().to_string();
                self.preview
                    .set_content("Select a file to preview its contents.");
                self.picker
                    .update(filepicker::Message::GoUp)
                    .map(Msg::Picker)
            }
            Msg::Picker(m) => self.picker.update(m).map(Msg::Picker),
            Msg::Preview(m) => self.preview.update(m).map(Msg::Preview),
            Msg::FileLoaded(content) => {
                self.preview.set_content(content);
                Command::none()
            }
            // Focus management: toggle which panel owns keyboard input and
            // update the focus/blur state on both child components.
            Msg::ToggleFocus => {
                match self.focus {
                    Focus::Picker => {
                        self.focus = Focus::Preview;
                        self.picker.blur();
                        self.preview.focus();
                    }
                    Focus::Preview => {
                        self.focus = Focus::Picker;
                        self.picker.focus();
                        self.preview.blur();
                    }
                }
                Command::none()
            }
            Msg::Quit => Command::quit(),
        }
    }

    fn view(&self, frame: &mut Frame) {
        let area = frame.area();

        let [title_area, main_area, status_area] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(area);

        // Title bar
        let title = Paragraph::new(Line::from(vec![Span::styled(
            " File Browser ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]))
        .style(Style::default().bg(Color::Cyan).fg(Color::Black));
        frame.render_widget(title, title_area);

        // Main two-panel split: picker (60%) | preview (40%)
        let [picker_area, preview_area] =
            Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)])
                .areas(main_area);

        self.picker.view(frame, picker_area);
        self.preview.view(frame, preview_area);

        // Status bar
        let focus_label = match self.focus {
            Focus::Picker => "picker",
            Focus::Preview => "preview",
        };
        let status_line = Line::from(vec![
            Span::styled(
                format!(" {} ", self.status_path),
                Style::default().fg(Color::White),
            ),
            Span::styled(
                format!(" [{}] ", focus_label),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " Tab:switch  j/k:navigate  Enter:select  Backspace:up  .:hidden  q:quit ",
                Style::default().fg(Color::DarkGray),
            ),
        ]);
        let status = Paragraph::new(status_line);
        frame.render_widget(status, status_area);
    }

    fn subscriptions(&self) -> Vec<Subscription<Msg>> {
        let focus = self.focus;
        vec![terminal_events(move |ev| match ev {
            TerminalEvent::Key(key) => match (key.code, key.modifiers) {
                (KeyCode::Char('q'), KeyModifiers::NONE) => Some(Msg::Quit),
                (KeyCode::Char('c'), m) if m.contains(KeyModifiers::CONTROL) => Some(Msg::Quit),
                (KeyCode::Esc, _) => Some(Msg::Quit),
                (KeyCode::Tab, _) => Some(Msg::ToggleFocus),
                _ => match focus {
                    Focus::Picker => Some(Msg::Picker(filepicker::Message::KeyPress(key))),
                    Focus::Preview => Some(Msg::Preview(viewport::Message::KeyPress(key))),
                },
            },
            _ => None,
        })]
    }
}

/// Maximum file size to preview (10 MB).
const MAX_PREVIEW_SIZE: u64 = 10 * 1024 * 1024;

/// Read file contents asynchronously. Returns an error message string on failure.
async fn load_file_contents(path: &PathBuf) -> String {
    // Check file size before reading to avoid memory exhaustion.
    match tokio::fs::metadata(path).await {
        Ok(meta) if meta.len() > MAX_PREVIEW_SIZE => {
            return format!(
                "(file too large to preview: {} bytes)\n\nMaximum preview size is {} bytes.",
                meta.len(),
                MAX_PREVIEW_SIZE
            );
        }
        Err(e) => return format!("Error reading file: {}", e),
        _ => {}
    }

    match tokio::fs::read(path).await {
        Ok(bytes) => {
            // Binary file detection: sample the first 8KB for null bytes.
            // If a null byte is found, treat the file as binary and show
            // a placeholder instead of garbled content.
            let sample = &bytes[..bytes.len().min(8192)];
            if sample.contains(&0) {
                format!(
                    "(binary file, {} bytes)\n\nBinary files cannot be previewed.",
                    bytes.len()
                )
            } else {
                String::from_utf8_lossy(&bytes).into_owned()
            }
        }
        Err(e) => format!("Error reading file: {}", e),
    }
}

#[boba::tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    boba::run::<FileBrowser>(()).await?;
    Ok(())
}
