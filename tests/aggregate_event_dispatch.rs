use ddd_toolkit_core::{AggregateRoot, DomainEvent, DomainEventPublisher, Entity, dispatch_events};
use std::cell::Cell;
use std::rc::Rc;

#[derive(Debug, PartialEq)]
struct TestEvent {
    id: u64,
}

impl DomainEvent for TestEvent {}

struct Aggregate {
    id: u64,
    events: Vec<TestEvent>,
    pull_calls: usize,
    restored_events: Vec<u64>,
}

impl Entity for Aggregate {
    type Id = u64;

    fn id(&self) -> &Self::Id {
        &self.id
    }
}

impl AggregateRoot for Aggregate {
    type Event = TestEvent;

    fn record_event(&mut self, event: Self::Event) {
        self.restored_events.push(event.id);
        self.events.push(event);
    }

    fn pull_events(&mut self) -> Vec<Self::Event> {
        self.pull_calls += 1;
        std::mem::take(&mut self.events)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PublishError {
    Unavailable { attempt: usize },
}

struct TestPublisher {
    calls: Vec<u64>,
    fail_at: Option<(usize, PublishError)>,
    panic_at: Option<usize>,
}

impl DomainEventPublisher for TestPublisher {
    type Event = TestEvent;
    type Error = PublishError;

    fn publish(&mut self, event: &Self::Event) -> Result<(), Self::Error> {
        let attempt = self.calls.len();
        self.calls.push(event.id);

        if self.panic_at == Some(attempt) {
            panic!("publisher panic at attempt {attempt}");
        }

        if let Some((failure_at, error)) = &self.fail_at {
            if *failure_at == attempt {
                return Err(error.clone());
            }
        }

        Ok(())
    }
}

fn aggregate_with_events() -> Aggregate {
    Aggregate {
        id: 1,
        events: vec![
            TestEvent { id: 1 },
            TestEvent { id: 2 },
            TestEvent { id: 3 },
        ],
        pull_calls: 0,
        restored_events: Vec::new(),
    }
}

fn publisher() -> TestPublisher {
    TestPublisher {
        calls: Vec::new(),
        fail_at: None,
        panic_at: None,
    }
}

struct InteriorEvent {
    id: u64,
    state: Rc<Cell<u64>>,
}

impl DomainEvent for InteriorEvent {}

struct InteriorAggregate {
    id: u64,
    events: Vec<InteriorEvent>,
    pull_calls: usize,
    restored_events: Vec<(u64, u64)>,
}

impl Entity for InteriorAggregate {
    type Id = u64;

    fn id(&self) -> &Self::Id {
        &self.id
    }
}

impl AggregateRoot for InteriorAggregate {
    type Event = InteriorEvent;

    fn record_event(&mut self, event: Self::Event) {
        self.restored_events.push((event.id, event.state.get()));
        self.events.push(event);
    }

    fn pull_events(&mut self) -> Vec<Self::Event> {
        self.pull_calls += 1;
        std::mem::take(&mut self.events)
    }
}

struct InteriorPublisher {
    observed: Vec<(u64, u64)>,
    fail_at: Option<(usize, PublishError)>,
}

impl DomainEventPublisher for InteriorPublisher {
    type Event = InteriorEvent;
    type Error = PublishError;

    fn publish(&mut self, event: &Self::Event) -> Result<(), Self::Error> {
        let attempt = self.observed.len();
        self.observed.push((event.id, event.state.get()));

        if let Some((failure_at, error)) = &self.fail_at {
            if *failure_at == attempt {
                return Err(error.clone());
            }
        }

        Ok(())
    }
}

fn interior_aggregate_with_events() -> (InteriorAggregate, Vec<Rc<Cell<u64>>>) {
    let states = (1..=3)
        .map(|value| Rc::new(Cell::new(value * 10)))
        .collect::<Vec<_>>();
    let events = states
        .iter()
        .enumerate()
        .map(|(index, state)| InteriorEvent {
            id: index as u64 + 1,
            state: Rc::clone(state),
        })
        .collect();

    (
        InteriorAggregate {
            id: 1,
            events,
            pull_calls: 0,
            restored_events: Vec::new(),
        },
        states,
    )
}

fn interior_event_values(events: &[InteriorEvent]) -> Vec<(u64, u64)> {
    events
        .iter()
        .map(|event| (event.id, event.state.get()))
        .collect()
}

fn interior_state_values(states: &[Rc<Cell<u64>>]) -> Vec<u64> {
    states.iter().map(|state| state.get()).collect()
}

fn interior_publisher() -> InteriorPublisher {
    InteriorPublisher {
        observed: Vec::new(),
        fail_at: None,
    }
}

#[test]
fn successful_dispatch_publishes_fifo_and_clears_queue() {
    let mut aggregate = aggregate_with_events();
    let mut publisher = publisher();

    assert_eq!(dispatch_events(&mut aggregate, &mut publisher), Ok(()));

    assert_eq!(aggregate.pull_calls, 1);
    assert_eq!(publisher.calls, [1, 2, 3]);
    assert!(aggregate.events.is_empty());
    assert!(aggregate.restored_events.is_empty());
}

#[test]
fn empty_dispatch_is_a_successful_no_op() {
    let mut aggregate = Aggregate {
        events: Vec::new(),
        ..aggregate_with_events()
    };
    let mut publisher = publisher();

    assert_eq!(dispatch_events(&mut aggregate, &mut publisher), Ok(()));

    assert_eq!(aggregate.pull_calls, 1);
    assert!(publisher.calls.is_empty());
    assert!(aggregate.events.is_empty());
}

#[test]
fn every_failure_position_stops_and_restores_the_complete_queue() {
    for failure_at in 0..3 {
        let mut aggregate = aggregate_with_events();
        let mut publisher = TestPublisher {
            fail_at: Some((
                failure_at,
                PublishError::Unavailable {
                    attempt: failure_at,
                },
            )),
            ..publisher()
        };
        let expected_error = publisher.fail_at.as_ref().unwrap().1.clone();

        assert_eq!(
            dispatch_events(&mut aggregate, &mut publisher),
            Err(expected_error)
        );

        assert_eq!(aggregate.pull_calls, 1);
        assert_eq!(
            publisher.calls,
            (1..=failure_at + 1).map(|id| id as u64).collect::<Vec<_>>()
        );
        assert_eq!(aggregate.restored_events, [1, 2, 3]);
        assert_eq!(
            aggregate.events,
            [
                TestEvent { id: 1 },
                TestEvent { id: 2 },
                TestEvent { id: 3 }
            ]
        );
    }
}

#[test]
fn publisher_panic_restores_queue_before_unwind_continues() {
    let mut aggregate = aggregate_with_events();
    let mut publisher = TestPublisher {
        panic_at: Some(1),
        ..publisher()
    };

    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        dispatch_events(&mut aggregate, &mut publisher)
    }));

    assert_eq!(
        panic
            .as_ref()
            .err()
            .and_then(|payload| payload.downcast_ref::<String>())
            .map(String::as_str),
        Some("publisher panic at attempt 1")
    );
    assert_eq!(aggregate.pull_calls, 1);
    assert_eq!(publisher.calls, [1, 2]);
    assert_eq!(aggregate.restored_events, [1, 2, 3]);
    assert_eq!(
        aggregate.events,
        [
            TestEvent { id: 1 },
            TestEvent { id: 2 },
            TestEvent { id: 3 }
        ]
    );
}

#[test]
fn publisher_observes_original_event_values() {
    let mut aggregate = aggregate_with_events();
    let mut publisher = publisher();

    dispatch_events(&mut aggregate, &mut publisher).unwrap();

    assert_eq!(publisher.calls, [1, 2, 3]);
}

#[test]
fn read_only_publisher_preserves_interior_event_state_on_success() {
    let (mut aggregate, states) = interior_aggregate_with_events();
    let mut publisher = interior_publisher();

    assert_eq!(dispatch_events(&mut aggregate, &mut publisher), Ok(()));

    assert_eq!(aggregate.pull_calls, 1);
    assert_eq!(publisher.observed, [(1, 10), (2, 20), (3, 30)]);
    assert_eq!(interior_state_values(&states), [10, 20, 30]);
    assert!(aggregate.events.is_empty());
}

#[test]
fn read_only_publisher_preserves_interior_event_state_on_failure_and_restores_queue() {
    let (mut aggregate, states) = interior_aggregate_with_events();
    let expected_error = PublishError::Unavailable { attempt: 1 };
    let mut publisher = InteriorPublisher {
        fail_at: Some((1, expected_error.clone())),
        ..interior_publisher()
    };

    assert_eq!(
        dispatch_events(&mut aggregate, &mut publisher),
        Err(expected_error)
    );

    assert_eq!(aggregate.pull_calls, 1);
    assert_eq!(publisher.observed, [(1, 10), (2, 20)]);
    assert_eq!(aggregate.restored_events, [(1, 10), (2, 20), (3, 30)]);
    assert_eq!(
        interior_event_values(&aggregate.events),
        [(1, 10), (2, 20), (3, 30)]
    );
    assert_eq!(interior_state_values(&states), [10, 20, 30]);
}
