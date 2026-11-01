use ddd_toolkit_core::{
    AggregateRoot, DomainEvent, DomainEventPublisher, Entity, LifecycleCallback,
    LifecycleContext, Repository,
};

struct Event;
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
impl Repository for Repo {
    type Aggregate = Aggregate;
    type Error = ();
    fn find_by_id(&self, _: &u64) -> Result<Option<Aggregate>, ()> { Ok(None) }
    fn save(&mut self, _: &Aggregate) -> Result<(), ()> { Ok(()) }
    fn delete(&mut self, _: &u64) -> Result<(), ()> { Ok(()) }
    fn exists(&self, _: &u64) -> Result<bool, ()> { Ok(false) }
}
struct Publisher;
impl DomainEventPublisher for Publisher {
    type Event = Event;
    type Error = ();
    fn publish(&mut self, _: &Event) -> Result<(), ()> { Ok(()) }
}
struct Callback;
impl<'storage> LifecycleCallback<'storage, Aggregate, Repo, Publisher> for Callback {
    type Output = ();
    type Error = ();
    fn call<'callback>(
        self,
        context: &'callback mut LifecycleContext<'storage, Aggregate, Repo, Publisher>,
    ) -> Result<(), ()>
    where
        'storage: 'callback,
    {
        context.save().map_err(|_| ())?;
        context.dispatch().map_err(|_| ())?;
        Ok(())
    }
}
fn main() {
    let mut aggregate = Aggregate { id: 1, events: vec![Event] };
    let mut repo = Repo;
    let mut publisher = Publisher;
    let mut context = LifecycleContext::new(&mut aggregate, &mut repo, &mut publisher);
    Callback.call(&mut context).unwrap();
    context.complete().unwrap();
}
