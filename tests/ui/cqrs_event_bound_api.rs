use ddd_toolkit_core::{AggregateRoot, CommandHandler, DomainEvent, DomainEventPublisher, Entity, Repository};

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

struct Repo;
impl Repository for Repo {
    type Aggregate = Aggregate;
    type Error = ();
    fn find_by_id(&self, _: &u64) -> Result<Option<Aggregate>, ()> { Ok(None) }
    fn save(&mut self, _: &Aggregate) -> Result<(), ()> { Ok(()) }
    fn delete(&mut self, _: &u64) -> Result<(), ()> { Ok(()) }
    fn exists(&self, _: &u64) -> Result<bool, ()> { Ok(false) }
}
struct WrongPublisher;
impl DomainEventPublisher for WrongPublisher {
    type Event = EventB;
    type Error = ();
    fn publish(&mut self, _: &EventB) -> Result<(), ()> { Ok(()) }
}

struct InvalidHandler;
impl CommandHandler for InvalidHandler {
    type Command = ();
    type Aggregate = Aggregate;
    type Repository = Repo;
    type Publisher = WrongPublisher;
    type Output = ();
    type Error = ();
    fn handle<F>(&mut self, _: &u64, _: (), _: F) -> Result<(), ()> {
        unimplemented!()
    }
}

fn main() {}
