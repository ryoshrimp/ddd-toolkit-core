use crate::{AggregateRoot, AsyncDomainEventPublisher};
use std::future::Future;
use std::pin::Pin;

struct EventRestorationGuard<'a, A: AggregateRoot> {
    aggregate: &'a mut A,
    snapshot: Option<Vec<A::Event>>,
}

impl<'a, A: AggregateRoot> EventRestorationGuard<'a, A> {
    fn new(aggregate: &'a mut A, snapshot: Vec<A::Event>) -> Self {
        Self {
            aggregate,
            snapshot: Some(snapshot),
        }
    }

    fn events(&self) -> impl Iterator<Item = &A::Event> {
        self.snapshot.as_deref().into_iter().flatten()
    }

    fn commit(&mut self) {
        self.snapshot.take();
    }
}

impl<A: AggregateRoot> Drop for EventRestorationGuard<'_, A> {
    fn drop(&mut self) {
        if let Some(snapshot) = self.snapshot.take() {
            for event in snapshot {
                self.aggregate.record_event(event);
            }
        }
    }
}

/// **English:** Asynchronously publishes the events currently held by `aggregate`, in the order
/// returned by [`AggregateRoot::pull_events`]. On success, the events are removed. If publishing
/// fails, all pulled events are recorded back into the aggregate before the publisher error is
/// returned; earlier successful publishes are not undone, so a retry can publish duplicates.
///
/// **日本語:** `aggregate` が保持するイベントを [`AggregateRoot::pull_events`] の返す順序で非同期に
/// 発行します。成功時はイベントを取り除きます。発行に失敗した場合、パブリッシャーのエラーを
/// 返す前に取り出した全イベントを集約へ記録し直します。それ以前に成功した発行は取り消されないため、
/// 再試行すると重複発行される可能性があります。
pub fn dispatch_events_async<'a, A, P>(
    aggregate: &'a mut A,
    publisher: &'a mut P,
) -> Pin<Box<dyn Future<Output = Result<(), P::Error>> + Send + 'a>>
where
    A: AggregateRoot + Send + 'a,
    A::Event: Send + Sync + 'a,
    P: AsyncDomainEventPublisher<Event = A::Event> + Send + 'a,
    P::Error: 'a,
{
    Box::pin(async move {
        let snapshot = aggregate.pull_events();
        let mut guard = EventRestorationGuard::new(aggregate, snapshot);

        for event in guard.events() {
            publisher.publish(event).await?
        }

        guard.commit();
        Ok(())
    })
}
