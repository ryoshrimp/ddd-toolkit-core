use ddd_toolkit_core::{
    AggregateReplay, AggregateRoot, AggregateSnapshot, DomainEvent, Entity, RehydratedAggregate,
    SnapshotLoadError, SnapshotReplayError, SnapshotSaveError, SnapshotStore, StreamHistory,
    StreamRevision, replay_with_optional_snapshot,
};
use std::cell::Cell;

#[derive(Debug, Eq, PartialEq)]
struct Event(u32);
impl DomainEvent for Event {}
#[derive(Debug, Eq, PartialEq)]
struct Aggregate {
    id: u64,
    value: u32,
    pending: Vec<Event>,
}

thread_local! {
    static INITIALIZATIONS: Cell<usize> = const { Cell::new(0) };
}
impl Entity for Aggregate {
    type Id = u64;
    fn id(&self) -> &u64 {
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
impl AggregateReplay for Aggregate {
    type InitializationError = ();
    type ApplicationError = ();
    type Snapshot = u32;
    type SnapshotValidationError = ();
    type SnapshotStateError = ();
    fn initial_state(id: &u64) -> Result<Self, ()> {
        INITIALIZATIONS.with(|calls| calls.set(calls.get() + 1));
        Ok(Self {
            id: *id,
            value: 0,
            pending: Vec::new(),
        })
    }
    fn apply_event(&mut self, event: &Event) -> Result<(), ()> {
        self.value += event.0;
        Ok(())
    }
    fn classify_snapshot(
        _: &AggregateSnapshot<u32>,
        _: StreamRevision,
    ) -> Result<ddd_toolkit_core::SnapshotUse, ()> {
        Ok(ddd_toolkit_core::SnapshotUse::Applicable)
    }
    fn from_snapshot(id: &u64, snapshot: AggregateSnapshot<u32>) -> Result<Self, ()> {
        let (value, _) = snapshot.into_parts();
        Ok(Self {
            id: *id,
            value,
            pending: Vec::new(),
        })
    }
}

#[derive(Debug, Eq, PartialEq)]
struct SnapshotAdapter {
    value: Option<AggregateSnapshot<u32>>,
    load_error: bool,
    save_error: bool,
    save_attempts: usize,
}
impl SnapshotStore for SnapshotAdapter {
    type Aggregate = Aggregate;
    type State = u32;
    type Error = &'static str;
    fn load(
        &self,
        _: &u64,
    ) -> Result<Option<AggregateSnapshot<u32>>, SnapshotLoadError<Self::Error>> {
        if self.load_error {
            Err(SnapshotLoadError::Adapter("load"))
        } else {
            Ok(self
                .value
                .as_ref()
                .map(|s| AggregateSnapshot::new(*s.state(), s.revision())))
        }
    }
    fn save(
        &mut self,
        _: &u64,
        snapshot: AggregateSnapshot<u32>,
    ) -> Result<(), SnapshotSaveError<Self::Error>> {
        self.save_attempts += 1;
        if self.save_error {
            Err(SnapshotSaveError::Adapter("save"))
        } else {
            self.value = Some(snapshot);
            Ok(())
        }
    }
}

fn history() -> StreamHistory<Event> {
    StreamHistory::try_new(vec![Event(2), Event(3)], StreamRevision::new(1)).unwrap()
}

#[test]
fn snapshot_store_keeps_absence_and_adapter_errors_distinct() {
    let mut adapter = SnapshotAdapter {
        value: None,
        load_error: false,
        save_error: false,
        save_attempts: 0,
    };
    assert_eq!(adapter.load(&1), Ok(None));
    adapter.load_error = true;
    assert_eq!(adapter.load(&1), Err(SnapshotLoadError::Adapter("load")));
    adapter.load_error = false;
    adapter
        .save(&1, AggregateSnapshot::new(5, StreamRevision::new(0)))
        .unwrap();
    assert_eq!(
        adapter.load(&1).unwrap().unwrap().into_parts(),
        (5, StreamRevision::new(0))
    );
    adapter.save_error = true;
    assert_eq!(
        adapter.save(&1, AggregateSnapshot::new(6, StreamRevision::new(1))),
        Err(SnapshotSaveError::Adapter("save"))
    );
}

#[test]
fn snapshot_load_results_reach_reconstruction_boundary() {
    INITIALIZATIONS.with(|calls| calls.set(0));
    let mut adapter = SnapshotAdapter {
        value: None,
        load_error: false,
        save_error: false,
        save_attempts: 0,
    };
    let loaded = adapter.load(&1).unwrap();
    let result = replay_with_optional_snapshot::<Aggregate>(&1, &history(), loaded).unwrap();
    assert_eq!(result.aggregate.value, 5);
    INITIALIZATIONS.with(|calls| assert_eq!(calls.get(), 1));
    adapter
        .save(
            &1,
            AggregateSnapshot::new(result.aggregate.value, result.revision),
        )
        .unwrap();
    let saved = adapter.value.as_ref().unwrap();
    assert_eq!(*saved.state(), 5);
    assert_eq!(saved.revision(), StreamRevision::new(1));

    adapter.load_error = true;
    assert_eq!(adapter.load(&1), Err(SnapshotLoadError::Adapter("load")));
    INITIALIZATIONS.with(|calls| assert_eq!(calls.get(), 1));
}

#[derive(Debug, Eq, PartialEq)]
enum ReconstructionError {
    SnapshotLoad(SnapshotLoadError<&'static str>),
    Replay(SnapshotReplayError<(), (), (), ()>),
}

fn reconstruct_from_snapshot_store(
    adapter: &SnapshotAdapter,
    id: &u64,
    history: &StreamHistory<Event>,
) -> Result<RehydratedAggregate<Aggregate>, ReconstructionError> {
    let snapshot = match adapter.load(id) {
        Ok(snapshot) => snapshot,
        Err(error) => return Err(ReconstructionError::SnapshotLoad(error)),
    };
    replay_with_optional_snapshot::<Aggregate>(id, history, snapshot)
        .map_err(ReconstructionError::Replay)
}

#[test]
fn snapshot_load_adapter_failure_stops_at_reconstruction_boundary() {
    INITIALIZATIONS.with(|calls| calls.set(0));
    let adapter = SnapshotAdapter {
        value: None,
        load_error: true,
        save_error: false,
        save_attempts: 0,
    };

    assert_eq!(
        reconstruct_from_snapshot_store(&adapter, &1, &history()),
        Err(ReconstructionError::SnapshotLoad(
            SnapshotLoadError::Adapter("load",)
        ))
    );
    INITIALIZATIONS.with(|calls| assert_eq!(calls.get(), 0));
}

#[test]
fn snapshot_save_failure_after_reconstruction_is_observable_without_queue_mutation() {
    INITIALIZATIONS.with(|calls| calls.set(0));
    let mut adapter = SnapshotAdapter {
        value: None,
        load_error: false,
        save_error: true,
        save_attempts: 0,
    };
    let mut source = Aggregate {
        id: 1,
        value: 0,
        pending: Vec::new(),
    };
    source.record_event(Event(99));

    let reconstructed = reconstruct_from_snapshot_store(&adapter, &1, &history()).unwrap();
    assert_eq!(reconstructed.aggregate.value, 5);
    INITIALIZATIONS.with(|calls| assert_eq!(calls.get(), 1));
    assert_eq!(source.pending, vec![Event(99)]);

    assert_eq!(
        adapter.save(
            &1,
            AggregateSnapshot::new(reconstructed.aggregate.value, reconstructed.revision),
        ),
        Err(SnapshotSaveError::Adapter("save"))
    );
    assert_eq!(adapter.save_attempts, 1);
    assert!(adapter.value.is_none());
    assert_eq!(source.pending, vec![Event(99)]);
}
