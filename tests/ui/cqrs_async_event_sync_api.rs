use ddd_toolkit_core::{
    dispatch_events_async, AggregateRoot, AsyncDomainEventPublisher, AsyncLifecycleContext,
    AsyncRepository, DomainEvent, Entity,
};
use std::cell::Cell;
use std::future::Future;
use std::pin::Pin;

struct Event(Cell<u64>);
impl DomainEvent for Event {}
struct Aggregate {
    id: u64,
    events: Vec<Event>,
}
impl Entity for Aggregate {
    type Id = u64;
    fn id(&self) -> &u64 { &self.id }
}
impl AggregateRoot for Aggregate {
    type Event = Event;
    fn record_event(&mut self, event: Event) { self.events.push(event); }
    fn pull_events(&mut self) -> Vec<Event> { std::mem::take(&mut self.events) }
}
struct Repo;
impl AsyncRepository for Repo {
    type Aggregate = Aggregate;
    type Error = ();
    fn find_by_id<'a>(&'a self, _: &'a u64) -> Pin<Box<dyn Future<Output = Result<Option<Aggregate>, ()>> + Send + 'a>> { Box::pin(async { Ok(None) }) }
    fn save<'a>(&'a mut self, _: &'a Aggregate) -> Pin<Box<dyn Future<Output = Result<(), ()>> + Send + 'a>> { Box::pin(async { Ok(()) }) }
    fn delete<'a>(&'a mut self, _: &'a u64) -> Pin<Box<dyn Future<Output = Result<(), ()>> + Send + 'a>> { Box::pin(async { Ok(()) }) }
    fn exists<'a>(&'a self, _: &'a u64) -> Pin<Box<dyn Future<Output = Result<bool, ()>> + Send + 'a>> { Box::pin(async { Ok(false) }) }
}
struct Publisher;
impl AsyncDomainEventPublisher for Publisher {
    type Event = Event;
    type Error = ();
    fn publish<'a>(&'a mut self, _: &'a Event) -> Pin<Box<dyn Future<Output = Result<(), ()>> + Send + 'a>> { Box::pin(async { Ok(()) }) }
}

fn main() {
    let mut aggregate = Aggregate { id: 1, events: vec![Event(Cell::new(1))] };
    let mut repo = Repo;
    let mut publisher = Publisher;
    let _ = AsyncLifecycleContext::new(&mut aggregate, &mut repo, &mut publisher);
    let _ = dispatch_events_async(&mut aggregate, &mut publisher);
}
