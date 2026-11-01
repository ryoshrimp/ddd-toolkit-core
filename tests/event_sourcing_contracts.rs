use ddd_toolkit_core::{
    AggregateReplay, AggregateRoot, AppendError, AppendResult, CurrentRevision, DomainEvent,
    Entity, EventStore, ExpectedRevision, LoadResult, NoStream, ReplayError, SnapshotReplayError,
    SnapshotUse, StreamHistory, StreamHistoryError, StreamRevision, replay_aggregate,
    replay_with_optional_snapshot,
};
use std::cell::Cell;
use std::collections::HashMap;

#[derive(Clone, Debug, Eq, PartialEq)]
struct Event(i32);
impl DomainEvent for Event {}

#[derive(Debug, Eq, PartialEq)]
struct Aggregate {
    id: u64,
    value: i32,
    pending: Vec<Event>,
}

thread_local! {
    static SNAPSHOT_RESTORATIONS: Cell<usize> = const { Cell::new(0) };
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
        self.pending.push(event);
    }
    fn pull_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.pending)
    }
}

#[derive(Debug, Eq, PartialEq)]
struct StoreError;
struct Store {
    streams: HashMap<u64, Vec<Event>>,
    fail_load: bool,
    fail_append: bool,
}
impl Store {
    fn new() -> Self {
        Self {
            streams: HashMap::new(),
            fail_load: false,
            fail_append: false,
        }
    }
}
impl EventStore for Store {
    type Aggregate = Aggregate;
    type Error = StoreError;
    fn load(&self, id: &u64) -> Result<LoadResult<Event>, StoreError> {
        if self.fail_load {
            return Err(StoreError);
        }
        match self.streams.get(id) {
            None => Ok(LoadResult::NoStream(NoStream)),
            Some(events) => Ok(LoadResult::Found(
                StreamHistory::try_new(
                    events.clone(),
                    StreamRevision::new(events.len() as u64 - 1),
                )
                .expect("fixture history is valid"),
            )),
        }
    }
    fn append(
        &mut self,
        id: &u64,
        expected: ExpectedRevision,
        events: Vec<Event>,
    ) -> Result<AppendResult, AppendError<StoreError>> {
        if events.is_empty() {
            return Err(AppendError::EmptyBatch);
        }
        let current = self
            .streams
            .get(id)
            .map(|e| StreamRevision::new(e.len() as u64 - 1));
        let current_carrier = current.map_or(
            CurrentRevision::NoStream(NoStream),
            CurrentRevision::Revision,
        );
        let matches = match (expected, current) {
            (ExpectedRevision::NoStream(_), None) => true,
            (ExpectedRevision::Revision(want), Some(have)) => want == have,
            _ => false,
        };
        if !matches {
            return Err(AppendError::Conflict(ddd_toolkit_core::RevisionConflict {
                expected,
                current: current_carrier,
            }));
        }
        if self.fail_append {
            return Err(AppendError::Adapter(StoreError));
        }
        let old = current.unwrap_or(StreamRevision::new(u64::MAX));
        let resulting = if current.is_none() {
            StreamRevision::new(events.len() as u64 - 1)
        } else {
            old.value()
                .checked_add(events.len() as u64)
                .map(StreamRevision::new)
                .ok_or(AppendError::RevisionExhausted { current: old })?
        };
        self.streams.entry(*id).or_default().extend(events);
        Ok(AppendResult {
            resulting_revision: resulting,
        })
    }
}

impl AggregateReplay for Aggregate {
    type InitializationError = &'static str;
    type ApplicationError = &'static str;
    type Snapshot = i32;
    type SnapshotValidationError = &'static str;
    type SnapshotStateError = &'static str;
    fn initial_state(id: &u64) -> Result<Self, Self::InitializationError> {
        if *id == 99 {
            Err("init")
        } else {
            Ok(Self {
                id: *id,
                value: 0,
                pending: Vec::new(),
            })
        }
    }
    fn apply_event(&mut self, event: &Event) -> Result<(), Self::ApplicationError> {
        if event.0 == 99 {
            Err("apply")
        } else {
            self.value += event.0;
            Ok(())
        }
    }
    fn classify_snapshot(
        snapshot: &ddd_toolkit_core::AggregateSnapshot<i32>,
        _revision: StreamRevision,
    ) -> Result<SnapshotUse, Self::SnapshotValidationError> {
        if *snapshot.state() == -1 {
            Err("invalid")
        } else if snapshot.revision().value() == 77 {
            Ok(SnapshotUse::Stale)
        } else if snapshot.revision().value() == 88 {
            Ok(SnapshotUse::Inapplicable)
        } else {
            Ok(SnapshotUse::Applicable)
        }
    }
    fn from_snapshot(
        id: &u64,
        snapshot: ddd_toolkit_core::AggregateSnapshot<i32>,
    ) -> Result<Self, Self::SnapshotStateError> {
        SNAPSHOT_RESTORATIONS.with(|calls| calls.set(calls.get() + 1));
        let (value, _) = snapshot.into_parts();
        if value == -2 {
            Err("state")
        } else {
            Ok(Self {
                id: *id,
                value,
                pending: Vec::new(),
            })
        }
    }
}

fn history(events: Vec<Event>) -> StreamHistory<Event> {
    let revision = StreamRevision::new(events.len() as u64 - 1);
    StreamHistory::try_new(events, revision).unwrap()
}

#[test]
fn stream_history_rejects_empty_and_mismatched_revision() {
    assert_eq!(
        StreamHistory::<Event>::try_new(Vec::new(), StreamRevision::new(0)),
        Err(StreamHistoryError::Empty)
    );
    assert_eq!(
        StreamHistory::try_new(vec![Event(1)], StreamRevision::new(1)),
        Err(StreamHistoryError::RevisionMismatch {
            expected: StreamRevision::new(0),
            actual: StreamRevision::new(1)
        })
    );
    assert_eq!(StreamRevision::new(u64::MAX).next(), None);
}

#[test]
fn append_load_and_conflict_are_typed_atomic_and_zero_based() {
    let mut store = Store::new();
    assert_eq!(store.load(&1), Ok(LoadResult::NoStream(NoStream)));
    assert_eq!(
        store.append(&1, ExpectedRevision::NoStream(NoStream), Vec::new()),
        Err(AppendError::EmptyBatch)
    );
    assert_eq!(
        store.append(
            &1,
            ExpectedRevision::NoStream(NoStream),
            vec![Event(1), Event(2)]
        ),
        Ok(AppendResult {
            resulting_revision: StreamRevision::new(1)
        })
    );
    let before = store.load(&1).unwrap();
    assert_eq!(
        store.append(
            &1,
            ExpectedRevision::Revision(StreamRevision::new(1)),
            Vec::new()
        ),
        Err(AppendError::EmptyBatch)
    );
    assert_eq!(store.load(&1).unwrap(), before);
    assert_eq!(
        store.append(&1, ExpectedRevision::NoStream(NoStream), vec![Event(3)]),
        Err(AppendError::Conflict(ddd_toolkit_core::RevisionConflict {
            expected: ExpectedRevision::NoStream(NoStream),
            current: CurrentRevision::Revision(StreamRevision::new(1))
        }))
    );
    assert_eq!(store.load(&1).unwrap(), before);
    assert_eq!(
        store.append(
            &1,
            ExpectedRevision::Revision(StreamRevision::new(0)),
            vec![Event(3), Event(4)]
        ),
        Err(AppendError::Conflict(ddd_toolkit_core::RevisionConflict {
            expected: ExpectedRevision::Revision(StreamRevision::new(0)),
            current: CurrentRevision::Revision(StreamRevision::new(1))
        }))
    );
    assert_eq!(store.load(&1).unwrap(), before);
    assert_eq!(
        store.append(
            &1,
            ExpectedRevision::Revision(StreamRevision::new(1)),
            vec![Event(3)]
        ),
        Ok(AppendResult {
            resulting_revision: StreamRevision::new(2)
        })
    );
    assert_eq!(
        store.load(&1).unwrap(),
        LoadResult::Found(history(vec![Event(1), Event(2), Event(3)]))
    );

    assert_eq!(
        store.append(&2, ExpectedRevision::NoStream(NoStream), vec![Event(5)]),
        Ok(AppendResult {
            resulting_revision: StreamRevision::new(0)
        })
    );
    assert_eq!(
        store.append(
            &2,
            ExpectedRevision::Revision(StreamRevision::new(0)),
            vec![Event(6)]
        ),
        Ok(AppendResult {
            resulting_revision: StreamRevision::new(1)
        })
    );
    assert_eq!(
        store.load(&2).unwrap(),
        LoadResult::Found(history(vec![Event(5), Event(6)]))
    );
}

#[test]
fn event_store_append_uses_owned_batch_without_consuming_pending_events() {
    let mut aggregate = Aggregate {
        id: 1,
        value: 0,
        pending: Vec::new(),
    };
    aggregate.record_event(Event(10));
    aggregate.record_event(Event(11));
    let pending_before = aggregate.pending.clone();
    let mut store = Store::new();

    let batch = vec![Event(1), Event(2)];
    assert_eq!(
        store.append(&aggregate.id, ExpectedRevision::NoStream(NoStream), batch),
        Ok(AppendResult {
            resulting_revision: StreamRevision::new(1)
        })
    );
    assert_eq!(aggregate.pending, pending_before);
    assert_eq!(aggregate.pull_events(), pending_before);
    assert_eq!(
        store.load(&aggregate.id),
        Ok(LoadResult::Found(history(vec![Event(1), Event(2)])))
    );
}

#[test]
fn replay_is_ordered_additive_and_keeps_pending_queue_empty() {
    let result = replay_aggregate::<Aggregate>(&1, &history(vec![Event(2), Event(3)])).unwrap();
    assert_eq!(result.revision, StreamRevision::new(1));
    assert_eq!(result.aggregate.value, 5);
    assert!(result.aggregate.pending.is_empty());
    assert_eq!(
        replay_aggregate::<Aggregate>(&99, &history(vec![Event(1)])).unwrap_err(),
        ReplayError::Initialization("init")
    );
    assert_eq!(
        replay_aggregate::<Aggregate>(&1, &history(vec![Event(1), Event(99)])).unwrap_err(),
        ReplayError::Application("apply")
    );
}

#[test]
fn optional_snapshot_replays_suffix_and_distinguishes_failures() {
    let snapshot = ddd_toolkit_core::AggregateSnapshot::new(10, StreamRevision::new(0));
    let result = replay_with_optional_snapshot::<Aggregate>(
        &1,
        &history(vec![Event(1), Event(2)]),
        Some(snapshot),
    )
    .unwrap();
    assert_eq!(result.aggregate.value, 12);
    assert_eq!(result.revision, StreamRevision::new(1));
    let full =
        replay_with_optional_snapshot::<Aggregate>(&1, &history(vec![Event(1), Event(2)]), None)
            .unwrap();
    assert_eq!(full.aggregate.value, 3);

    SNAPSHOT_RESTORATIONS.with(|calls| calls.set(0));
    let final_snapshot = replay_with_optional_snapshot::<Aggregate>(
        &1,
        &history(vec![Event(1), Event(2)]),
        Some(ddd_toolkit_core::AggregateSnapshot::new(
            10,
            StreamRevision::new(1),
        )),
    )
    .unwrap();
    assert_eq!(final_snapshot.aggregate.value, 10);
    assert_eq!(final_snapshot.revision, StreamRevision::new(1));
    SNAPSHOT_RESTORATIONS.with(|calls| assert_eq!(calls.get(), 1));

    SNAPSHOT_RESTORATIONS.with(|calls| calls.set(0));
    assert_eq!(
        replay_with_optional_snapshot::<Aggregate>(
            &1,
            &history(vec![Event(1)]),
            Some(ddd_toolkit_core::AggregateSnapshot::new(
                42,
                StreamRevision::new(1)
            ))
        )
        .unwrap_err(),
        SnapshotReplayError::SnapshotRevision {
            snapshot_revision: StreamRevision::new(1),
            history_revision: StreamRevision::new(0)
        }
    );
    SNAPSHOT_RESTORATIONS.with(|calls| assert_eq!(calls.get(), 0));
    assert_eq!(
        replay_with_optional_snapshot::<Aggregate>(
            &1,
            &history(vec![Event(1)]),
            Some(ddd_toolkit_core::AggregateSnapshot::new(
                -1,
                StreamRevision::new(0)
            ))
        )
        .unwrap_err(),
        SnapshotReplayError::SnapshotValidation("invalid")
    );
    assert_eq!(
        replay_with_optional_snapshot::<Aggregate>(
            &1,
            &history(vec![Event(1)]),
            Some(ddd_toolkit_core::AggregateSnapshot::new(
                -2,
                StreamRevision::new(0)
            ))
        )
        .unwrap_err(),
        SnapshotReplayError::SnapshotState("state")
    );
    assert_eq!(
        replay_with_optional_snapshot::<Aggregate>(
            &1,
            &history(vec![Event(1)]),
            Some(ddd_toolkit_core::AggregateSnapshot::new(
                77,
                StreamRevision::new(77)
            ))
        )
        .unwrap()
        .aggregate
        .value,
        1
    );
    assert_eq!(
        replay_with_optional_snapshot::<Aggregate>(
            &1,
            &history(vec![Event(1)]),
            Some(ddd_toolkit_core::AggregateSnapshot::new(
                88,
                StreamRevision::new(88)
            ))
        )
        .unwrap()
        .aggregate
        .value,
        1
    );
}

#[test]
fn adapter_failures_are_not_normalized() {
    let mut store = Store::new();
    store.fail_load = true;
    assert_eq!(store.load(&1), Err(StoreError));
    store.fail_load = false;
    store.fail_append = true;
    assert_eq!(
        store.append(&1, ExpectedRevision::NoStream(NoStream), vec![Event(1)]),
        Err(AppendError::Adapter(StoreError))
    );
    assert_eq!(store.load(&1), Ok(LoadResult::NoStream(NoStream)));
}
