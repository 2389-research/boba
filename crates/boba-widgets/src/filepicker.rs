//! File system browser component with directory navigation, hidden file
//! toggling, extension filtering, and async directory loading.

use boba_core::command::Command;
use boba_core::component::Component;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;
use std::path::{Path, PathBuf};

/// Messages for the file picker component.
#[derive(Debug, Clone)]
pub enum Message {
    /// A key press event forwarded to the file picker.
    KeyPress(KeyEvent),
    /// A file was selected (Enter on a file).
    SelectFile(PathBuf),
    /// A directory was entered (Enter on a directory).
    EnterDir(PathBuf),
    /// Navigate to the parent directory.
    GoUp,
    /// Refresh the current directory listing.
    Refresh,
    /// Directory contents have been loaded.
    FilesLoaded(Vec<FileEntry>),
}

/// A single entry in the file picker listing.
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// Full path to the file or directory.
    pub path: PathBuf,
    /// Display name (file or directory name without parent path).
    pub name: String,
    /// Whether this entry is a directory.
    pub is_dir: bool,
    /// File size in bytes (0 for directories).
    pub size: u64,
    /// Unix-style permission string (e.g. "rwxr-xr-x").
    pub permissions: String,
}

/// Style configuration for the file picker.
#[derive(Debug, Clone)]
pub struct FilePickerStyle {
    /// Style applied to directory names.
    pub directory: Style,
    /// Style applied to regular file names.
    pub file: Style,
    /// Style applied to the currently highlighted entry.
    pub selected: Style,
}

impl Default for FilePickerStyle {
    fn default() -> Self {
        Self {
            directory: Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
            file: Style::default(),
            selected: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED),
        }
    }
}

/// A file system browser component.
///
/// Displays the contents of a directory and allows navigation with keyboard.
/// Supports showing/hiding hidden files, filtering by extension, and
/// displaying file sizes and permissions.
pub struct FilePicker {
    current_dir: PathBuf,
    entries: Vec<FileEntry>,
    cursor: usize,
    show_hidden: bool,
    allowed_extensions: Vec<String>,
    show_permissions: bool,
    show_size: bool,
    height: u16,
    focus: bool,
    style: FilePickerStyle,
    block: Option<Block<'static>>,
}

impl FilePicker {
    /// Create a new file picker starting at the given directory.
    pub fn new(dir: PathBuf) -> Self {
        let entries = read_directory(&dir, false, &[]);
        Self {
            current_dir: dir,
            entries,
            cursor: 0,
            show_hidden: false,
            allowed_extensions: Vec::new(),
            show_permissions: false,
            show_size: false,
            height: 10,
            focus: false,
            style: FilePickerStyle::default(),
            block: None,
        }
    }

    /// Set whether hidden files (starting with '.') are shown.
    pub fn with_show_hidden(mut self, show: bool) -> Self {
        self.show_hidden = show;
        self
    }

    /// Set allowed file extensions to filter by (e.g., `vec!["rs", "toml"]`).
    /// An empty list means all files are shown.
    pub fn with_extensions(mut self, exts: Vec<String>) -> Self {
        self.allowed_extensions = exts;
        self
    }

    /// Set whether file permissions are displayed.
    pub fn with_show_permissions(mut self, show: bool) -> Self {
        self.show_permissions = show;
        self
    }

    /// Set whether file sizes are displayed.
    pub fn with_show_size(mut self, show: bool) -> Self {
        self.show_size = show;
        self
    }

    /// Set the visible height of the file list (in rows).
    pub fn with_height(mut self, h: u16) -> Self {
        self.height = h;
        self
    }

    /// Set the file picker style.
    pub fn with_style(mut self, style: FilePickerStyle) -> Self {
        self.style = style;
        self
    }

    /// Set the block (border/title container) for the file picker.
    pub fn with_block(mut self, block: Block<'static>) -> Self {
        self.block = Some(block);
        self
    }

    /// Give focus to the file picker.
    pub fn focus(&mut self) {
        self.focus = true;
    }

    /// Remove focus from the file picker.
    pub fn blur(&mut self) {
        self.focus = false;
    }

    /// Get the current directory.
    pub fn current_dir(&self) -> &Path {
        &self.current_dir
    }

    /// Get the currently highlighted entry, if any.
    pub fn selected_entry(&self) -> Option<&FileEntry> {
        self.entries.get(self.cursor)
    }

    /// Build a command to load files asynchronously.
    ///
    /// Directory reading via `std::fs::read_dir` is fast enough to perform
    /// inline in an async block without needing `spawn_blocking`.
    fn load_files_command(
        dir: PathBuf,
        show_hidden: bool,
        extensions: Vec<String>,
    ) -> Command<Message> {
        Command::perform(
            async move { read_directory(&dir, show_hidden, &extensions) },
            Message::FilesLoaded,
        )
    }
}

/// Read directory contents synchronously, returning sorted file entries.
/// Directories come first, then files. Both groups are sorted alphabetically.
fn read_directory(dir: &Path, show_hidden: bool, allowed_extensions: &[String]) -> Vec<FileEntry> {
    let read = match std::fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(_) => return Vec::new(),
    };

    let mut entries: Vec<FileEntry> = read
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            // Filter hidden files
            if !show_hidden && name.starts_with('.') {
                return None;
            }

            let metadata = entry.metadata().ok()?;
            let is_dir = metadata.is_dir();
            let size = if is_dir { 0 } else { metadata.len() };

            // Filter by extension (only applies to files)
            if !is_dir && !allowed_extensions.is_empty() {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                if !allowed_extensions.iter().any(|a| a == ext) {
                    return None;
                }
            }

            // Build permissions string (Unix-style approximation)
            let permissions = format_permissions(&metadata);

            Some(FileEntry {
                path,
                name,
                is_dir,
                size,
                permissions,
            })
        })
        .collect();

    // Sort: directories first, then files; alphabetical within each group
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    entries
}

/// Format file permissions as a simple string.
fn format_permissions(metadata: &std::fs::Metadata) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = metadata.permissions().mode();
        let flags = [
            if mode & 0o400 != 0 { 'r' } else { '-' },
            if mode & 0o200 != 0 { 'w' } else { '-' },
            if mode & 0o100 != 0 { 'x' } else { '-' },
            if mode & 0o040 != 0 { 'r' } else { '-' },
            if mode & 0o020 != 0 { 'w' } else { '-' },
            if mode & 0o010 != 0 { 'x' } else { '-' },
            if mode & 0o004 != 0 { 'r' } else { '-' },
            if mode & 0o002 != 0 { 'w' } else { '-' },
            if mode & 0o001 != 0 { 'x' } else { '-' },
        ];
        flags.iter().collect()
    }
    #[cfg(not(unix))]
    {
        if metadata.permissions().readonly() {
            "r--".to_string()
        } else {
            "rw-".to_string()
        }
    }
}

/// Format a file size in human-readable form.
fn format_size(size: u64) -> String {
    if size < 1024 {
        format!("{}B", size)
    } else if size < 1024 * 1024 {
        format!("{:.1}K", size as f64 / 1024.0)
    } else if size < 1024 * 1024 * 1024 {
        format!("{:.1}M", size as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1}G", size as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

impl Component for FilePicker {
    type Message = Message;

    fn update(&mut self, msg: Message) -> Command<Message> {
        match msg {
            Message::KeyPress(key) if self.focus => match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.cursor > 0 {
                        self.cursor -= 1;
                    }
                    Command::none()
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.cursor + 1 < self.entries.len() {
                        self.cursor += 1;
                    }
                    Command::none()
                }
                KeyCode::Enter => {
                    if let Some(entry) = self.entries.get(self.cursor).cloned() {
                        if entry.is_dir {
                            self.current_dir = entry.path.clone();
                            self.cursor = 0;
                            let dir = self.current_dir.clone();
                            let show_hidden = self.show_hidden;
                            let exts = self.allowed_extensions.clone();
                            return Command::batch([
                                Command::message(Message::EnterDir(entry.path)),
                                FilePicker::load_files_command(dir, show_hidden, exts),
                            ]);
                        } else {
                            return Command::message(Message::SelectFile(entry.path));
                        }
                    }
                    Command::none()
                }
                KeyCode::Backspace => {
                    if let Some(parent) = self.current_dir.parent().map(|p| p.to_path_buf()) {
                        self.current_dir = parent;
                        self.cursor = 0;
                        let dir = self.current_dir.clone();
                        let show_hidden = self.show_hidden;
                        let exts = self.allowed_extensions.clone();
                        return Command::batch([
                            Command::message(Message::GoUp),
                            FilePicker::load_files_command(dir, show_hidden, exts),
                        ]);
                    }
                    Command::none()
                }
                KeyCode::Char('.') => {
                    self.show_hidden = !self.show_hidden;
                    self.cursor = 0;
                    let dir = self.current_dir.clone();
                    let show_hidden = self.show_hidden;
                    let exts = self.allowed_extensions.clone();
                    Command::batch([
                        Command::message(Message::Refresh),
                        FilePicker::load_files_command(dir, show_hidden, exts),
                    ])
                }
                KeyCode::Char('r') => {
                    self.cursor = 0;
                    let dir = self.current_dir.clone();
                    let show_hidden = self.show_hidden;
                    let exts = self.allowed_extensions.clone();
                    Command::batch([
                        Command::message(Message::Refresh),
                        FilePicker::load_files_command(dir, show_hidden, exts),
                    ])
                }
                _ => Command::none(),
            },
            Message::FilesLoaded(entries) => {
                self.entries = entries;
                if self.cursor >= self.entries.len() {
                    self.cursor = self.entries.len().saturating_sub(1);
                }
                Command::none()
            }
            // EnterDir and GoUp are notification messages emitted by the key handlers.
            // The key handlers already update state and issue load commands, so these
            // are no-ops internally. Parent components can match on them for side effects.
            Message::EnterDir(_) | Message::GoUp => Command::none(),
            Message::Refresh => {
                let dir = self.current_dir.clone();
                let show_hidden = self.show_hidden;
                let exts = self.allowed_extensions.clone();
                FilePicker::load_files_command(dir, show_hidden, exts)
            }
            _ => Command::none(),
        }
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let inner = if let Some(ref block) = self.block {
            let inner = block.inner(area);
            frame.render_widget(block.clone(), area);
            inner
        } else {
            area
        };

        if inner.height == 0 || inner.width == 0 {
            return;
        }

        // Determine visible range (scrolling)
        let visible_height = inner.height as usize;
        let scroll_offset = if self.cursor >= visible_height {
            self.cursor - visible_height + 1
        } else {
            0
        };

        let visible_entries = self
            .entries
            .iter()
            .enumerate()
            .skip(scroll_offset)
            .take(visible_height);

        let mut lines: Vec<Line> = Vec::new();
        for (i, entry) in visible_entries {
            let is_selected = i == self.cursor;

            // Icon
            let icon = if entry.is_dir {
                "\u{1F4C1} "
            } else {
                "\u{1F4C4} "
            };

            let mut spans = Vec::new();
            spans.push(Span::raw(icon));

            // Name
            let name_style = if is_selected {
                self.style.selected
            } else if entry.is_dir {
                self.style.directory
            } else {
                self.style.file
            };
            spans.push(Span::styled(&entry.name, name_style));

            // Optional size (files only)
            if self.show_size && !entry.is_dir {
                spans.push(Span::raw("  "));
                spans.push(Span::styled(
                    format_size(entry.size),
                    Style::default().fg(Color::DarkGray),
                ));
            }

            // Optional permissions
            if self.show_permissions {
                spans.push(Span::raw("  "));
                spans.push(Span::styled(
                    &entry.permissions,
                    Style::default().fg(Color::DarkGray),
                ));
            }

            lines.push(Line::from(spans));
        }

        if lines.is_empty() {
            lines.push(Line::from(Span::styled(
                "(empty)",
                Style::default().fg(Color::DarkGray),
            )));
        }

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, inner);
    }

    fn focused(&self) -> bool {
        self.focus
    }
}
