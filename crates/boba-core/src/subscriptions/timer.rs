use crate::subscription::{SubscriptionId, SubscriptionSource};
use futures::stream::BoxStream;
use futures::StreamExt;
use std::time::{Duration, Instant};

/// A repeating timer that fires at a fixed interval.
///
/// Each tick emits the current [`Instant`]. The `id` field allows multiple
/// `Every` subscriptions to coexist with distinct identities.
///
/// # Example
///
/// ```rust,ignore
/// use std::time::Duration;
/// use boba_core::subscriptions::Every;
/// use boba_core::subscription::subscribe;
///
/// let sub = subscribe(Every::new(Duration::from_secs(1), "tick"))
///     .map(|instant| Msg::Tick(instant));
/// ```
pub struct Every {
    /// The interval between ticks.
    pub interval: Duration,
    /// A string identifier used to distinguish this timer from others.
    pub id: &'static str,
}

impl Every {
    /// Create a new repeating timer with the given interval and identifier.
    pub fn new(interval: Duration, id: &'static str) -> Self {
        Self { interval, id }
    }
}

impl SubscriptionSource for Every {
    type Output = Instant;

    fn id(&self) -> SubscriptionId {
        SubscriptionId::with_str::<Self>(self.id)
    }

    fn stream(self) -> BoxStream<'static, Instant> {
        let stream = tokio_stream::wrappers::IntervalStream::new(tokio::time::interval(
            self.interval,
        ))
        .map(|tick| tick.into_std());
        Box::pin(stream)
    }
}

/// A one-shot delay that fires once after the specified duration.
///
/// Emits a single [`Instant`] when the delay elapses, then the subscription
/// stream completes.
///
/// # Example
///
/// ```rust,ignore
/// use std::time::Duration;
/// use boba_core::subscriptions::After;
/// use boba_core::subscription::subscribe;
///
/// let sub = subscribe(After::new(Duration::from_millis(500)))
///     .map(|_| Msg::DelayElapsed);
/// ```
pub struct After {
    /// How long to wait before firing.
    pub duration: Duration,
}

impl After {
    /// Create a new one-shot delay for the given duration.
    pub fn new(duration: Duration) -> Self {
        Self { duration }
    }
}

impl SubscriptionSource for After {
    type Output = Instant;

    fn id(&self) -> SubscriptionId {
        SubscriptionId::new::<Self>(self.duration.as_nanos() as u64)
    }

    fn stream(self) -> BoxStream<'static, Instant> {
        let stream = futures::stream::once(async move {
            tokio::time::sleep(self.duration).await;
            Instant::now()
        });
        Box::pin(stream)
    }
}
