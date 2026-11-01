use crate::{AggregateRoot, DomainEventPublisher};

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

/// **English:** Publishes the events currently held by `aggregate` through `publisher`.
///
/// Events are passed to [`DomainEventPublisher::publish`] in the order returned by
/// [`AggregateRoot::pull_events`]. If a publish operation returns an error, all events pulled at
/// the start of this call are recorded back into the aggregate before that error is returned.
/// This restoration does not undo publishes that already succeeded. If every publish succeeds,
/// the events are removed from the aggregate and the function returns `Ok(())`.
///
/// The function may panic if the aggregate's [`AggregateRoot::pull_events`] or
/// [`AggregateRoot::record_event`] implementation panics. It does not impose additional
/// synchronization; the aggregate and publisher must be mutably borrowed by the caller for the
/// duration of the dispatch.
///
/// **日本語:** `aggregate` が現在保持しているイベントを `publisher` 経由で発行します。
///
/// イベントは [`AggregateRoot::pull_events`] が返した順序で [`DomainEventPublisher::publish`]
/// に渡されます。発行処理がエラーを返すと、この呼び出しの開始時に取り出したすべての
/// イベントを集約へ記録し直してから、そのエラーを返します。すでに成功した発行処理は
/// 取り消されません。すべての発行に成功した場合、イベントは集約から削除され、`Ok(())`
/// を返します。
///
/// 集約の [`AggregateRoot::pull_events`] または [`AggregateRoot::record_event`] の実装が
/// パニックすると、この関数もパニックする可能性があります。この関数は追加の同期を
/// 行わないため、呼び出し側はディスパッチ中、集約とパブリッシャーを可変借用します。
pub fn dispatch_events<A, P>(aggregate: &mut A, publisher: &mut P) -> Result<(), P::Error>
where
    A: AggregateRoot,
    P: DomainEventPublisher<Event = A::Event>,
{
    let snapshot = aggregate.pull_events();
    let mut guard = EventRestorationGuard::new(aggregate, snapshot);

    for event in guard.events() {
        publisher.publish(event)?;
    }

    guard.commit();
    Ok(())
}
