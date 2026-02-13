use crate::command::{Action, Command, CommandInner};
use crate::model::Model;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::Terminal;

/// A headless test harness that drives a [`Model`] without a real terminal.
///
/// `TestProgram` lets you exercise every part of the init/update/view cycle in
/// a plain `#[test]` function -- no tokio runtime or TTY required.  Synchronous
/// commands (e.g. [`Command::message`]) are collected and can be flushed with
/// [`drain_messages`](TestProgram::drain_messages); async commands and terminal
/// commands are silently ignored.
///
/// # Example
///
/// ```rust,ignore
/// use boba_core::testing::TestProgram;
///
/// let mut prog = TestProgram::<Counter>::new(0);  // calls Counter::init(0)
/// prog.send(CounterMsg::Increment);               // triggers update
/// prog.send(CounterMsg::Increment);
/// assert_eq!(prog.model().count, 2);              // inspect state
///
/// let output = prog.render_string(40, 1);          // render to string
/// assert!(output.contains("Count: 2"));
/// ```
pub struct TestProgram<M: Model> {
    model: M,
    pending_messages: Vec<M::Message>,
}

impl<M: Model> TestProgram<M> {
    /// Create a test program by calling [`Model::init`] with the given flags.
    ///
    /// Any synchronous commands produced by `init` (e.g. [`Command::message`])
    /// are collected into the pending-message queue.  Call
    /// [`drain_messages`](TestProgram::drain_messages) to process them.
    pub fn new(flags: M::Flags) -> Self {
        let (model, init_cmd) = M::init(flags);
        let mut program = Self {
            model,
            pending_messages: Vec::new(),
        };
        program.collect_sync_messages(init_cmd);
        program
    }

    /// Send a message, triggering a single update cycle.
    ///
    /// The message is passed to [`Model::update`] immediately.  Any
    /// synchronous commands returned by `update` are enqueued; call
    /// [`drain_messages`](TestProgram::drain_messages) to flush them.
    pub fn send(&mut self, msg: M::Message) {
        let cmd = self.model.update(msg);
        self.collect_sync_messages(cmd);
    }

    /// Process all pending synchronous messages produced by [`Command::message`].
    ///
    /// Repeatedly drains the pending queue, calling [`Model::update`] for each
    /// message, until no new synchronous messages are generated.  This is
    /// useful for testing command-chaining scenarios where one update produces
    /// a message that triggers another update.
    pub fn drain_messages(&mut self) {
        while !self.pending_messages.is_empty() {
            let messages: Vec<_> = self.pending_messages.drain(..).collect();
            for msg in messages {
                let cmd = self.model.update(msg);
                self.collect_sync_messages(cmd);
            }
        }
    }

    /// Get a shared reference to the model for assertions.
    pub fn model(&self) -> &M {
        &self.model
    }

    /// Get a mutable reference to the model for direct test setup.
    ///
    /// This bypasses the normal message-driven update cycle, which can be
    /// useful for arranging test state before sending messages.
    pub fn model_mut(&mut self) -> &mut M {
        &mut self.model
    }

    /// Render the model to a ratatui [`Buffer`] of the given dimensions.
    ///
    /// Returns the raw buffer, which you can inspect cell-by-cell.  For a
    /// simpler string-based assertion, see
    /// [`render_string`](TestProgram::render_string).
    pub fn render(&self, width: u16, height: u16) -> Buffer {
        let backend = ratatui::backend::TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                self.model.view(frame);
            })
            .unwrap();
        terminal.backend().buffer().clone()
    }

    /// Render the model and return the visible content as a plain string.
    ///
    /// Each row of the buffer is concatenated into a line; rows are separated
    /// by newlines.  Trailing whitespace within each row is preserved.
    pub fn render_string(&self, width: u16, height: u16) -> String {
        let buf = self.render(width, height);
        let area = Rect::new(0, 0, width, height);
        let mut output = String::new();
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                let cell = &buf[(x, y)];
                output.push_str(cell.symbol());
            }
            if y < area.bottom() - 1 {
                output.push('\n');
            }
        }
        output
    }

    fn collect_sync_messages(&mut self, cmd: Command<M::Message>) {
        match cmd.inner {
            CommandInner::None => {}
            CommandInner::Action(Action::Message(msg)) => {
                self.pending_messages.push(msg);
            }
            CommandInner::Action(Action::Quit) => {}
            CommandInner::Batch(cmds) => {
                for cmd in cmds {
                    self.collect_sync_messages(cmd);
                }
            }
            CommandInner::Sequence(cmds) => {
                for cmd in cmds {
                    self.collect_sync_messages(cmd);
                }
            }
            // Async commands can't be executed synchronously in tests
            CommandInner::Future(_) | CommandInner::Stream(_) => {}
            CommandInner::Terminal(_) => {}
            CommandInner::Exec { .. } => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::widgets::Paragraph;

    // A minimal counter model for testing
    struct Counter {
        count: i64,
    }

    #[derive(Debug)]
    enum CounterMsg {
        Increment,
        Decrement,
        Reset,
    }

    impl Model for Counter {
        type Message = CounterMsg;
        type Flags = i64;

        fn init(initial: i64) -> (Self, Command<CounterMsg>) {
            (Counter { count: initial }, Command::none())
        }

        fn update(&mut self, msg: CounterMsg) -> Command<CounterMsg> {
            match msg {
                CounterMsg::Increment => self.count += 1,
                CounterMsg::Decrement => self.count -= 1,
                CounterMsg::Reset => self.count = 0,
            }
            Command::none()
        }

        fn view(&self, frame: &mut ratatui::Frame) {
            let area = frame.area();
            let text = format!("Count: {}", self.count);
            let paragraph = Paragraph::new(text);
            frame.render_widget(paragraph, area);
        }
    }

    #[test]
    fn test_program_init() {
        let prog = TestProgram::<Counter>::new(0);
        assert_eq!(prog.model().count, 0);
    }

    #[test]
    fn test_program_init_with_flags() {
        let prog = TestProgram::<Counter>::new(42);
        assert_eq!(prog.model().count, 42);
    }

    #[test]
    fn test_program_send_increment() {
        let mut prog = TestProgram::<Counter>::new(0);
        prog.send(CounterMsg::Increment);
        assert_eq!(prog.model().count, 1);
    }

    #[test]
    fn test_program_send_multiple() {
        let mut prog = TestProgram::<Counter>::new(0);
        prog.send(CounterMsg::Increment);
        prog.send(CounterMsg::Increment);
        prog.send(CounterMsg::Increment);
        prog.send(CounterMsg::Decrement);
        assert_eq!(prog.model().count, 2);
    }

    #[test]
    fn test_program_reset() {
        let mut prog = TestProgram::<Counter>::new(10);
        prog.send(CounterMsg::Increment);
        prog.send(CounterMsg::Reset);
        assert_eq!(prog.model().count, 0);
    }

    #[test]
    fn test_program_render() {
        let prog = TestProgram::<Counter>::new(0);
        let content = prog.render_string(40, 1);
        assert!(content.contains("Count: 0"));
    }

    #[test]
    fn test_program_render_after_update() {
        let mut prog = TestProgram::<Counter>::new(0);
        prog.send(CounterMsg::Increment);
        prog.send(CounterMsg::Increment);
        prog.send(CounterMsg::Increment);
        let content = prog.render_string(40, 1);
        assert!(content.contains("Count: 3"));
    }

    #[test]
    fn test_program_render_negative() {
        let mut prog = TestProgram::<Counter>::new(0);
        prog.send(CounterMsg::Decrement);
        let content = prog.render_string(40, 1);
        assert!(content.contains("Count: -1"));
    }

    // Test a model that uses Command::message for chaining
    struct ChainModel {
        steps: Vec<String>,
    }

    #[derive(Debug)]
    enum ChainMsg {
        Start,
        Step(String),
    }

    impl Model for ChainModel {
        type Message = ChainMsg;
        type Flags = ();

        fn init(_: ()) -> (Self, Command<ChainMsg>) {
            (ChainModel { steps: vec![] }, Command::none())
        }

        fn update(&mut self, msg: ChainMsg) -> Command<ChainMsg> {
            match msg {
                ChainMsg::Start => {
                    self.steps.push("started".into());
                    Command::message(ChainMsg::Step("auto".into()))
                }
                ChainMsg::Step(s) => {
                    self.steps.push(s);
                    Command::none()
                }
            }
        }

        fn view(&self, frame: &mut ratatui::Frame) {
            let text = self.steps.join(", ");
            frame.render_widget(Paragraph::new(text), frame.area());
        }
    }

    #[test]
    fn test_command_message_chaining() {
        let mut prog = TestProgram::<ChainModel>::new(());
        prog.send(ChainMsg::Start);
        // The Command::message should have queued ChainMsg::Step
        prog.drain_messages();
        assert_eq!(prog.model().steps, vec!["started", "auto"]);
    }
}
