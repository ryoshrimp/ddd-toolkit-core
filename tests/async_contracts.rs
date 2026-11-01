use ddd_toolkit_core::{
    AggregateRoot, AsyncDomainEventPublisher, AsyncRepository, DomainEvent, Entity,
    dispatch_events_async,
};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

#[derive(Clone, Debug, PartialEq)]
struct TestEvent {
    id: u64,
}

impl DomainEvent for TestEvent {}

#[derive(Clone, Debug, PartialEq)]
struct TestAggregate {
    id: u64,
    events: Vec<TestEvent>,
}

impl Entity for TestAggregate {
    type Id = u64;

    fn id(&self) -> &Self::Id {
        &self.id
    }
}

impl AggregateRoot for TestAggregate {
    type Event = TestEvent;

    fn record_event(&mut self, event: Self::Event) {
        self.events.push(event);
    }

    fn pull_events(&mut self) -> Vec<Self::Event> {
        std::mem::take(&mut self.events)
    }
}

fn poll_ready<T>(mut future: Pin<Box<dyn Future<Output = T> + Send + '_>>) -> T {
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    match future.as_mut().poll(&mut context) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("fixture future did not complete in one poll"),
    }
}

struct InMemoryRepository {
    stored: Option<TestAggregate>,
    failure: Option<&'static str>,
}

impl AsyncRepository for InMemoryRepository {
    type Aggregate = TestAggregate;
    type Error = &'static str;

    fn find_by_id<'a>(
        &'a self,
        id: &'a <Self::Aggregate as Entity>::Id,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Self::Aggregate>, Self::Error>> + Send + 'a>>
    {
        Box::pin(async move {
            if let Some(error) = self.failure {
                return Err(error);
            }
            Ok(self
                .stored
                .as_ref()
                .filter(|aggregate| aggregate.id() == id)
                .cloned())
        })
    }

    fn save<'a>(
        &'a mut self,
        aggregate: &'a Self::Aggregate,
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'a>> {
        Box::pin(async move {
            if let Some(error) = self.failure {
                return Err(error);
            }
            self.stored = Some(aggregate.clone());
            Ok(())
        })
    }

    fn delete<'a>(
        &'a mut self,
        id: &'a <Self::Aggregate as Entity>::Id,
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'a>> {
        Box::pin(async move {
            if let Some(error) = self.failure {
                return Err(error);
            }
            if self
                .stored
                .as_ref()
                .is_some_and(|aggregate| aggregate.id() == id)
            {
                self.stored = None;
            }
            Ok(())
        })
    }

    fn exists<'a>(
        &'a self,
        id: &'a <Self::Aggregate as Entity>::Id,
    ) -> Pin<Box<dyn Future<Output = Result<bool, Self::Error>> + Send + 'a>> {
        Box::pin(async move {
            if let Some(error) = self.failure {
                return Err(error);
            }
            Ok(self
                .stored
                .as_ref()
                .is_some_and(|aggregate| aggregate.id() == id))
        })
    }
}

#[test]
fn async_repository_preserves_result_shapes_and_borrowed_inputs() {
    let aggregate = TestAggregate {
        id: 7,
        events: vec![TestEvent { id: 1 }],
    };
    let mut repository = InMemoryRepository {
        stored: None,
        failure: None,
    };

    assert_eq!(poll_ready(repository.save(&aggregate)), Ok(()));
    assert_eq!(aggregate.events.len(), 1);
    assert_eq!(poll_ready(repository.exists(&7)), Ok(true));
    assert_eq!(
        poll_ready(repository.find_by_id(&7)),
        Ok(Some(aggregate.clone()))
    );
    assert_eq!(poll_ready(repository.delete(&7)), Ok(()));
    assert_eq!(poll_ready(repository.find_by_id(&7)), Ok(None));
    assert_eq!(poll_ready(repository.exists(&7)), Ok(false));
}

#[test]
fn async_repository_returns_adapter_errors_unchanged() {
    let mut repository = InMemoryRepository {
        stored: None,
        failure: Some("adapter failure"),
    };
    let aggregate = TestAggregate {
        id: 7,
        events: Vec::new(),
    };

    assert_eq!(
        poll_ready(repository.find_by_id(&7)),
        Err("adapter failure")
    );
    assert_eq!(
        poll_ready(repository.save(&aggregate)),
        Err("adapter failure")
    );
    assert_eq!(poll_ready(repository.delete(&7)), Err("adapter failure"));
    assert_eq!(poll_ready(repository.exists(&7)), Err("adapter failure"));
}

struct RecordingPublisher {
    calls: Vec<u64>,
    fail_on: Option<u64>,
    pending_on: Option<u64>,
}

impl AsyncDomainEventPublisher for RecordingPublisher {
    type Event = TestEvent;
    type Error = &'static str;

    fn publish<'a>(
        &'a mut self,
        event: &'a Self::Event,
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'a>> {
        self.calls.push(event.id);
        if self.fail_on == Some(event.id) {
            return Box::pin(async { Err("publication failure") });
        }
        if self.pending_on == Some(event.id) {
            return Box::pin(std::future::pending());
        }
        Box::pin(async { Ok(()) })
    }
}

fn aggregate_with_events() -> TestAggregate {
    TestAggregate {
        id: 7,
        events: vec![
            TestEvent { id: 1 },
            TestEvent { id: 2 },
            TestEvent { id: 3 },
        ],
    }
}

#[test]
fn async_publisher_borrows_events_and_preserves_adapter_errors() {
    let event = TestEvent { id: 9 };
    let mut publisher = RecordingPublisher {
        calls: Vec::new(),
        fail_on: Some(9),
        pending_on: None,
    };

    assert_eq!(
        poll_ready(publisher.publish(&event)),
        Err("publication failure")
    );
    assert_eq!(event.id, 9);
    assert_eq!(publisher.calls, vec![9]);
}

#[test]
fn async_dispatch_publishes_fifo_sequentially_and_commits_on_success() {
    let mut aggregate = aggregate_with_events();
    let mut publisher = RecordingPublisher {
        calls: Vec::new(),
        fail_on: None,
        pending_on: None,
    };

    assert_eq!(
        poll_ready(dispatch_events_async(&mut aggregate, &mut publisher)),
        Ok(())
    );
    assert!(aggregate.events.is_empty());
    assert_eq!(publisher.calls, vec![1, 2, 3]);
}

#[test]
fn async_dispatch_restores_complete_snapshot_on_error() {
    let mut aggregate = aggregate_with_events();
    let mut publisher = RecordingPublisher {
        calls: Vec::new(),
        fail_on: Some(2),
        pending_on: None,
    };

    assert_eq!(
        poll_ready(dispatch_events_async(&mut aggregate, &mut publisher)),
        Err("publication failure")
    );
    assert_eq!(publisher.calls, vec![1, 2]);
    assert_eq!(aggregate.events, aggregate_with_events().events);
}

#[test]
fn async_dispatch_restores_snapshot_when_future_is_dropped() {
    let mut aggregate = aggregate_with_events();
    let mut publisher = RecordingPublisher {
        calls: Vec::new(),
        fail_on: None,
        pending_on: Some(2),
    };
    let mut future = dispatch_events_async(&mut aggregate, &mut publisher);
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);

    assert!(future.as_mut().poll(&mut context).is_pending());
    drop(future);
    assert_eq!(publisher.calls, vec![1, 2]);
    assert_eq!(aggregate.events, aggregate_with_events().events);
}

#[test]
fn dropping_standalone_publisher_future_only_cancels_the_await() {
    let event = TestEvent { id: 4 };
    let mut publisher = RecordingPublisher {
        calls: Vec::new(),
        fail_on: None,
        pending_on: Some(4),
    };

    let future = publisher.publish(&event);
    drop(future);
    assert_eq!(publisher.calls, vec![4]);
}
