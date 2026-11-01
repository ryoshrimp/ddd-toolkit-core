use ddd_toolkit_core::{AggregateReplay, AggregateRoot, DomainEvent, Entity};
struct Event; impl DomainEvent for Event {}
struct Aggregate;
impl Entity for Aggregate { type Id = u64; fn id(&self) -> &u64 { unreachable!() } }
impl AggregateRoot for Aggregate { type Event = Event; fn record_event(&mut self, _: Event) {} fn pull_events(&mut self) -> Vec<Event> { Vec::new() } }
impl AggregateReplay for Aggregate {}
fn main() {}
