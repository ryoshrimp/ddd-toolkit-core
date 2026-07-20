use crate::domain::{DomainEvent, Entity};

/// An [`Entity`] that is a consistency boundary and the unit of persistence:
/// changes to it (and whatever it contains) are saved and loaded together,
/// and it records the domain events those changes produce.
pub trait AggregateRoot: Entity {
    /// The domain event type this aggregate records.
    type Event: DomainEvent;

    /// Records that `event` happened, to be drained later via
    /// [`AggregateRoot::take_events`].
    fn record(&mut self, event: Self::Event);

    /// Returns and clears every event recorded since the last call.
    fn take_events(&mut self) -> Vec<Self::Event>;
}

#[cfg(test)]
mod test {
    use std::fmt::Display;

    use crate::domain::{EntityId, ValueObject};

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
    struct FooId(String);

    impl ValueObject for FooId {}

    impl Display for FooId {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl EntityId for FooId {}

    #[derive(Debug, PartialEq)]
    struct FooEvent {
        name: String,
    }

    impl DomainEvent for FooEvent {}

    fn event(name: &str) -> FooEvent {
        FooEvent {
            name: name.to_string(),
        }
    }

    struct Foo {
        id: FooId,
        events: Vec<FooEvent>,
    }

    impl Foo {
        fn new(id: &str) -> Self {
            Self {
                id: FooId(id.to_string()),
                events: Vec::new(),
            }
        }
    }

    impl Entity for Foo {
        type Id = FooId;

        fn id(&self) -> &Self::Id {
            &self.id
        }
    }

    impl AggregateRoot for Foo {
        type Event = FooEvent;

        fn record(&mut self, event: Self::Event) {
            self.events.push(event);
        }

        fn take_events(&mut self) -> Vec<Self::Event> {
            std::mem::take(&mut self.events)
        }
    }

    #[test]
    fn record_single_event_take_events_returns_it() {
        let mut foo = Foo::new("foo-1");

        foo.record(event("created"));

        assert_eq!(foo.take_events(), vec![event("created")]);
    }

    #[test]
    fn record_multiple_events_take_events_returns_in_recorded_order() {
        let mut foo = Foo::new("foo-1");

        foo.record(event("created"));
        foo.record(event("updated"));
        foo.record(event("deleted"));

        assert_eq!(
            foo.take_events(),
            vec![event("created"), event("updated"), event("deleted")]
        );
    }

    #[test]
    fn take_events_clears_recorded_events() {
        let mut foo = Foo::new("foo-1");
        foo.record(event("created"));

        foo.take_events();

        assert_eq!(foo.take_events(), vec![]);
    }

    #[test]
    fn record_after_take_events_returns_only_new_events() {
        let mut foo = Foo::new("foo-1");
        foo.record(event("created"));
        foo.take_events();

        foo.record(event("updated"));

        assert_eq!(foo.take_events(), vec![event("updated")]);
    }

    #[test]
    fn aggregate_root_impl_satisfies_entity_supertrait() {
        let a = Foo::new("foo-1");
        let b = Foo::new("foo-1");
        let c = Foo::new("foo-2");

        assert_eq!(a.id(), &FooId("foo-1".to_string()));
        assert!(a.is_same_as(&b));
        assert!(!a.is_same_as(&c));
    }

    #[test]
    fn aggregate_root_impl_satisfies_send_and_sync() {
        fn assert_impl<T: Send + Sync>() {}
        assert_impl::<Foo>();
    }

    #[test]
    fn generic_fn_accepts_aggregate_root_impl() {
        fn drain<T: AggregateRoot>(aggregate: &mut T, event: T::Event) -> Vec<T::Event> {
            aggregate.record(event);
            aggregate.take_events()
        }

        let mut foo = Foo::new("foo-1");

        assert_eq!(drain(&mut foo, event("created")), vec![event("created")]);
    }

    #[test]
    fn take_events_on_fresh_aggregate_returns_empty_vec() {
        let mut foo = Foo::new("foo-1");

        assert_eq!(foo.take_events(), vec![]);
    }

    #[test]
    fn record_duplicate_events_are_both_returned() {
        let mut foo = Foo::new("foo-1");

        foo.record(event("created"));
        foo.record(event("created"));

        assert_eq!(foo.take_events(), vec![event("created"), event("created")]);
    }
}
