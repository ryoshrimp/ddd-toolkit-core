use ddd_toolkit_core::{
    AggregateRoot, DomainEvent, DomainEventPublisher, Entity, LifecycleContext, Repository,
};

struct Event;
impl DomainEvent for Event {}

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
    fn record_event(&mut self, event: Self::Event) {
        self.events.push(event);
    }
    fn pull_events(&mut self) -> Vec<Self::Event> {
        std::mem::take(&mut self.events)
    }
}

struct RepositoryAdapter;
impl Repository for RepositoryAdapter {
    type Aggregate = Aggregate;
    type Error = ();
    fn find_by_id(&self, _: &u64) -> Result<Option<Aggregate>, ()> {
        Ok(None)
    }
    fn save(&mut self, _: &Aggregate) -> Result<(), ()> {
        Ok(())
    }
    fn delete(&mut self, _: &u64) -> Result<(), ()> {
        Ok(())
    }
    fn exists(&self, _: &u64) -> Result<bool, ()> {
        Ok(false)
    }
}

struct Publisher;
impl DomainEventPublisher for Publisher {
    type Event = Event;
    type Error = ();
    fn publish(&mut self, _: &Event) -> Result<(), ()> {
        Ok(())
    }
}

fn leak_capability<'a>(context: &'a mut LifecycleContext<'a, Aggregate, RepositoryAdapter, Publisher>) {
    let _ = &context.aggregate;
    let _ = &context.repository;
    let _ = &context.publisher;
    let _ = &context.aggregate.events;
}

fn main() {}
