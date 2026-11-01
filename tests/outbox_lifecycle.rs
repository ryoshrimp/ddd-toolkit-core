use ddd_toolkit_core::{
    AggregateRoot, AppendError, AppendResult, CurrentRevision, DomainEvent, Entity,
    ExpectedRevision, LifecycleCompletionError, OutboxLifecycleCompletionError,
    OutboxLifecycleContext, OutboxLifecycleOperationError, RevisionConflict, StreamRevision,
    TransactionalOutbox, TransactionalOutboxCompletionError, TransactionalOutboxLifecycleContext,
};

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

#[derive(Debug, PartialEq, Eq)]
enum AdapterError {
    Storage,
}

struct Outbox {
    calls: usize,
    events: Vec<Event>,
    result: Result<AppendResult, AppendError<AdapterError>>,
}
impl TransactionalOutbox for Outbox {
    type Aggregate = Aggregate;
    type Error = AdapterError;

    fn save_and_enqueue(
        &mut self,
        _aggregate: &Aggregate,
        _expected: ExpectedRevision,
        events: Vec<Event>,
    ) -> Result<AppendResult, AppendError<Self::Error>> {
        self.calls += 1;
        self.events = events;
        match &self.result {
            Ok(result) => Ok(*result),
            Err(AppendError::EmptyBatch) => Err(AppendError::EmptyBatch),
            Err(AppendError::Conflict(conflict)) => Err(AppendError::Conflict(*conflict)),
            Err(AppendError::Adapter(_)) => Err(AppendError::Adapter(AdapterError::Storage)),
            Err(AppendError::RevisionExhausted { current }) => {
                Err(AppendError::RevisionExhausted { current: *current })
            }
        }
    }

    fn claim(&mut self) -> Result<Option<ddd_toolkit_core::OutboxClaim<Event>>, Self::Error> {
        Ok(None)
    }

    fn acknowledge(&mut self, _: &ddd_toolkit_core::OutboxMessageId) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[test]
fn successful_atomic_enqueue_consumes_events_and_completes() {
    let mut aggregate = Aggregate {
        id: 1,
        events: vec![Event(1), Event(2)],
    };
    let mut outbox = Outbox {
        calls: 0,
        events: Vec::new(),
        result: Ok(AppendResult {
            resulting_revision: StreamRevision::new(2),
        }),
    };
    let mut context = TransactionalOutboxLifecycleContext::new(
        &mut aggregate,
        &mut outbox,
        ExpectedRevision::NoStream(ddd_toolkit_core::NoStream),
    );
    assert_eq!(
        context.save_and_enqueue(),
        Ok(AppendResult {
            resulting_revision: StreamRevision::new(2),
        })
    );
    assert_eq!(context.complete(), Ok(()));
    assert_eq!(outbox.calls, 1);
    assert_eq!(outbox.events, vec![Event(1), Event(2)]);
    assert!(aggregate.events.is_empty());
}

#[test]
fn adapter_error_is_preserved_and_operation_cannot_retry() {
    let mut aggregate = Aggregate {
        id: 1,
        events: vec![Event(7)],
    };
    let mut outbox = Outbox {
        calls: 0,
        events: Vec::new(),
        result: Err(AppendError::Conflict(RevisionConflict {
            expected: ExpectedRevision::NoStream(ddd_toolkit_core::NoStream),
            current: CurrentRevision::NoStream(ddd_toolkit_core::NoStream),
        })),
    };
    let mut context = OutboxLifecycleContext::new(
        &mut aggregate,
        &mut outbox,
        ExpectedRevision::NoStream(ddd_toolkit_core::NoStream),
    );
    assert!(matches!(
        context.save_and_enqueue(),
        Err(OutboxLifecycleOperationError::Adapter(
            AppendError::Conflict(_)
        ))
    ));
    assert!(matches!(
        context.save_and_enqueue(),
        Err(OutboxLifecycleOperationError::AlreadyEnqueued)
    ));
    assert_eq!(
        context.complete(),
        Err(OutboxLifecycleCompletionError::EnqueueFailedConsumed)
    );
    assert_eq!(outbox.calls, 1);
    assert_eq!(aggregate.events, Vec::<Event>::new());
}

#[test]
fn completion_rejects_missing_enqueue() {
    let mut aggregate = Aggregate {
        id: 1,
        events: Vec::new(),
    };
    let mut outbox = Outbox {
        calls: 0,
        events: Vec::new(),
        result: Err(AppendError::EmptyBatch),
    };
    let context = TransactionalOutboxLifecycleContext::new(
        &mut aggregate,
        &mut outbox,
        ExpectedRevision::NoStream(ddd_toolkit_core::NoStream),
    );
    assert_eq!(
        context.complete(),
        Err(TransactionalOutboxCompletionError::MissingEnqueue)
    );
}

#[allow(dead_code)]
fn public_error_names_are_stable() {
    let _: Option<LifecycleCompletionError> = None;
}
