use ddd_toolkit_core::{AggregateRoot, CommandHandler, DomainEvent, DomainEventPublisher, Entity, Repository};

struct Event;
impl DomainEvent for Event {}

struct Aggregate;
impl Entity for Aggregate {
    type Id = u64;
    fn id(&self) -> &u64 { &0 }
}
impl AggregateRoot for Aggregate {
    type Event = Event;
    fn record_event(&mut self, _: Event) {}
    fn pull_events(&mut self) -> Vec<Event> { Vec::new() }
}

struct NotAnAggregate;
struct Repo;
impl Repository for Repo {
    type Aggregate = NotAnAggregate;
    type Error = ();
    fn find_by_id(&self, _: &u64) -> Result<Option<NotAnAggregate>, ()> { Ok(None) }
    fn save(&mut self, _: &NotAnAggregate) -> Result<(), ()> { Ok(()) }
    fn delete(&mut self, _: &u64) -> Result<(), ()> { Ok(()) }
    fn exists(&self, _: &u64) -> Result<bool, ()> { Ok(false) }
}
struct Publisher;
impl DomainEventPublisher for Publisher {
    type Event = Event;
    type Error = ();
    fn publish(&mut self, _: &Event) -> Result<(), ()> { Ok(()) }
}

struct InvalidHandler;
impl CommandHandler for InvalidHandler {
    type Command = ();
    type Aggregate = NotAnAggregate;
    type Repository = Repo;
    type Publisher = Publisher;
    type Output = ();
    type Error = ();
    fn handle<F>(&mut self, _: &u64, _: (), _: F) -> Result<(), ()> {
        unimplemented!()
    }
}

fn main() {}
