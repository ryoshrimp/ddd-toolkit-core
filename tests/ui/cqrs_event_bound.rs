use ddd_toolkit_core::{AggregateRoot, DomainEvent, DomainEventPublisher, Entity};

struct EventA;
impl DomainEvent for EventA {}
struct EventB;
impl DomainEvent for EventB {}

struct Aggregate;
impl Entity for Aggregate {
    type Id = u64;
    fn id(&self) -> &u64 { &0 }
}
impl AggregateRoot for Aggregate {
    type Event = EventA;
    fn record_event(&mut self, _: EventA) {}
    fn pull_events(&mut self) -> Vec<EventA> { Vec::new() }
}

struct WrongPublisher;
impl DomainEventPublisher for WrongPublisher {
    type Event = EventB;
    type Error = ();
    fn publish(&mut self, _: &EventB) -> Result<(), ()> { Ok(()) }
}

fn requires_matching_publisher<A, P>()
where
    A: AggregateRoot,
    P: DomainEventPublisher<Event = A::Event>,
{
}

fn main() {
    requires_matching_publisher::<Aggregate, WrongPublisher>();
}
