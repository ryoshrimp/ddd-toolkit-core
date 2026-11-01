use ddd_toolkit_core::{
    AggregateRoot, AggregateSnapshot, DomainEvent, Entity, SnapshotLoadError, SnapshotSaveError,
    SnapshotStore,
};

struct Event;
impl DomainEvent for Event {}
struct Aggregate {
    id: u64,
}
impl Entity for Aggregate {
    type Id = u64;
    fn id(&self) -> &u64 {
        &self.id
    }
}
impl AggregateRoot for Aggregate {
    type Event = Event;
    fn record_event(&mut self, _: Event) {}
    fn pull_events(&mut self) -> Vec<Event> {
        Vec::new()
    }
}

struct Store;
impl SnapshotStore for Store {
    type Aggregate = Aggregate;
    type State = u32;
    type Error = ();

    fn load(
        &self,
        _: &u64,
    ) -> Result<Option<AggregateSnapshot<u32>>, SnapshotLoadError<()>> {
        let result: Result<(), ()> = Err(());
        let snapshot: Option<AggregateSnapshot<u32>> = result;
        Ok(snapshot)
    }

    fn save(
        &mut self,
        _: &u64,
        _: AggregateSnapshot<u32>,
    ) -> Result<(), SnapshotSaveError<()>> {
        Ok(())
    }
}

fn main() {}
