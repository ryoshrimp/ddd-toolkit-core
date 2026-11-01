use ddd_toolkit_core::{
    AggregateRoot, AsyncDomainEventPublisher, AsyncLifecycleContext, AsyncRepository, DomainEvent,
};
use std::future::Future;
use std::pin::Pin;

fn leak_capability<'a, A, R, P>(context: &'a AsyncLifecycleContext<'a, A, R, P>)
where
    A: AggregateRoot + Send + 'a,
    R: AsyncRepository<Aggregate = A> + Send + 'a,
    P: AsyncDomainEventPublisher<Event = A::Event> + Send + 'a,
    A::Event: DomainEvent + Send + Sync + 'a,
    R::Error: Send + 'a,
    P::Error: Send + 'a,
{
    let _ = &context.aggregate;
    let _ = &context.repository;
    let _ = &context.publisher;
}

fn main() {}

#[allow(dead_code)]
struct Unused;
impl DomainEvent for Unused {}

#[allow(dead_code)]
fn _future_bound<T: Future + Send + 'static>(_: Pin<Box<T>>) {}
