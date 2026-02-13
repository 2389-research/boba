use futures::stream::BoxStream;
use futures::StreamExt;
use std::any::TypeId;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use tokio::sync::mpsc;
use tokio::task::AbortHandle;

/// A long-lived event source managed by the runtime.
///
/// Subscriptions are declared in [`Model::subscriptions`](crate::Model::subscriptions) and automatically
/// started or stopped through diffing: the runtime compares the set of
/// subscriptions returned on each update cycle and starts any new ones while
/// stopping any that are no longer present.
pub struct Subscription<Msg: Send + 'static> {
    pub(crate) id: SubscriptionId,
    pub(crate) spawn: Box<dyn FnOnce(mpsc::UnboundedSender<Msg>) -> AbortHandle + Send>,
}

/// Identity for diffing subscriptions between update cycles.
///
/// Each subscription carries a `SubscriptionId` composed of a Rust [`TypeId`]
/// and an optional numeric discriminant. The runtime uses this to determine
/// which subscriptions are new, unchanged, or removed when reconciling.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SubscriptionId {
    type_id: TypeId,
    discriminant: u64,
}

impl SubscriptionId {
    /// Create an ID from a type and a numeric discriminant.
    pub fn new<T: 'static>(discriminant: u64) -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            discriminant,
        }
    }

    /// Create an ID from a type alone (for singletons).
    pub fn of<T: 'static>() -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            discriminant: 0,
        }
    }

    /// Create an ID from a type and a string discriminant.
    pub fn with_str<T: 'static>(s: &str) -> Self {
        let mut hasher = std::hash::DefaultHasher::new();
        s.hash(&mut hasher);
        Self {
            type_id: TypeId::of::<T>(),
            discriminant: hasher.finish(),
        }
    }
}

/// Trait for types that produce a stream of values.
///
/// Implement this to create custom subscription sources. The runtime will call
/// [`stream`](SubscriptionSource::stream) once when the subscription is first
/// started, and will drop the stream when the subscription is removed.
pub trait SubscriptionSource: Send + 'static {
    /// The type of values this source emits.
    type Output: Send + 'static;

    /// Unique ID for this subscription instance.
    fn id(&self) -> SubscriptionId;

    /// Create the stream of values.
    fn stream(self) -> BoxStream<'static, Self::Output>;
}

/// Create a [`Subscription`] from a [`SubscriptionSource`].
///
/// This spawns a tokio task that drives the source's stream and forwards
/// each emitted value to the runtime's message channel.
pub fn subscribe<S>(source: S) -> Subscription<S::Output>
where
    S: SubscriptionSource,
    S::Output: Send + 'static,
{
    let id = source.id();
    Subscription {
        id,
        spawn: Box::new(move |tx| {
            let handle = tokio::spawn(async move {
                let mut stream = source.stream();
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

impl<Msg: Send + 'static> Subscription<Msg> {
    /// Create from a raw stream and id.
    pub fn from_stream(id: SubscriptionId, stream: BoxStream<'static, Msg>) -> Self {
        Subscription {
            id,
            spawn: Box::new(move |tx| {
                let handle = tokio::spawn(async move {
                    let mut stream = stream;
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

    /// Transform the message type (for component composition).
    pub fn map<NewMsg: Send + 'static>(
        self,
        f: impl Fn(Msg) -> NewMsg + Send + Sync + 'static,
    ) -> Subscription<NewMsg> {
        let f = std::sync::Arc::new(f);
        Subscription {
            id: self.id,
            spawn: Box::new(move |new_tx: mpsc::UnboundedSender<NewMsg>| {
                let (inner_tx, mut inner_rx) = mpsc::unbounded_channel::<Msg>();
                let abort = (self.spawn)(inner_tx);

                tokio::spawn(async move {
                    while let Some(msg) = inner_rx.recv().await {
                        if new_tx.send(f(msg)).is_err() {
                            break;
                        }
                    }
                });

                // When source is aborted, inner_tx drops, inner_rx returns None,
                // and the mapper task ends naturally.
                abort
            }),
        }
    }
}

/// Manages active subscriptions, performing diffing between cycles.
pub(crate) struct SubscriptionManager<Msg: Send + 'static> {
    active: HashMap<SubscriptionId, AbortHandle>,
    msg_tx: mpsc::UnboundedSender<Msg>,
}

impl<Msg: Send + 'static> SubscriptionManager<Msg> {
    pub fn new(msg_tx: mpsc::UnboundedSender<Msg>) -> Self {
        Self {
            active: HashMap::new(),
            msg_tx,
        }
    }

    /// Diff new subscriptions against active ones.
    /// Start new ones, stop removed ones, keep unchanged ones.
    pub fn reconcile(&mut self, new_subs: Vec<Subscription<Msg>>) {
        let mut new_ids: HashMap<SubscriptionId, Subscription<Msg>> = HashMap::new();
        for sub in new_subs {
            new_ids.insert(sub.id.clone(), sub);
        }

        // Stop subscriptions that are no longer present
        let to_remove: Vec<SubscriptionId> = self
            .active
            .keys()
            .filter(|id| !new_ids.contains_key(id))
            .cloned()
            .collect();

        for id in to_remove {
            if let Some(handle) = self.active.remove(&id) {
                handle.abort();
            }
        }

        // Start subscriptions that are new
        for (id, sub) in new_ids {
            if !self.active.contains_key(&id) {
                let handle = (sub.spawn)(self.msg_tx.clone());
                self.active.insert(id, handle);
            }
        }
    }

    /// Abort all active subscriptions.
    pub fn shutdown(&mut self) {
        for (_, handle) in self.active.drain() {
            handle.abort();
        }
    }

    /// Number of active subscriptions (for testing).
    #[cfg(test)]
    pub fn active_count(&self) -> usize {
        self.active.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subscription_id_equality() {
        let id1 = SubscriptionId::of::<String>();
        let id2 = SubscriptionId::of::<String>();
        assert_eq!(id1, id2);
    }

    #[test]
    fn subscription_id_different_types() {
        let id1 = SubscriptionId::of::<String>();
        let id2 = SubscriptionId::of::<i32>();
        assert_ne!(id1, id2);
    }

    #[test]
    fn subscription_id_with_discriminant() {
        let id1 = SubscriptionId::new::<String>(1);
        let id2 = SubscriptionId::new::<String>(2);
        assert_ne!(id1, id2);
    }

    #[test]
    fn subscription_id_with_str() {
        let id1 = SubscriptionId::with_str::<String>("a");
        let id2 = SubscriptionId::with_str::<String>("b");
        assert_ne!(id1, id2);

        let id3 = SubscriptionId::with_str::<String>("a");
        assert_eq!(id1, id3);
    }

    #[tokio::test]
    async fn subscription_manager_starts_new() {
        let (tx, _rx) = mpsc::unbounded_channel::<i32>();
        let mut manager = SubscriptionManager::new(tx);

        let id = SubscriptionId::of::<String>();
        let stream: BoxStream<'static, i32> = Box::pin(futures::stream::pending());
        let sub = Subscription::from_stream(id.clone(), stream);

        manager.reconcile(vec![sub]);
        assert_eq!(manager.active_count(), 1);
    }

    #[tokio::test]
    async fn subscription_manager_stops_removed() {
        let (tx, _rx) = mpsc::unbounded_channel::<i32>();
        let mut manager = SubscriptionManager::new(tx);

        let id = SubscriptionId::of::<String>();
        let stream: BoxStream<'static, i32> = Box::pin(futures::stream::pending());
        let sub = Subscription::from_stream(id, stream);

        manager.reconcile(vec![sub]);
        assert_eq!(manager.active_count(), 1);

        // Reconcile with empty — should stop the subscription
        manager.reconcile(vec![]);
        assert_eq!(manager.active_count(), 0);
    }

    #[tokio::test]
    async fn subscription_manager_keeps_existing() {
        let (tx, _rx) = mpsc::unbounded_channel::<i32>();
        let mut manager = SubscriptionManager::new(tx);

        let id = SubscriptionId::of::<String>();
        let stream: BoxStream<'static, i32> = Box::pin(futures::stream::pending());
        let sub = Subscription::from_stream(id.clone(), stream);
        manager.reconcile(vec![sub]);

        // Reconcile with same ID — should keep it
        let stream2: BoxStream<'static, i32> = Box::pin(futures::stream::pending());
        let sub2 = Subscription::from_stream(id, stream2);
        manager.reconcile(vec![sub2]);
        assert_eq!(manager.active_count(), 1);
    }

    #[tokio::test]
    async fn subscription_manager_shutdown() {
        let (tx, _rx) = mpsc::unbounded_channel::<i32>();
        let mut manager = SubscriptionManager::new(tx);

        let id1 = SubscriptionId::new::<String>(1);
        let id2 = SubscriptionId::new::<String>(2);
        let stream1: BoxStream<'static, i32> = Box::pin(futures::stream::pending());
        let stream2: BoxStream<'static, i32> = Box::pin(futures::stream::pending());

        manager.reconcile(vec![
            Subscription::from_stream(id1, stream1),
            Subscription::from_stream(id2, stream2),
        ]);
        assert_eq!(manager.active_count(), 2);

        manager.shutdown();
        assert_eq!(manager.active_count(), 0);
    }
}
