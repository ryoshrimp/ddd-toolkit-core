use std::fmt::Debug;

/// Something that happened in the domain, recorded by an
/// [`crate::domain::AggregateRoot`] and later published through an
/// [`crate::port::event::EventDispatcher`].
///
/// # Examples
///
/// See [`AggregateRoot`](crate::domain::AggregateRoot#examples) for a worked
/// example of recording and draining `DomainEvent`s.
pub trait DomainEvent: Debug + Send + Sync + 'static {}

#[cfg(test)]
mod test {
    use super::*;

    #[derive(Debug)]
    struct TestEvent;

    impl DomainEvent for TestEvent {}

    #[test]
    fn test_double_implements_domain_event() {
        let _event = TestEvent;
    }

    #[test]
    fn domain_event_is_dyn_compatible() {
        let _event: Box<dyn DomainEvent> = Box::new(TestEvent);
    }

    #[test]
    fn debug_format_works_via_trait_object() {
        let event: &dyn DomainEvent = &TestEvent;

        assert_eq!(format!("{event:?}"), "TestEvent");
    }

    #[test]
    fn trait_object_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync + ?Sized>() {}

        assert_send_sync::<dyn DomainEvent>();
    }

    #[test]
    fn test_double_with_fields_implements_domain_event() {
        #[derive(Debug)]
        struct TestEventWithFields {
            id: String,
        }

        impl DomainEvent for TestEventWithFields {}

        let event = TestEventWithFields {
            id: "event-1".to_string(),
        };
        assert_eq!(event.id, "event-1");

        let event: &dyn DomainEvent = &event;
        assert_eq!(
            format!("{event:?}"),
            r#"TestEventWithFields { id: "event-1" }"#
        );
    }
}
