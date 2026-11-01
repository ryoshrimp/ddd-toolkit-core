use ddd_toolkit_core::{replay_with_optional_snapshot, AggregateReplay, AggregateRoot, AggregateSnapshot, DomainEvent, Entity, SnapshotUse, StreamHistory, StreamRevision};
struct Event; impl DomainEvent for Event {}
struct Aggregate;
impl Entity for Aggregate { type Id = u64; fn id(&self) -> &u64 { unreachable!() } }
impl AggregateRoot for Aggregate { type Event = Event; fn record_event(&mut self, _: Event) {} fn pull_events(&mut self) -> Vec<Event> { Vec::new() } }
impl AggregateReplay for Aggregate {
 type InitializationError=(); type ApplicationError=(); type Snapshot=u32; type SnapshotValidationError=(); type SnapshotStateError=();
 fn initial_state(_: &u64)->Result<Self,()>{Ok(Aggregate)} fn apply_event(&mut self, _: &Event)->Result<(),()>{Ok(())}
 fn classify_snapshot(_: &AggregateSnapshot<u32>, _: StreamRevision)->Result<SnapshotUse,()>{Ok(SnapshotUse::Applicable)}
 fn from_snapshot(_: &u64, _: AggregateSnapshot<u32>)->Result<Self,()>{Ok(Aggregate)}
}
fn main() { let h = StreamHistory::try_new(vec![Event], StreamRevision::new(0)).unwrap(); let _ = replay_with_optional_snapshot::<Aggregate>(&1, &h, Some(AggregateSnapshot::new(String::new(), StreamRevision::new(0)))); }
