use crate::event::TerminalEvent;
use crate::subscription::{SubscriptionId, SubscriptionSource};
use crossterm::event::EventStream;
use futures::stream::BoxStream;
use futures::StreamExt;
use std::sync::Arc;

/// Subscription source for terminal events (keyboard, mouse, resize, focus, paste).
///
/// # Input TTY behavior
///
/// crossterm 0.28's `EventStream::new()` internally calls `tty_fd()`, which
/// automatically opens `/dev/tty` when stdin is not a TTY (i.e., when stdin is
/// piped). This means the equivalent of Bubble Tea's `WithInputTTY()`
/// functionality is built-in: programs using boba will correctly receive
/// keyboard input even when stdin is redirected. For example,
/// `echo "data" | my_boba_app` will still read interactive keyboard events
/// from the terminal, not from the pipe.
pub struct TerminalEvents;

impl SubscriptionSource for TerminalEvents {
    type Output = TerminalEvent;

    fn id(&self) -> SubscriptionId {
        SubscriptionId::of::<Self>()
    }

    fn stream(self) -> BoxStream<'static, TerminalEvent> {
        let stream = EventStream::new().filter_map(|result| async move {
            match result {
                Ok(event) => Some(TerminalEvent::from(event)),
                Err(_) => None,
            }
        });
        Box::pin(stream)
    }
}

/// Create a terminal events subscription that maps each event through a
/// user-provided function.
///
/// The `map` closure receives every [`TerminalEvent`] and returns `Some(Msg)`
/// to forward it to the runtime or `None` to discard it.
///
/// # Example
///
/// ```rust,ignore
/// fn subscriptions(&self) -> Vec<Subscription<Msg>> {
///     vec![terminal_events(|event| match event {
///         TerminalEvent::Key(key) => Some(Msg::KeyPress(key)),
///         TerminalEvent::Resize(w, h) => Some(Msg::Resize(w, h)),
///         _ => None,
///     })]
/// }
/// ```
pub fn terminal_events<Msg: Send + 'static>(
    map: impl Fn(TerminalEvent) -> Option<Msg> + Send + Sync + 'static,
) -> crate::subscription::Subscription<Msg> {
    use crate::subscription::Subscription;
    use tokio::sync::mpsc;
    use tokio::task::AbortHandle;

    let id = SubscriptionId::of::<TerminalEvents>();
    let map = Arc::new(map);

    // Create EventStream lazily inside the spawned task, not eagerly.
    // Eager creation causes crossterm's global InternalEventReader to be
    // accessed on every subscriptions() call (each update cycle), which
    // interferes with the active EventStream's polling.
    Subscription {
        id,
        spawn: Box::new(move |tx: mpsc::UnboundedSender<Msg>| -> AbortHandle {
            let handle = tokio::spawn(async move {
                let stream = EventStream::new().filter_map(move |result| {
                    let map = map.clone();
                    async move {
                        match result {
                            Ok(event) => map(TerminalEvent::from(event)),
                            Err(_) => None,
                        }
                    }
                });
                futures::pin_mut!(stream);
                while let Some(msg) = stream.next().await {
                    if tx.send(msg).is_err() {
                        break;
                    }
                }
            });
            handle.abort_handle()
        }),
    }
}
