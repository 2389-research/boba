use crate::command::{Action, Command, CommandInner, MouseMode, TerminalCommand};
use crate::model::Model;
use crate::subscription::SubscriptionManager;
use crossterm::{
    cursor::{self, SetCursorStyle as CrosstermSetCursorStyle},
    event::{
        DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste,
        EnableFocusChange, EnableMouseCapture,
    },
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen, SetTitle,
    },
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, stderr, stdout, Stderr, Stdout, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time::MissedTickBehavior;

/// Output target for the terminal UI.
///
/// By default the TUI renders to **stdout**.  When your program's stdout is
/// piped (e.g. to capture structured output), switch to [`Stderr`](OutputTarget::Stderr)
/// so the UI goes to the terminal while data flows through the pipe.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum OutputTarget {
    /// Write to stdout (default).
    #[default]
    Stdout,
    /// Write to stderr (useful when stdout is piped).
    Stderr,
}

/// Writer that wraps either stdout or stderr.
enum Output {
    Stdout(Stdout),
    Stderr(Stderr),
}

impl Write for Output {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Output::Stdout(w) => w.write(buf),
            Output::Stderr(w) => w.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Output::Stdout(w) => w.flush(),
            Output::Stderr(w) => w.flush(),
        }
    }
}

impl Output {
    fn new(target: OutputTarget) -> Self {
        match target {
            OutputTarget::Stdout => Output::Stdout(stdout()),
            OutputTarget::Stderr => Output::Stderr(stderr()),
        }
    }
}

/// Errors that can occur while initializing or running a [`Program`].
#[derive(Debug, thiserror::Error)]
pub enum ProgramError {
    /// An I/O error from terminal setup, rendering, or teardown.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Configuration options for a [`Program`].
///
/// All fields have sensible defaults (see [`Default`] impl).  Use struct
/// update syntax to override only the options you need:
///
/// # Example
///
/// ```rust,ignore
/// use boba_core::{ProgramOptions, OutputTarget, MouseMode};
///
/// let opts = ProgramOptions {
///     fps: 30,
///     mouse_mode: Some(MouseMode::Normal),
///     title: Some("My App".into()),
///     output: OutputTarget::Stderr,
///     ..ProgramOptions::default()
/// };
/// ```
pub struct ProgramOptions {
    /// Target frames per second (default: 60, max: 120).
    pub fps: u32,
    /// Start in alternate screen (default: true).
    pub alt_screen: bool,
    /// Enable mouse capture mode.
    pub mouse_mode: Option<MouseMode>,
    /// Enable bracketed paste (default: true).
    pub bracketed_paste: bool,
    /// Enable focus reporting.
    pub focus_reporting: bool,
    /// Set terminal title.
    pub title: Option<String>,
    /// Whether to catch panics and restore terminal (default: true).
    pub catch_panics: bool,
    /// Whether to handle signals gracefully (default: true).
    pub handle_signals: bool,
    /// Log file path for debugging TUI apps.
    pub log_file: Option<std::path::PathBuf>,
    /// Output target: stdout (default) or stderr.
    pub output: OutputTarget,
}

impl Default for ProgramOptions {
    fn default() -> Self {
        Self {
            fps: 60,
            alt_screen: true,
            mouse_mode: None,
            bracketed_paste: true,
            focus_reporting: false,
            title: None,
            catch_panics: true,
            handle_signals: true,
            log_file: None,
            output: OutputTarget::default(),
        }
    }
}

/// A cloneable handle to a running [`Program`] for external control.
///
/// `ProgramHandle` is [`Clone`] and can safely be sent across threads or into
/// async tasks.  It provides two capabilities:
///
/// * [`send`](ProgramHandle::send) -- inject a message into the program's
///   event loop from outside.
/// * [`kill`](ProgramHandle::kill) -- force the program to exit immediately.
///
/// Obtain a handle by calling [`Program::handle`] before entering the run
/// loop.
#[derive(Clone)]
pub struct ProgramHandle<Msg: Send + 'static> {
    msg_tx: mpsc::UnboundedSender<Msg>,
    killed: Arc<AtomicBool>,
}

impl<Msg: Send + 'static> ProgramHandle<Msg> {
    /// Send a message to the running program.
    ///
    /// The message is enqueued on an unbounded channel and will be processed
    /// on the next iteration of the event loop.  Returns silently if the
    /// program has already exited.
    pub fn send(&self, msg: Msg) {
        let _ = self.msg_tx.send(msg);
    }

    /// Force-kill the program immediately.
    ///
    /// Sets an atomic flag that the event loop checks on every iteration.
    /// The program will exit at the next opportunity without processing
    /// remaining messages.
    pub fn kill(&self) {
        self.killed.store(true, Ordering::SeqCst);
    }
}

/// The program runtime.  Manages terminal setup, the event loop, and the
/// full [`Model`] lifecycle.
///
/// `Program` wires a [`Model`] to a real terminal via
/// [`ratatui`]/[`crossterm`] and drives the init/update/view loop until the
/// model returns [`Command::quit()`] or the process receives a signal.
///
/// # Example
///
/// ```rust,ignore
/// use boba_core::{Program, ProgramError};
///
/// #[tokio::main]
/// async fn main() -> Result<(), ProgramError> {
///     let model = Program::<MyApp>::new(())?.run().await?;
///     // `model` is the final state after quit
///     Ok(())
/// }
/// ```
pub struct Program<M: Model> {
    model: M,
    terminal: Terminal<CrosstermBackend<Output>>,
    msg_tx: mpsc::UnboundedSender<M::Message>,
    msg_rx: mpsc::UnboundedReceiver<M::Message>,
    subscription_manager: SubscriptionManager<M::Message>,
    options: ProgramOptions,
    needs_redraw: bool,
    should_quit: bool,
    killed: Arc<AtomicBool>,
    #[allow(clippy::type_complexity)]
    filter: Option<Box<dyn Fn(M::Message) -> Option<M::Message> + Send>>,
    terminal_released: bool,
    log_file: Option<std::fs::File>,
}

impl<M: Model> Program<M> {
    /// Create a new program with default options.
    ///
    /// Returns an error if terminal initialization fails.
    pub fn new(flags: M::Flags) -> Result<Self, ProgramError> {
        Self::with_options(flags, ProgramOptions::default())
    }

    /// Create a new program with custom options.
    ///
    /// Returns an error if terminal initialization fails.
    pub fn with_options(flags: M::Flags, options: ProgramOptions) -> Result<Self, ProgramError> {
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();

        // Open log file if specified
        let log_file = if let Some(ref path) = options.log_file {
            Some(
                std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)?,
            )
        } else {
            None
        };

        let (model, init_cmd) = M::init(flags);

        let terminal = init_terminal(&options)?;
        let subscription_manager = SubscriptionManager::new(msg_tx.clone());
        let killed = Arc::new(AtomicBool::new(false));

        let mut program = Self {
            model,
            terminal,
            msg_tx,
            msg_rx,
            subscription_manager,
            options,
            needs_redraw: true,
            should_quit: false,
            killed,
            filter: None,
            terminal_released: false,
            log_file,
        };

        program.debug_log("program initialized");

        // Execute the initial command
        program.execute_command(init_cmd);

        // Initial subscription reconciliation
        let subs = program.model.subscriptions();
        program.subscription_manager.reconcile(subs);

        Ok(program)
    }

    /// Set a message filter. Messages pass through the filter before reaching `update`.
    /// Return `Some(msg)` to pass (possibly transformed), `None` to drop.
    pub fn with_filter(
        mut self,
        filter: impl Fn(M::Message) -> Option<M::Message> + Send + 'static,
    ) -> Self {
        self.filter = Some(Box::new(filter));
        self
    }

    /// Get a sender for external message injection.
    pub fn sender(&self) -> mpsc::UnboundedSender<M::Message> {
        self.msg_tx.clone()
    }

    /// Get a handle for external control (send messages, force-kill).
    pub fn handle(&self) -> ProgramHandle<M::Message> {
        ProgramHandle {
            msg_tx: self.msg_tx.clone(),
            killed: self.killed.clone(),
        }
    }

    /// Run the program. Blocks until quit.
    pub async fn run(mut self) -> Result<M, ProgramError> {
        self.event_loop().await?;

        // Cleanup
        self.debug_log("shutting down");
        self.subscription_manager.shutdown();
        if !self.terminal_released {
            restore_terminal(&self.options)?;
        }

        Ok(self.model)
    }

    /// Temporarily release terminal control without quitting.
    pub fn release_terminal(&mut self) -> Result<(), ProgramError> {
        if !self.terminal_released {
            restore_terminal(&self.options)?;
            self.terminal_released = true;
        }
        Ok(())
    }

    /// Re-acquire terminal after `release_terminal()`.
    pub fn restore_terminal_control(&mut self) -> Result<(), ProgramError> {
        if self.terminal_released {
            self.terminal = init_terminal(&self.options)?;
            self.terminal_released = false;
            self.needs_redraw = true;
        }
        Ok(())
    }

    async fn event_loop(&mut self) -> Result<(), ProgramError> {
        // Initial render
        self.render()?;

        let fps = self.options.fps.clamp(1, 120);
        let mut frame_interval =
            tokio::time::interval(Duration::from_secs_f64(1.0 / fps as f64));
        frame_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        let handle_signals = self.options.handle_signals;

        loop {
            // Check for kill signal
            if self.killed.load(Ordering::SeqCst) {
                return Ok(());
            }

            tokio::select! {
                biased;

                _ = tokio::signal::ctrl_c(), if handle_signals => {
                    self.debug_log("received ctrl+c signal");
                    self.should_quit = true;
                    return Ok(());
                }

                Some(msg) = self.msg_rx.recv() => {
                    self.process_message(msg);

                    // Micro-batch: drain additional messages within 100μs, up to 100 messages
                    let deadline = Instant::now() + Duration::from_micros(100);
                    let mut batch_count = 0u32;
                    while Instant::now() < deadline && batch_count < 100 {
                        match self.msg_rx.try_recv() {
                            Ok(msg) => {
                                self.process_message(msg);
                                batch_count += 1;
                            }
                            Err(_) => break,
                        }
                    }

                    if self.should_quit || self.killed.load(Ordering::SeqCst) {
                        return Ok(());
                    }
                }

                _ = frame_interval.tick() => {
                    if self.needs_redraw && !self.terminal_released {
                        self.render()?;
                        self.needs_redraw = false;
                    }
                }
            }
        }
    }

    fn process_message(&mut self, msg: M::Message) {
        // Apply filter if set
        let msg = if let Some(ref filter) = self.filter {
            match filter(msg) {
                Some(msg) => msg,
                None => return, // Message filtered out
            }
        } else {
            msg
        };

        let cmd = self.model.update(msg);
        self.execute_command(cmd);

        // Reconcile subscriptions
        let subs = self.model.subscriptions();
        self.subscription_manager.reconcile(subs);

        self.needs_redraw = true;
    }

    fn execute_command(&mut self, cmd: Command<M::Message>) {
        match cmd.inner {
            CommandInner::None => {}
            CommandInner::Action(Action::Message(msg)) => {
                let _ = self.msg_tx.send(msg);
            }
            CommandInner::Action(Action::Quit) => {
                self.should_quit = true;
            }
            CommandInner::Future(fut) => {
                let tx = self.msg_tx.clone();
                tokio::spawn(async move {
                    let msg = fut.await;
                    let _ = tx.send(msg);
                });
            }
            CommandInner::Stream(stream) => {
                use futures::StreamExt;
                let tx = self.msg_tx.clone();
                tokio::spawn(async move {
                    futures::pin_mut!(stream);
                    while let Some(msg) = stream.next().await {
                        if tx.send(msg).is_err() {
                            break;
                        }
                    }
                });
            }
            CommandInner::Batch(cmds) => {
                for cmd in cmds {
                    self.execute_command(cmd);
                }
            }
            CommandInner::Sequence(cmds) => {
                let tx = self.msg_tx.clone();
                tokio::spawn(async move {
                    for cmd in cmds {
                        execute_command_sequential(cmd, &tx).await;
                    }
                });
            }
            CommandInner::Terminal(tcmd) => {
                self.execute_terminal_command(tcmd);
            }
            CommandInner::Exec { cmd: exec_cmd, on_exit } => {
                // Release terminal, run process, restore terminal
                let _ = self.release_terminal();

                let mut process = std::process::Command::new(&exec_cmd.program);
                process.args(&exec_cmd.args);
                if let Some(dir) = &exec_cmd.working_dir {
                    process.current_dir(dir);
                }
                process
                    .stdin(std::process::Stdio::inherit())
                    .stdout(std::process::Stdio::inherit())
                    .stderr(std::process::Stdio::inherit());
                let result = process.status();

                let _ = self.restore_terminal_control();
                let msg = on_exit(result);
                let _ = self.msg_tx.send(msg);
            }
        }
    }

    fn execute_terminal_command(&mut self, cmd: TerminalCommand) {
        let mut writer = Output::new(self.options.output);
        match cmd {
            TerminalCommand::EnterAltScreen => {
                execute!(writer, EnterAlternateScreen).ok();
            }
            TerminalCommand::ExitAltScreen => {
                execute!(writer, LeaveAlternateScreen).ok();
            }
            TerminalCommand::EnableMouseCapture(_mode) => {
                execute!(writer, EnableMouseCapture).ok();
            }
            TerminalCommand::DisableMouse => {
                execute!(writer, DisableMouseCapture).ok();
            }
            TerminalCommand::ShowCursor => {
                execute!(writer, cursor::Show).ok();
            }
            TerminalCommand::HideCursor => {
                execute!(writer, cursor::Hide).ok();
            }
            TerminalCommand::SetCursorStyle(style) => {
                let ct_style = match style {
                    crate::command::CursorStyle::DefaultUserShape => {
                        CrosstermSetCursorStyle::DefaultUserShape
                    }
                    crate::command::CursorStyle::BlinkingBlock => {
                        CrosstermSetCursorStyle::BlinkingBlock
                    }
                    crate::command::CursorStyle::SteadyBlock => {
                        CrosstermSetCursorStyle::SteadyBlock
                    }
                    crate::command::CursorStyle::BlinkingUnderScore => {
                        CrosstermSetCursorStyle::BlinkingUnderScore
                    }
                    crate::command::CursorStyle::SteadyUnderScore => {
                        CrosstermSetCursorStyle::SteadyUnderScore
                    }
                    crate::command::CursorStyle::BlinkingBar => {
                        CrosstermSetCursorStyle::BlinkingBar
                    }
                    crate::command::CursorStyle::SteadyBar => {
                        CrosstermSetCursorStyle::SteadyBar
                    }
                };
                execute!(writer, ct_style).ok();
            }
            TerminalCommand::EnableBracketedPaste => {
                execute!(writer, EnableBracketedPaste).ok();
            }
            TerminalCommand::DisableBracketedPaste => {
                execute!(writer, DisableBracketedPaste).ok();
            }
            TerminalCommand::EnableFocusReporting => {
                execute!(writer, EnableFocusChange).ok();
            }
            TerminalCommand::DisableFocusReporting => {
                execute!(writer, DisableFocusChange).ok();
            }
            TerminalCommand::SetTitle(title) => {
                execute!(writer, SetTitle(title)).ok();
            }
            TerminalCommand::ClearScreen => {
                execute!(writer, crossterm::terminal::Clear(crossterm::terminal::ClearType::All)).ok();
            }
            TerminalCommand::ScrollUp(n) => {
                execute!(writer, crossterm::terminal::ScrollUp(n)).ok();
            }
            TerminalCommand::ScrollDown(n) => {
                execute!(writer, crossterm::terminal::ScrollDown(n)).ok();
            }
            TerminalCommand::Println(text) => {
                // Use \r\n to produce correct output in raw mode (raw mode does
                // not translate \n to \r\n).
                execute!(writer, crossterm::style::Print(format!("{text}\r\n"))).ok();
            }
            TerminalCommand::Printf(text) => {
                execute!(writer, crossterm::style::Print(text)).ok();
            }
            TerminalCommand::Suspend => {
                self.suspend();
            }
        }
    }

    /// Write a debug message to the log file, if configured.
    fn debug_log(&mut self, msg: &str) {
        if let Some(ref mut f) = self.log_file {
            let _ = writeln!(f, "{msg}");
        }
    }

    fn suspend(&mut self) {
        restore_terminal(&self.options).ok();

        #[cfg(unix)]
        {
            unsafe {
                libc::raise(libc::SIGTSTP);
            }
        }

        match init_terminal(&self.options) {
            Ok(terminal) => {
                self.terminal = terminal;
                self.needs_redraw = true;
            }
            Err(_) => {
                // Terminal re-init failed (e.g., detached). Signal quit so the
                // event loop can exit gracefully rather than panicking.
                self.should_quit = true;
            }
        }
    }

    fn render(&mut self) -> Result<(), ProgramError> {
        self.terminal.draw(|frame| {
            self.model.view(frame);
        })?;
        Ok(())
    }
}

/// Execute a command sequentially (for `Command::sequence`).
fn execute_command_sequential<Msg: Send + 'static>(
    cmd: Command<Msg>,
    tx: &mpsc::UnboundedSender<Msg>,
) -> futures::future::BoxFuture<'_, ()> {
    Box::pin(async move {
        match cmd.inner {
            CommandInner::None => {}
            CommandInner::Action(Action::Message(msg)) => {
                let _ = tx.send(msg);
            }
            CommandInner::Action(Action::Quit) => {
                // Can't easily signal quit from here; send would need a special channel
            }
            CommandInner::Future(fut) => {
                let msg = fut.await;
                let _ = tx.send(msg);
            }
            CommandInner::Stream(stream) => {
                use futures::StreamExt;
                futures::pin_mut!(stream);
                while let Some(msg) = stream.next().await {
                    if tx.send(msg).is_err() {
                        break;
                    }
                }
            }
            CommandInner::Batch(cmds) => {
                // In a sequence, batch still runs concurrently within itself
                let handles: Vec<_> = cmds
                    .into_iter()
                    .map(|cmd| {
                        let tx = tx.clone();
                        tokio::spawn(async move {
                            execute_command_sequential(cmd, &tx).await;
                        })
                    })
                    .collect();
                for handle in handles {
                    handle.await.ok();
                }
            }
            CommandInner::Sequence(cmds) => {
                for cmd in cmds {
                    execute_command_sequential(cmd, tx).await;
                }
            }
            CommandInner::Terminal(_) => {
                // Terminal commands from a sequential context are not supported
                // (they need mutable terminal access)
            }
            CommandInner::Exec { .. } => {
                // Exec commands from a sequential context are not supported
                // (they need mutable terminal access)
            }
        }
    })
}

fn init_terminal(options: &ProgramOptions) -> Result<Terminal<CrosstermBackend<Output>>, ProgramError> {
    // Install panic hook that restores terminal (only once to avoid stacking)
    if options.catch_panics {
        use std::sync::Once;
        static HOOK_INSTALLED: Once = Once::new();
        let alt_screen = options.alt_screen;
        let output_target = options.output;
        HOOK_INSTALLED.call_once(|| {
            let original_hook = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |info| {
                let _ = restore_terminal_minimal(alt_screen, output_target);
                original_hook(info);
            }));
        });
    }

    enable_raw_mode()?;
    let mut writer = Output::new(options.output);

    if options.alt_screen {
        execute!(writer, EnterAlternateScreen)?;
    }
    if options.bracketed_paste {
        execute!(writer, EnableBracketedPaste)?;
    }
    if options.mouse_mode.is_some() {
        execute!(writer, EnableMouseCapture)?;
    }
    if options.focus_reporting {
        execute!(writer, EnableFocusChange)?;
    }
    if let Some(ref title) = options.title {
        execute!(writer, SetTitle(title))?;
    }
    execute!(writer, cursor::Hide)?;

    let backend = CrosstermBackend::new(writer);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal(options: &ProgramOptions) -> Result<(), ProgramError> {
    restore_terminal_minimal(options.alt_screen, options.output)?;
    Ok(())
}

fn restore_terminal_minimal(alt_screen: bool, output_target: OutputTarget) -> Result<(), std::io::Error> {
    // Use best-effort cleanup: continue even if individual steps fail,
    // so we restore as much terminal state as possible.
    let r1 = disable_raw_mode();
    let mut writer = Output::new(output_target);
    execute!(writer, DisableBracketedPaste).ok();
    execute!(writer, DisableMouseCapture).ok();
    execute!(writer, DisableFocusChange).ok();
    execute!(writer, cursor::Show).ok();
    if alt_screen {
        execute!(writer, LeaveAlternateScreen).ok();
    }
    // Propagate the raw mode error if it was the only failure that matters
    r1
}

/// Open a log file for debugging TUI applications.
///
/// Returns a file handle that can be used with `writeln!` or passed to
/// a logging framework. The file is opened in append mode.
///
/// This is the equivalent of Bubble Tea's `tea.LogToFile()`.
///
/// # Example
///
/// ```no_run
/// use boba_core::runtime::log_to_file;
/// use std::io::Write;
///
/// let mut f = log_to_file("debug.log").unwrap();
/// writeln!(f, "debug message").unwrap();
/// ```
pub fn log_to_file(path: impl AsRef<std::path::Path>) -> Result<std::fs::File, std::io::Error> {
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
}
