use ddd_toolkit_core::{
    AggregateRoot, AsyncCommandHandler, AsyncDomainEventPublisher, AsyncLifecycleCallback,
    AsyncLifecycleContext, AsyncRepository, DomainEvent, Entity, LifecycleCompletionError,
    LifecycleOperationError,
};
use std::cell::Cell;
use std::future::Future;
use std::pin::Pin;

struct Event;
impl DomainEvent for Event {}
struct Aggregate {
    id: Cell<u64>,
}
impl Entity for Aggregate {
    type Id = Cell<u64>;
    fn id(&self) -> &Self::Id { &self.id }
}
impl AggregateRoot for Aggregate {
    type Event = Event;
    fn record_event(&mut self, _: Event) {}
    fn pull_events(&mut self) -> Vec<Event> { Vec::new() }
}
struct Repo;
impl AsyncRepository for Repo {
    type Aggregate = Aggregate;
    type Error = ();
    fn find_by_id<'a>(&'a self, _: &'a Cell<u64>) -> Pin<Box<dyn Future<Output = Result<Option<Aggregate>, ()>> + Send + 'a>> { Box::pin(async { Ok(None) }) }
    fn save<'a>(&'a mut self, _: &'a Aggregate) -> Pin<Box<dyn Future<Output = Result<(), ()>> + Send + 'a>> { Box::pin(async { Ok(()) }) }
    fn delete<'a>(&'a mut self, _: &'a Cell<u64>) -> Pin<Box<dyn Future<Output = Result<(), ()>> + Send + 'a>> { Box::pin(async { Ok(()) }) }
    fn exists<'a>(&'a self, _: &'a Cell<u64>) -> Pin<Box<dyn Future<Output = Result<bool, ()>> + Send + 'a>> { Box::pin(async { Ok(false) }) }
}
struct Publisher;
impl AsyncDomainEventPublisher for Publisher {
    type Event = Event;
    type Error = ();
    fn publish<'a>(&'a mut self, _: &'a Event) -> Pin<Box<dyn Future<Output = Result<(), ()>> + Send + 'a>> { Box::pin(async { Ok(()) }) }
}
#[derive(Debug)] struct Error;
impl From<LifecycleOperationError<()>> for Error { fn from(_: LifecycleOperationError<()>) -> Self { Error } }
impl From<LifecycleCompletionError> for Error { fn from(_: LifecycleCompletionError) -> Self { Error } }

struct Callback;
impl AsyncLifecycleCallback<Aggregate, Repo, Publisher> for Callback {
    type Output = ();
    type Error = Error;
    fn call<'storage, 'callback>(
        self,
        _: &'callback mut AsyncLifecycleContext<'storage, Aggregate, Repo, Publisher>,
    ) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send + 'callback>>
    where
        'storage: 'callback,
        Aggregate: 'storage,
        Repo: 'storage,
        Publisher: 'storage,
        Event: 'storage,
        (): 'storage,
        Self: Send + 'callback,
        Self::Output: Send + 'callback,
        Self::Error: Send + 'callback,
    { Box::pin(async { Ok(()) }) }
}

struct Handler;
impl AsyncCommandHandler for Handler {
    type Command = ();
    type Aggregate = Aggregate;
    type Repository = Repo;
    type Publisher = Publisher;
    type Output = ();
    type Error = Error;
    fn handle<'a, F>(
        &'a mut self,
        _: &'a Cell<u64>,
        _: (),
        _: F,
    ) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send + 'a>>
    where
        Self: Send + 'a,
        Self::Command: Send + 'a,
        Self::Aggregate: Send + 'a,
        <Self::Aggregate as AggregateRoot>::Event: Send + Sync + 'a,
        Self::Repository: Send + 'a,
        <Self::Repository as AsyncRepository>::Error: Send + 'a,
        Self::Publisher: Send + 'a,
        <Self::Publisher as AsyncDomainEventPublisher>::Error: Send + 'a,
        Self::Output: Send + 'a,
        Self::Error: From<LifecycleOperationError<<Self::Repository as AsyncRepository>::Error>>
            + From<LifecycleOperationError<<Self::Publisher as AsyncDomainEventPublisher>::Error>>
            + From<LifecycleCompletionError> + Send + 'a,
        F: AsyncLifecycleCallback<Self::Aggregate, Self::Repository, Self::Publisher, Output = Self::Output, Error = Self::Error> + Send + 'a,
    { unimplemented!() }
}

fn main() {
    let mut handler = Handler;
    let id = Cell::new(1);
    let _ = handler.handle(&id, (), Callback);
}
