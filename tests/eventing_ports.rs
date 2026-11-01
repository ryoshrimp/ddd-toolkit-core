use ddd_toolkit_core::{
    AggregateRoot, AggregateSnapshot, AppendError, AppendResult, AsyncEventConsumer,
    AsyncEventStore, AsyncProjection, AsyncSnapshotStore, CurrentRevision, DeliveryMetadata,
    DomainEvent, DomainEventDeliveryMetadata, Entity, EventConsumer, EventStore, ExpectedRevision,
    LoadResult, NoStream, OutboxClaim, OutboxMessageId, Projection, SnapshotLoadError,
    SnapshotSaveError, StreamHistory, StreamRevision, TransactionalOutbox,
};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

#[derive(Clone, Debug, Eq, PartialEq)]
struct Event(u32);
impl DomainEvent for Event {}

#[derive(Debug)]
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
    fn record_event(&mut self, event: Event) {
        self.events.push(event);
    }
    fn pull_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }
}

fn block_on<F: Future>(mut future: F) -> F::Output {
    const VTABLE: RawWakerVTable = RawWakerVTable::new(
        |_| RawWaker::new(std::ptr::null(), &VTABLE),
        |_| {},
        |_| {},
        |_| {},
    );
    let waker = unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) };
    let mut context = Context::from_waker(&waker);
    // Test futures are immediately ready; a production executor owns polling policy.
    loop {
        match unsafe { Pin::new_unchecked(&mut future) }.poll(&mut context) {
            Poll::Ready(value) => return value,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

#[derive(Default)]
struct AsyncStore {
    history: Option<StreamHistory<Event>>,
    snapshot: Option<AggregateSnapshot<u32>>,
}
impl AsyncEventStore for AsyncStore {
    type Aggregate = Aggregate;
    type Error = &'static str;
    fn load<'a>(
        &'a self,
        _: &'a u64,
    ) -> Pin<Box<dyn Future<Output = Result<LoadResult<Event>, Self::Error>> + Send + 'a>> {
        let result = match &self.history {
            Some(history) => LoadResult::Found(
                StreamHistory::try_new(history.events().to_vec(), history.revision()).unwrap(),
            ),
            None => LoadResult::NoStream(NoStream),
        };
        Box::pin(async move { Ok(result) })
    }
    fn append<'a>(
        &'a mut self,
        _: &'a u64,
        expected: ExpectedRevision,
        events: Vec<Event>,
    ) -> Pin<Box<dyn Future<Output = Result<AppendResult, AppendError<Self::Error>>> + Send + 'a>>
    {
        Box::pin(async move {
            if events.is_empty() {
                return Err(AppendError::EmptyBatch);
            }
            if !matches!(expected, ExpectedRevision::NoStream(NoStream)) {
                return Err(AppendError::Conflict(ddd_toolkit_core::RevisionConflict {
                    expected,
                    current: CurrentRevision::NoStream(NoStream),
                }));
            }
            let revision = StreamRevision::new(events.len() as u64 - 1);
            self.history = Some(StreamHistory::try_new(events, revision).unwrap());
            Ok(AppendResult {
                resulting_revision: revision,
            })
        })
    }
}
impl AsyncSnapshotStore for AsyncStore {
    type Aggregate = Aggregate;
    type State = u32;
    type Error = &'static str;
    fn load<'a>(
        &'a self,
        _: &'a u64,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<Option<AggregateSnapshot<u32>>, SnapshotLoadError<Self::Error>>,
                > + Send
                + 'a,
        >,
    > {
        let snapshot = self
            .snapshot
            .as_ref()
            .map(|s| AggregateSnapshot::new(*s.state(), s.revision()));
        Box::pin(async move { Ok(snapshot) })
    }
    fn save<'a>(
        &'a mut self,
        _: &'a u64,
        snapshot: AggregateSnapshot<u32>,
    ) -> Pin<Box<dyn Future<Output = Result<(), SnapshotSaveError<Self::Error>>> + Send + 'a>> {
        Box::pin(async move {
            self.snapshot = Some(snapshot);
            Ok(())
        })
    }
}

#[test]
fn async_event_and_snapshot_ports_preserve_typed_contracts() {
    let mut store = AsyncStore::default();
    assert_eq!(
        block_on(AsyncEventStore::load(&store, &1)).unwrap(),
        LoadResult::NoStream(NoStream)
    );
    assert_eq!(
        block_on(store.append(&1, ExpectedRevision::NoStream(NoStream), vec![Event(1)]))
            .unwrap()
            .resulting_revision,
        StreamRevision::new(0)
    );
    assert!(matches!(
        block_on(AsyncEventStore::load(&store, &1)).unwrap(),
        LoadResult::Found(_)
    ));
    block_on(AsyncSnapshotStore::save(
        &mut store,
        &1,
        AggregateSnapshot::new(7, StreamRevision::new(0)),
    ))
    .unwrap();
    assert!(
        block_on(AsyncSnapshotStore::load(&store, &1))
            .unwrap()
            .is_some()
    );
}

#[test]
fn delivery_metadata_is_opt_in_and_consumer_projection_are_distinct() {
    struct WithMetadata(DeliveryMetadata);
    impl DomainEvent for WithMetadata {}
    impl DomainEventDeliveryMetadata for WithMetadata {
        fn delivery_metadata(&self) -> Option<&DeliveryMetadata> {
            Some(&self.0)
        }
    }
    let event = WithMetadata(DeliveryMetadata::new("e-1").with_correlation_id("c-1"));
    assert_eq!(event.delivery_metadata().unwrap().event_id(), "e-1");
    assert_eq!(
        event.delivery_metadata().unwrap().correlation_id(),
        Some("c-1")
    );

    struct Handler(usize);
    impl EventConsumer for Handler {
        type Event = Event;
        type Error = ();
        fn consume(&mut self, _: &Event) -> Result<(), ()> {
            self.0 += 1;
            Ok(())
        }
    }
    impl Projection for Handler {
        type Event = Event;
        type Error = ();
        fn project(&mut self, _: &Event) -> Result<(), ()> {
            self.0 += 10;
            Ok(())
        }
    }
    impl AsyncEventConsumer for Handler {
        type Event = Event;
        type Error = ();
        fn consume<'a>(
            &'a mut self,
            _: &'a Event,
        ) -> Pin<Box<dyn Future<Output = Result<(), ()>> + Send + 'a>> {
            Box::pin(async move {
                self.0 += 1;
                Ok(())
            })
        }
    }
    impl AsyncProjection for Handler {
        type Event = Event;
        type Error = ();
        fn project<'a>(
            &'a mut self,
            _: &'a Event,
        ) -> Pin<Box<dyn Future<Output = Result<(), ()>> + Send + 'a>> {
            Box::pin(async move {
                self.0 += 10;
                Ok(())
            })
        }
    }
    let mut handler = Handler(0);
    EventConsumer::consume(&mut handler, &Event(1)).unwrap();
    Projection::project(&mut handler, &Event(1)).unwrap();
    block_on(AsyncEventConsumer::consume(&mut handler, &Event(1))).unwrap();
    block_on(AsyncProjection::project(&mut handler, &Event(1))).unwrap();
    assert_eq!(handler.0, 22);
}

#[test]
fn transactional_outbox_exposes_owned_claim_and_ack_boundary() {
    struct Adapter {
        claimed: bool,
        acked: Option<OutboxMessageId>,
    }
    impl TransactionalOutbox for Adapter {
        type Aggregate = Aggregate;
        type Error = ();
        fn save_and_enqueue(
            &mut self,
            _: &Aggregate,
            _: ExpectedRevision,
            events: Vec<Event>,
        ) -> Result<AppendResult, AppendError<()>> {
            if events.is_empty() {
                return Err(AppendError::EmptyBatch);
            }
            Ok(AppendResult {
                resulting_revision: StreamRevision::new(events.len() as u64 - 1),
            })
        }
        fn claim(&mut self) -> Result<Option<OutboxClaim<Event>>, ()> {
            if self.claimed {
                Ok(None)
            } else {
                self.claimed = true;
                Ok(Some(OutboxClaim::new(
                    OutboxMessageId::new("m-1"),
                    Event(9),
                )))
            }
        }
        fn acknowledge(&mut self, id: &OutboxMessageId) -> Result<(), ()> {
            self.acked = Some(id.clone());
            Ok(())
        }
    }
    let mut adapter = Adapter {
        claimed: false,
        acked: None,
    };
    let claim = adapter.claim().unwrap().unwrap();
    assert_eq!(claim.event(), &Event(9));
    adapter.acknowledge(claim.message_id()).unwrap();
    assert_eq!(adapter.acked.unwrap().as_str(), "m-1");
}

#[test]
fn synchronous_event_store_remains_available() {
    fn assert_port<T: EventStore<Aggregate = Aggregate>>() {}
    struct Store;
    impl EventStore for Store {
        type Aggregate = Aggregate;
        type Error = ();
        fn load(&self, _: &u64) -> Result<LoadResult<Event>, ()> {
            Ok(LoadResult::NoStream(NoStream))
        }
        fn append(
            &mut self,
            _: &u64,
            _: ExpectedRevision,
            _: Vec<Event>,
        ) -> Result<AppendResult, AppendError<()>> {
            unimplemented!()
        }
    }
    assert_port::<Store>();
}
