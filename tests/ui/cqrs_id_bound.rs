use ddd_toolkit_core::{AggregateRoot, CommandHandler, DomainEvent, DomainEventPublisher, Entity, LifecycleCallback, LifecycleCompletionError, LifecycleContext, LifecycleOperationError, Repository};

struct Event;
impl DomainEvent for Event {}
struct Aggregate;
impl Entity for Aggregate { type Id = u64; fn id(&self) -> &u64 { &0 } }
impl AggregateRoot for Aggregate {
    type Event = Event;
    fn record_event(&mut self, _: Event) {}
    fn pull_events(&mut self) -> Vec<Event> { Vec::new() }
}
struct Other;
impl Entity for Other { type Id = u64; fn id(&self) -> &u64 { &0 } }
impl AggregateRoot for Other {
    type Event = Event;
    fn record_event(&mut self, _: Event) {}
    fn pull_events(&mut self) -> Vec<Event> { Vec::new() }
}
struct BadRepo;
impl Repository for BadRepo {
    type Aggregate = Other;
    type Error = ();
    fn find_by_id(&self, _: &u64) -> Result<Option<Other>, ()> { Ok(None) }
    fn save(&mut self, _: &Other) -> Result<(), ()> { Ok(()) }
    fn delete(&mut self, _: &u64) -> Result<(), ()> { Ok(()) }
    fn exists(&self, _: &u64) -> Result<bool, ()> { Ok(false) }
}
struct Publisher;
impl DomainEventPublisher for Publisher {
    type Event = Event;
    type Error = ();
    fn publish(&mut self, _: &Event) -> Result<(), ()> { Ok(()) }
}
struct Error;
impl From<LifecycleOperationError<()>> for Error {
    fn from(_: LifecycleOperationError<()>) -> Self { Self }
}
impl From<LifecycleCompletionError> for Error {
    fn from(_: LifecycleCompletionError) -> Self { Self }
}
struct Callback;
impl<'storage> LifecycleCallback<'storage, Aggregate, BadRepo, Publisher> for Callback {
    type Output = ();
    type Error = Error;
    fn call<'callback>(self, _: &'callback mut LifecycleContext<'storage, Aggregate, BadRepo, Publisher>) -> Result<(), Error>
    where 'storage: 'callback { Ok(()) }
}
struct BadHandler;
impl CommandHandler for BadHandler {
    type Command = ();
    type Aggregate = Aggregate;
    type Repository = BadRepo;
    type Publisher = Publisher;
    type Output = ();
    type Error = Error;
    fn handle<F>(&mut self, _: &u64, _: (), _: F) -> Result<(), Error>
    where
        F: for<'a> LifecycleCallback<'a, Aggregate, BadRepo, Publisher, Output = (), Error = Error>,
        Error: From<LifecycleOperationError<()>> + From<LifecycleCompletionError>,
    { Ok(()) }
}
fn main() {}
