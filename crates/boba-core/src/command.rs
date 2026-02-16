use futures::future::BoxFuture;
use futures::stream::BoxStream;
use std::future::Future;
use std::path::PathBuf;

/// A side effect returned from [`Model::update`](crate::Model::update) or [`Model::init`](crate::Model::init).
///
/// Commands represent async operations, immediate messages, terminal management,
/// and program lifecycle actions. They are the primary way to perform work that
/// goes beyond pure state updates.
///
/// # Examples
///
/// ```rust,ignore
/// // Do nothing:
/// let cmd = Command::none();
///
/// // Run an async task and map the result to a message:
/// let cmd = Command::perform(
///     async { fetch_data().await },
///     |data| Msg::DataLoaded(data),
/// );
///
/// // Quit the program:
/// let cmd = Command::quit();
/// ```
pub struct Command<Msg: Send + 'static> {
    pub(crate) inner: CommandInner<Msg>,
}

#[allow(dead_code)]
pub(crate) enum CommandInner<Msg: Send + 'static> {
    None,
    Action(Action<Msg>),
    Future(BoxFuture<'static, Msg>),
    Stream(BoxStream<'static, Msg>),
    Batch(Vec<Command<Msg>>),
    Sequence(Vec<Command<Msg>>),
    Terminal(TerminalCommand),
    /// Execute an external process, releasing terminal control.
    Exec {
        cmd: ExecCommand,
        on_exit: Box<dyn FnOnce(std::io::Result<std::process::ExitStatus>) -> Msg + Send>,
    },
}

/// Internal action variants handled synchronously by the runtime.
///
/// These are side-effect-free actions that the runtime processes immediately,
/// without spawning async tasks.
pub enum Action<Msg> {
    /// Send a message immediately (no async).
    Message(Msg),
    /// Quit the program.
    Quit,
}

/// Terminal management commands executed by the runtime.
///
/// Sent via [`Command::terminal`] or convenience methods such as
/// [`Command::enter_alt_screen`] and [`Command::hide_cursor`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalCommand {
    /// Switch to the alternate screen buffer.
    EnterAltScreen,
    /// Return to the primary screen buffer.
    ExitAltScreen,
    /// Enable mouse event capture with the specified mode.
    EnableMouseCapture(MouseMode),
    /// Disable mouse event capture.
    DisableMouse,
    /// Make the terminal cursor visible.
    ShowCursor,
    /// Hide the terminal cursor.
    HideCursor,
    /// Change the terminal cursor shape.
    SetCursorStyle(CursorStyle),
    /// Enable bracketed paste mode.
    EnableBracketedPaste,
    /// Disable bracketed paste mode.
    DisableBracketedPaste,
    /// Enable focus-in/focus-out event reporting.
    EnableFocusReporting,
    /// Disable focus-in/focus-out event reporting.
    DisableFocusReporting,
    /// Set the terminal window title.
    SetTitle(String),
    /// Clear the entire terminal screen.
    ClearScreen,
    /// Scroll the terminal viewport up by the given number of lines.
    ScrollUp(u16),
    /// Scroll the terminal viewport down by the given number of lines.
    ScrollDown(u16),
    /// Print a line above the TUI (for inline mode).
    Println(String),
    /// Print formatted text above the TUI (for inline mode).
    Printf(String),
    /// Suspend the process (send SIGTSTP on Unix).
    Suspend,
}

/// Configuration for executing an external process via [`Command::exec`].
///
/// Build an `ExecCommand` using the builder pattern: call [`ExecCommand::new`],
/// then chain [`arg`](ExecCommand::arg), [`args`](ExecCommand::args), and
/// [`working_dir`](ExecCommand::working_dir) as needed.
#[derive(Debug)]
pub struct ExecCommand {
    /// The command to execute.
    pub program: String,
    /// Arguments to the command.
    pub args: Vec<String>,
    /// Working directory (None = inherit).
    pub working_dir: Option<PathBuf>,
}

impl ExecCommand {
    /// Create a new `ExecCommand` for the given program name or path.
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            working_dir: None,
        }
    }

    /// Append a single argument to the command.
    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Append multiple arguments to the command.
    pub fn args(mut self, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    /// Set the working directory for the child process.
    pub fn working_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }
}

/// Mouse capture modes for the terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseMode {
    /// Click, release, wheel, drag.
    CellMotion,
    /// All of above + hover.
    AllMotion,
}

/// Terminal cursor shape styles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorStyle {
    /// The user's default cursor shape as configured in the terminal.
    DefaultUserShape,
    /// A blinking block cursor.
    BlinkingBlock,
    /// A non-blinking (steady) block cursor.
    SteadyBlock,
    /// A blinking underscore cursor.
    BlinkingUnderScore,
    /// A non-blinking (steady) underscore cursor.
    SteadyUnderScore,
    /// A blinking vertical bar cursor.
    BlinkingBar,
    /// A non-blinking (steady) vertical bar cursor.
    SteadyBar,
}

impl<Msg: Send + 'static> Command<Msg> {
    /// No-op command.
    pub fn none() -> Self {
        Command {
            inner: CommandInner::None,
        }
    }

    /// Run an async future, map the result to a message.
    pub fn perform<F, T>(future: F, map: impl FnOnce(T) -> Msg + Send + 'static) -> Self
    where
        F: Future<Output = T> + Send + 'static,
    {
        Command {
            inner: CommandInner::Future(Box::pin(async move { map(future.await) })),
        }
    }

    /// Send a message immediately.
    pub fn message(msg: Msg) -> Self {
        Command {
            inner: CommandInner::Action(Action::Message(msg)),
        }
    }

    /// Quit the program.
    pub fn quit() -> Self {
        Command {
            inner: CommandInner::Action(Action::Quit),
        }
    }

    /// Run multiple commands concurrently.
    pub fn batch(cmds: impl IntoIterator<Item = Command<Msg>>) -> Self {
        let cmds: Vec<_> = cmds.into_iter().collect();
        if cmds.is_empty() {
            return Command::none();
        }
        if cmds.len() == 1 {
            let mut cmds = cmds;
            return cmds.pop().unwrap();
        }
        Command {
            inner: CommandInner::Batch(cmds),
        }
    }

    /// Run commands sequentially — each command's messages are delivered
    /// before the next command starts.
    pub fn sequence(cmds: impl IntoIterator<Item = Command<Msg>>) -> Self {
        let cmds: Vec<_> = cmds.into_iter().collect();
        if cmds.is_empty() {
            return Command::none();
        }
        if cmds.len() == 1 {
            let mut cmds = cmds;
            return cmds.pop().unwrap();
        }
        Command {
            inner: CommandInner::Sequence(cmds),
        }
    }

    /// Terminal management command.
    pub fn terminal(cmd: TerminalCommand) -> Self {
        Command {
            inner: CommandInner::Terminal(cmd),
        }
    }

    /// Transform the message type (for component composition).
    pub fn map<NewMsg: Send + 'static>(
        self,
        f: impl Fn(Msg) -> NewMsg + Send + Sync + 'static,
    ) -> Command<NewMsg> {
        self.map_with(std::sync::Arc::new(f))
    }

    fn map_with<NewMsg: Send + 'static>(
        self,
        f: std::sync::Arc<dyn Fn(Msg) -> NewMsg + Send + Sync>,
    ) -> Command<NewMsg> {
        match self.inner {
            CommandInner::None => Command::none(),
            CommandInner::Action(Action::Message(msg)) => Command::message(f(msg)),
            CommandInner::Action(Action::Quit) => Command::quit(),
            CommandInner::Future(fut) => {
                let f = f.clone();
                Command {
                    inner: CommandInner::Future(Box::pin(async move { f(fut.await) })),
                }
            }
            CommandInner::Stream(stream) => {
                use futures::StreamExt;
                let f = f.clone();
                Command {
                    inner: CommandInner::Stream(Box::pin(stream.map(move |msg| f(msg)))),
                }
            }
            CommandInner::Batch(cmds) => Command {
                inner: CommandInner::Batch(
                    cmds.into_iter()
                        .map(|cmd| cmd.map_with(f.clone()))
                        .collect(),
                ),
            },
            CommandInner::Sequence(cmds) => Command {
                inner: CommandInner::Sequence(
                    cmds.into_iter()
                        .map(|cmd| cmd.map_with(f.clone()))
                        .collect(),
                ),
            },
            CommandInner::Terminal(tcmd) => Command::terminal(tcmd),
            CommandInner::Exec { cmd, on_exit } => {
                let f = f.clone();
                Command {
                    inner: CommandInner::Exec {
                        cmd,
                        on_exit: Box::new(move |result| f(on_exit(result))),
                    },
                }
            }
        }
    }

    /// Execute an external process (e.g., `$EDITOR`), releasing terminal control.
    /// The runtime restores the terminal before running and re-initializes after.
    /// The callback receives the process exit status.
    pub fn exec(
        cmd: ExecCommand,
        on_exit: impl FnOnce(std::io::Result<std::process::ExitStatus>) -> Msg + Send + 'static,
    ) -> Self {
        Command {
            inner: CommandInner::Exec {
                cmd,
                on_exit: Box::new(on_exit),
            },
        }
    }

    /// One-shot timer: fires once after `duration`, mapping the instant to a message.
    pub fn tick(
        duration: std::time::Duration,
        map: impl FnOnce(std::time::Instant) -> Msg + Send + 'static,
    ) -> Self {
        Command {
            inner: CommandInner::Future(Box::pin(async move {
                tokio::time::sleep(duration).await;
                map(std::time::Instant::now())
            })),
        }
    }

    /// Request the current window size. The callback receives (columns, rows).
    pub fn window_size(map: impl FnOnce(u16, u16) -> Msg + Send + 'static) -> Self {
        Command {
            inner: CommandInner::Future(Box::pin(async move {
                let size = crossterm::terminal::size().unwrap_or((80, 24));
                map(size.0, size.1)
            })),
        }
    }

    /// Print a line above the TUI area (for inline mode).
    pub fn println(text: impl Into<String>) -> Self {
        Command::terminal(TerminalCommand::Println(text.into()))
    }

    /// Print formatted text above the TUI area (for inline mode).
    pub fn printf(text: impl Into<String>) -> Self {
        Command::terminal(TerminalCommand::Printf(text.into()))
    }

    // Convenience terminal command constructors

    /// Switch to the alternate screen buffer.
    pub fn enter_alt_screen() -> Self {
        Command::terminal(TerminalCommand::EnterAltScreen)
    }

    /// Return to the primary screen buffer.
    pub fn exit_alt_screen() -> Self {
        Command::terminal(TerminalCommand::ExitAltScreen)
    }

    /// Enable mouse capture in cell-motion mode (click, release, wheel, drag).
    pub fn enable_mouse_capture() -> Self {
        Command::terminal(TerminalCommand::EnableMouseCapture(MouseMode::CellMotion))
    }

    /// Enable mouse capture in all-motion mode (includes hover events).
    pub fn enable_mouse_all() -> Self {
        Command::terminal(TerminalCommand::EnableMouseCapture(MouseMode::AllMotion))
    }

    /// Disable mouse event capture.
    pub fn disable_mouse() -> Self {
        Command::terminal(TerminalCommand::DisableMouse)
    }

    /// Make the terminal cursor visible.
    pub fn show_cursor() -> Self {
        Command::terminal(TerminalCommand::ShowCursor)
    }

    /// Hide the terminal cursor.
    pub fn hide_cursor() -> Self {
        Command::terminal(TerminalCommand::HideCursor)
    }

    /// Set the terminal window title.
    pub fn set_title(title: impl Into<String>) -> Self {
        Command::terminal(TerminalCommand::SetTitle(title.into()))
    }

    /// Clear the entire terminal screen.
    pub fn clear_screen() -> Self {
        Command::terminal(TerminalCommand::ClearScreen)
    }

    /// Scroll the terminal viewport up by the given number of lines.
    pub fn scroll_up(lines: u16) -> Self {
        Command::terminal(TerminalCommand::ScrollUp(lines))
    }

    /// Scroll the terminal viewport down by the given number of lines.
    pub fn scroll_down(lines: u16) -> Self {
        Command::terminal(TerminalCommand::ScrollDown(lines))
    }

    /// Suspend the process (send SIGTSTP on Unix).
    pub fn suspend() -> Self {
        Command::terminal(TerminalCommand::Suspend)
    }

    // --- Inspection methods (useful for testing) ---

    /// Returns `true` if this is a no-op command.
    pub fn is_none(&self) -> bool {
        matches!(self.inner, CommandInner::None)
    }

    /// If this command is an immediate message action, return it.
    pub fn into_message(self) -> Option<Msg> {
        match self.inner {
            CommandInner::Action(Action::Message(msg)) => Some(msg),
            _ => None,
        }
    }

    /// If this command is a batch, return the inner commands.
    pub fn into_batch(self) -> Option<Vec<Command<Msg>>> {
        match self.inner {
            CommandInner::Batch(cmds) => Some(cmds),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_none_is_none() {
        let cmd: Command<()> = Command::none();
        assert!(matches!(cmd.inner, CommandInner::None));
    }

    #[test]
    fn command_message_creates_action() {
        let cmd: Command<i32> = Command::message(42);
        match cmd.inner {
            CommandInner::Action(Action::Message(msg)) => assert_eq!(msg, 42),
            _ => panic!("Expected Action::Message"),
        }
    }

    #[test]
    fn command_quit_creates_quit() {
        let cmd: Command<()> = Command::quit();
        assert!(matches!(cmd.inner, CommandInner::Action(Action::Quit)));
    }

    #[test]
    fn command_batch_empty_returns_none() {
        let cmd: Command<()> = Command::batch(vec![]);
        assert!(matches!(cmd.inner, CommandInner::None));
    }

    #[test]
    fn command_batch_single_unwraps() {
        let cmd: Command<i32> = Command::batch(vec![Command::message(1)]);
        match cmd.inner {
            CommandInner::Action(Action::Message(msg)) => assert_eq!(msg, 1),
            _ => panic!("Expected single command unwrapped"),
        }
    }

    #[test]
    fn command_batch_multiple() {
        let cmd: Command<i32> = Command::batch(vec![Command::message(1), Command::message(2)]);
        match cmd.inner {
            CommandInner::Batch(cmds) => assert_eq!(cmds.len(), 2),
            _ => panic!("Expected Batch"),
        }
    }

    #[test]
    fn command_sequence_empty_returns_none() {
        let cmd: Command<()> = Command::sequence(vec![]);
        assert!(matches!(cmd.inner, CommandInner::None));
    }

    #[test]
    fn command_map_none() {
        let cmd: Command<i32> = Command::none();
        let mapped: Command<String> = cmd.map(|n| n.to_string());
        assert!(matches!(mapped.inner, CommandInner::None));
    }

    #[test]
    fn command_map_message() {
        let cmd: Command<i32> = Command::message(42);
        let mapped: Command<String> = cmd.map(|n| n.to_string());
        match mapped.inner {
            CommandInner::Action(Action::Message(s)) => assert_eq!(s, "42"),
            _ => panic!("Expected mapped message"),
        }
    }

    #[test]
    fn command_map_quit_stays_quit() {
        let cmd: Command<i32> = Command::quit();
        let mapped: Command<String> = cmd.map(|n| n.to_string());
        assert!(matches!(mapped.inner, CommandInner::Action(Action::Quit)));
    }

    #[test]
    fn command_map_terminal_preserves_command() {
        let cmd: Command<i32> = Command::enter_alt_screen();
        let mapped: Command<String> = cmd.map(|n| n.to_string());
        match mapped.inner {
            CommandInner::Terminal(TerminalCommand::EnterAltScreen) => {}
            _ => panic!("Expected terminal command preserved"),
        }
    }

    #[test]
    fn command_map_batch() {
        let cmd: Command<i32> = Command::batch(vec![Command::message(1), Command::message(2)]);
        let mapped: Command<String> = cmd.map(|n| n.to_string());
        match mapped.inner {
            CommandInner::Batch(cmds) => assert_eq!(cmds.len(), 2),
            _ => panic!("Expected mapped batch"),
        }
    }

    #[test]
    fn terminal_command_constructors() {
        let cmd: Command<()> = Command::enter_alt_screen();
        assert!(matches!(
            cmd.inner,
            CommandInner::Terminal(TerminalCommand::EnterAltScreen)
        ));

        let cmd: Command<()> = Command::show_cursor();
        assert!(matches!(
            cmd.inner,
            CommandInner::Terminal(TerminalCommand::ShowCursor)
        ));

        let cmd: Command<()> = Command::set_title("test");
        match cmd.inner {
            CommandInner::Terminal(TerminalCommand::SetTitle(s)) => assert_eq!(s, "test"),
            _ => panic!("Expected SetTitle"),
        }
    }
}
