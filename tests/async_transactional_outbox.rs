use ddd_toolkit_core::{
    AggregateRoot, AppendError, AppendResult, AsyncTransactionalOutbox,
    AsyncTransactionalOutboxCompletionError, AsyncTransactionalOutboxLifecycleContext,
    AsyncTransactionalOutboxOperationError, CurrentRevision, DomainEvent, Entity, ExpectedRevision,
    OutboxClaim, OutboxMessageId, ProjectionCheckpoint, RevisionConflict, StreamRevision,
};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

#[derive(Clone, Debug, PartialEq, Eq)]
struct Event(u64);
impl DomainEvent for Event {}

#[derive(Debug, PartialEq, Eq)]
struct Aggregate {
    id: u64,
    events: Vec<Event>,
}
impl Entity for Aggregate {
    type Id = u64;
    fn id(&self) -> &Self::Id {
        &self.id
    }
}
impl AggregateRoot for Aggregate {
    type Event = Event;
    fn record_event(&mut self, event: Self::Event) {
        self.events.push(event);
    }
    fn pull_events(&mut self) -> Vec<Self::Event> {
        std::mem::take(&mut self.events)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Success,
    Failure,
    Pending,
}

struct Outbox {
    calls: usize,
    events: Vec<Event>,
    mode: Mode,
}
impl AsyncTransactionalOutbox for Outbox {
    type Aggregate = Aggregate;
    type Error = &'static str;

    fn save_and_enqueue<'a>(
        &'a mut self,
        _: &'a Aggregate,
        _: ExpectedRevision,
        events: Vec<Event>,
    ) -> Pin<Box<dyn Future<Output = Result<AppendResult, AppendError<Self::Error>>> + Send + 'a>>
    {
        self.calls += 1;
        self.events = events;
        match self.mode {
            Mode::Success => Box::pin(async {
                Ok(AppendResult {
                    resulting_revision: StreamRevision::new(1),
                })
            }),
            Mode::Failure => Box::pin(async {
                Err(AppendError::Conflict(RevisionConflict {
                    expected: ExpectedRevision::NoStream(ddd_toolkit_core::NoStream),
                    current: CurrentRevision::NoStream(ddd_toolkit_core::NoStream),
                }))
            }),
            Mode::Pending => Box::pin(std::future::pending()),
        }
    }

    fn claim<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn Future<Output = Result<Option<OutboxClaim<Event>>, Self::Error>> + Send + 'a>>
    {
        Box::pin(async {
            Ok(Some(OutboxClaim::new(
                OutboxMessageId::new("m-1"),
                Event(9),
            )))
        })
    }

    fn acknowledge<'a>(
        &'a mut self,
        _: &'a OutboxMessageId,
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'a>> {
        Box::pin(async { Ok(()) })
    }
}

fn poll_ready<T>(mut future: Pin<Box<dyn Future<Output = T> + Send + '_>>) -> T {
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    match future.as_mut().poll(&mut context) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("fixture future did not complete in one poll"),
    }
}

fn expected() -> ExpectedRevision {
    ExpectedRevision::NoStream(ddd_toolkit_core::NoStream)
}

#[test]
fn async_outbox_success_preserves_owned_fifo_batch_and_completion() {
    let mut aggregate = Aggregate {
        id: 1,
        events: vec![Event(1), Event(2)],
    };
    let mut outbox = Outbox {
        calls: 0,
        events: Vec::new(),
        mode: Mode::Success,
    };
    let mut context =
        AsyncTransactionalOutboxLifecycleContext::new(&mut aggregate, &mut outbox, expected());

    assert_eq!(
        poll_ready(context.save_and_enqueue()),
        Ok(AppendResult {
            resulting_revision: StreamRevision::new(1)
        })
    );
    assert_eq!(context.complete(), Ok(()));
    assert_eq!(outbox.calls, 1);
    assert_eq!(outbox.events, vec![Event(1), Event(2)]);
    assert!(aggregate.events.is_empty());
}

#[test]
fn async_outbox_error_is_preserved_and_cannot_retry() {
    let mut aggregate = Aggregate {
        id: 1,
        events: vec![Event(7)],
    };
    let mut outbox = Outbox {
        calls: 0,
        events: Vec::new(),
        mode: Mode::Failure,
    };
    let mut context =
        AsyncTransactionalOutboxLifecycleContext::new(&mut aggregate, &mut outbox, expected());

    assert!(matches!(
        poll_ready(context.save_and_enqueue()),
        Err(AsyncTransactionalOutboxOperationError::Adapter(
            AppendError::Conflict(_)
        ))
    ));
    assert_eq!(
        poll_ready(context.save_and_enqueue()),
        Err(AsyncTransactionalOutboxOperationError::AlreadyEnqueued)
    );
    assert_eq!(
        context.complete(),
        Err(AsyncTransactionalOutboxCompletionError::EnqueueFailedConsumed)
    );
    assert_eq!(outbox.calls, 1);
    assert!(aggregate.events.is_empty());
}

#[test]
fn dropping_async_outbox_future_consumes_operation_without_retry() {
    let mut aggregate = Aggregate {
        id: 1,
        events: vec![Event(3)],
    };
    let mut outbox = Outbox {
        calls: 0,
        events: Vec::new(),
        mode: Mode::Pending,
    };
    let mut context =
        AsyncTransactionalOutboxLifecycleContext::new(&mut aggregate, &mut outbox, expected());
    let mut future = context.save_and_enqueue();
    let waker = Waker::noop();
    let mut task_context = Context::from_waker(waker);
    assert!(future.as_mut().poll(&mut task_context).is_pending());
    drop(future);

    assert_eq!(
        poll_ready(context.save_and_enqueue()),
        Err(AsyncTransactionalOutboxOperationError::AlreadyEnqueued)
    );
    assert_eq!(
        context.complete(),
        Err(AsyncTransactionalOutboxCompletionError::EnqueueFailedConsumed)
    );
    assert_eq!(outbox.calls, 1);
    assert_eq!(outbox.events, vec![Event(3)]);
    assert!(aggregate.events.is_empty());
}

#[test]
fn async_outbox_claim_and_acknowledge_are_runtime_neutral() {
    let mut outbox = Outbox {
        calls: 0,
        events: Vec::new(),
        mode: Mode::Success,
    };
    let claim = poll_ready(outbox.claim()).unwrap().unwrap();
    let (message_id, event) = claim.into_parts();
    assert_eq!(message_id.as_str(), "m-1");
    assert_eq!(event, Event(9));
    assert_eq!(poll_ready(outbox.acknowledge(&message_id)), Ok(()));
}

#[allow(dead_code)]
fn compile_contracts_include_shared_checkpoint_types() {
    let checkpoint = ProjectionCheckpoint::new(7u64);
    assert_eq!(checkpoint.cursor(), &7);
}
