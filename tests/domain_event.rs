use ddd_toolkit_core::{AggregateRoot, DomainEvent, Entity};

struct ManualEvent;
impl DomainEvent for ManualEvent {}

struct ManualAggregate {
    id: u64,
    events: Vec<ManualEvent>,
}

impl Entity for ManualAggregate {
    type Id = u64;

    fn id(&self) -> &Self::Id {
        &self.id
    }
}

impl AggregateRoot for ManualAggregate {
    type Event = ManualEvent;

    fn record_event(&mut self, event: Self::Event) {
        self.events.push(event);
    }

    fn pull_events(&mut self) -> Vec<Self::Event> {
        std::mem::take(&mut self.events)
    }
}

#[test]
fn manually_implemented_domain_event_and_aggregate_root_are_usable() {
    let mut aggregate = ManualAggregate {
        id: 1,
        events: vec![],
    };

    aggregate.record_event(ManualEvent);
    assert_eq!(aggregate.pull_events().len(), 1);
    assert!(aggregate.pull_events().is_empty())
}
