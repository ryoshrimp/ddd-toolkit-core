use ddd_toolkit_core::{AggregateReplay, AggregateRoot, Entity, StreamRevision, AggregateSnapshot, SnapshotUse};
struct NotEvent;
struct Aggregate;
impl Entity for Aggregate { type Id = u64; fn id(&self) -> &u64 { unreachable!() } }
impl AggregateRoot for Aggregate { type Event = NotEvent; fn record_event(&mut self, _: NotEvent) {} fn pull_events(&mut self) -> Vec<NotEvent> { Vec::new() } }
impl AggregateReplay for Aggregate {
    type InitializationError = (); type ApplicationError = (); type Snapshot = (); type SnapshotValidationError = (); type SnapshotStateError = ();
    fn initial_state(_: &u64) -> Result<Self, ()> { Ok(Aggregate) }
    fn apply_event(&mut self, _: &NotEvent) -> Result<(), ()> { Ok(()) }
    fn classify_snapshot(_: &AggregateSnapshot<()>, _: StreamRevision) -> Result<SnapshotUse, ()> { Ok(SnapshotUse::Applicable) }
    fn from_snapshot(_: &u64, _: AggregateSnapshot<()>) -> Result<Self, ()> { Ok(Aggregate) }
}
fn main() {}
