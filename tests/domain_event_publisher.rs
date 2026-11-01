use ddd_toolkit_core::{AggregateRoot, DomainEvent, DomainEventPublisher, Entity};

#[derive(Debug, PartialEq)]
struct UserRegistered {
    id: u64,
}

impl DomainEvent for UserRegistered {}

#[derive(Debug, PartialEq, Eq)]
enum PublishError {
    Unavailable,
}

struct InMemoryPublisher {
    fail: bool,
    published_ids: Vec<u64>,
}

impl DomainEventPublisher for InMemoryPublisher {
    type Event = UserRegistered;
    type Error = PublishError;

    fn publish(&mut self, event: &Self::Event) -> Result<(), Self::Error> {
        if self.fail {
            return Err(PublishError::Unavailable);
        }

        self.published_ids.push(event.id);
        Ok(())
    }
}

#[test]
fn publisher_associates_a_domain_event_and_adapter_error() {
    let mut publisher = InMemoryPublisher {
        fail: false,
        published_ids: vec![],
    };
    let event = UserRegistered { id: 7 };

    assert_eq!(publisher.publish(&event), Ok(()));
    assert_eq!(publisher.published_ids, [7]);
}

#[test]
fn publisher_returns_its_adapter_error_unchanged() {
    let mut publisher = InMemoryPublisher {
        fail: true,
        published_ids: Vec::new(),
    };
    let event = UserRegistered { id: 7 };

    assert_eq!(publisher.publish(&event), Err(PublishError::Unavailable));
    assert!(publisher.published_ids.is_empty());
}

#[test]
fn publisher_borrows_and_does_not_consume_the_event() {
    let mut publisher = InMemoryPublisher {
        fail: false,
        published_ids: Vec::new(),
    };
    let event = UserRegistered { id: 11 };

    publisher.publish(&event).unwrap();

    assert_eq!(event, UserRegistered { id: 11 });
}

struct Aggregate {
    id: u64,
    events: Vec<UserRegistered>,
}

impl Entity for Aggregate {
    type Id = u64;

    fn id(&self) -> &Self::Id {
        &self.id
    }
}

impl AggregateRoot for Aggregate {
    type Event = UserRegistered;

    fn record_event(&mut self, event: Self::Event) {
        self.events.push(event);
    }

    fn pull_events(&mut self) -> Vec<Self::Event> {
        std::mem::take(&mut self.events)
    }
}

#[test]
fn application_code_controls_event_queue_and_publication_order() {
    let mut aggregate = Aggregate {
        id: 1,
        events: vec![],
    };
    aggregate.record_event(UserRegistered { id: 1 });

    let mut publisher = InMemoryPublisher {
        fail: false,
        published_ids: vec![],
    };
    let events = aggregate.pull_events();
    assert_eq!(events, [UserRegistered { id: 1 }]);
    publisher.publish(&events[0]).unwrap();

    assert!(aggregate.pull_events().is_empty());
    assert_eq!(publisher.published_ids, [1]);
}
