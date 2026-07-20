use std::fmt;

use crate::{domain::DomainEvent, port::PortError};

/// A `dispatch` failure that reports which events were not delivered, so a
/// caller can retry or persist just the remainder instead of losing track
/// of the whole batch.
///
/// Implementors that dispatch events one at a time should populate
/// `undelivered` with every event from the input that was not confirmed
/// delivered (typically the failing event and everything after it).
/// Implementors that dispatch atomically (all-or-nothing) should populate
/// it with the entire input batch on failure.
#[derive(Debug)]
pub struct DispatchError<E> {
    pub undelivered: Vec<E>,
    pub source: PortError,
}

impl<E> DispatchError<E> {
    pub fn new(undelivered: Vec<E>, source: PortError) -> Self {
        Self { undelivered, source }
    }
}

impl<E: fmt::Debug> fmt::Display for DispatchError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "dispatch failed with {} undelivered event(s): {}",
            self.undelivered.len(),
            self.source
        )
    }
}

impl<E: fmt::Debug> std::error::Error for DispatchError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

#[trait_variant::make(Send)]
pub trait EventDispatcher<E: DomainEvent> {
    async fn dispatch(&self, events: Vec<E>) -> Result<(), DispatchError<E>>;
}

#[cfg(test)]
mod test {
    use std::sync::Mutex;

    use crate::{port::PortErrorKind, testing::block_on};

    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct FooEvent {
        name: String,
    }

    impl FooEvent {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
            }
        }
    }

    impl DomainEvent for FooEvent {}

    struct RecordingDispatcher {
        dispatched: Mutex<Vec<FooEvent>>,
    }

    impl RecordingDispatcher {
        fn new() -> Self {
            Self {
                dispatched: Mutex::new(Vec::new()),
            }
        }
    }

    impl EventDispatcher<FooEvent> for RecordingDispatcher {
        async fn dispatch(&self, events: Vec<FooEvent>) -> Result<(), DispatchError<FooEvent>> {
            self.dispatched.lock().unwrap().extend(events);
            Ok(())
        }
    }

    struct FailingDispatcher;

    impl EventDispatcher<FooEvent> for FailingDispatcher {
        async fn dispatch(&self, events: Vec<FooEvent>) -> Result<(), DispatchError<FooEvent>> {
            Err(DispatchError::new(
                events,
                PortError::unavailable("dispatch failed"),
            ))
        }
    }

    // Dispatches events one at a time, failing on (and after) the event
    // whose name is "boom" - a stand-in for a partial-failure dispatcher.
    struct PartiallyFailingDispatcher;

    impl EventDispatcher<FooEvent> for PartiallyFailingDispatcher {
        async fn dispatch(&self, events: Vec<FooEvent>) -> Result<(), DispatchError<FooEvent>> {
            let boom_at = events.iter().position(|e| e.name == "boom");
            match boom_at {
                Some(index) => Err(DispatchError::new(
                    events[index..].to_vec(),
                    PortError::unavailable("dispatch failed"),
                )),
                None => Ok(()),
            }
        }
    }

    #[test]
    fn dispatch_records_all_events_in_order() {
        let dispatcher = RecordingDispatcher::new();
        let events = vec![FooEvent::new("created"), FooEvent::new("renamed")];

        block_on(dispatcher.dispatch(events)).expect("dispatch should succeed");

        assert_eq!(
            *dispatcher.dispatched.lock().unwrap(),
            vec![FooEvent::new("created"), FooEvent::new("renamed")]
        );
    }

    #[test]
    fn dispatch_called_twice_delivers_both_batches() {
        let dispatcher = RecordingDispatcher::new();

        block_on(dispatcher.dispatch(vec![FooEvent::new("created")]))
            .expect("first dispatch should succeed");
        block_on(dispatcher.dispatch(vec![FooEvent::new("renamed")]))
            .expect("second dispatch should succeed");

        assert_eq!(
            *dispatcher.dispatched.lock().unwrap(),
            vec![FooEvent::new("created"), FooEvent::new("renamed")]
        );
    }

    #[test]
    fn generic_fn_accepts_event_dispatcher_bound() {
        fn publish<D: EventDispatcher<FooEvent>>(
            dispatcher: &D,
            events: Vec<FooEvent>,
        ) -> Result<(), DispatchError<FooEvent>> {
            block_on(dispatcher.dispatch(events))
        }

        let dispatcher = RecordingDispatcher::new();

        publish(&dispatcher, vec![FooEvent::new("created")]).expect("publish should succeed");

        assert_eq!(
            *dispatcher.dispatched.lock().unwrap(),
            vec![FooEvent::new("created")]
        );
    }

    #[test]
    fn dispatch_propagates_port_error() {
        let dispatcher = FailingDispatcher;

        let error = block_on(dispatcher.dispatch(vec![FooEvent::new("created")]))
            .expect_err("dispatch should fail");

        assert_eq!(error.source.kind(), PortErrorKind::Unavailable);
    }

    #[test]
    fn dispatch_error_reports_the_undelivered_events() {
        let dispatcher = FailingDispatcher;
        let events = vec![FooEvent::new("created"), FooEvent::new("renamed")];

        let error = block_on(dispatcher.dispatch(events)).expect_err("dispatch should fail");

        assert_eq!(
            error.undelivered,
            vec![FooEvent::new("created"), FooEvent::new("renamed")]
        );
    }

    #[test]
    fn partial_failure_reports_only_the_events_after_the_failure_point() {
        let dispatcher = PartiallyFailingDispatcher;
        let events = vec![
            FooEvent::new("created"),
            FooEvent::new("boom"),
            FooEvent::new("renamed"),
        ];

        let error = block_on(dispatcher.dispatch(events)).expect_err("dispatch should fail");

        assert_eq!(
            error.undelivered,
            vec![FooEvent::new("boom"), FooEvent::new("renamed")]
        );
    }

    #[test]
    fn dispatch_error_display_includes_undelivered_count_and_source() {
        let error = DispatchError::new(
            vec![FooEvent::new("created")],
            PortError::unavailable("boom"),
        );

        assert_eq!(
            error.to_string(),
            "dispatch failed with 1 undelivered event(s): Unavailable: boom"
        );
    }

    #[test]
    fn dispatch_empty_vec_is_ok() {
        let dispatcher = RecordingDispatcher::new();

        block_on(dispatcher.dispatch(Vec::new())).expect("dispatch should succeed");

        assert!(dispatcher.dispatched.lock().unwrap().is_empty());
    }

    // Compile-time only (no runtime assertion): the generic bound accepts the
    // future as `Send` purely because `#[trait_variant::make(Send)]` promises
    // it at the trait level, not because of the concrete impl.
    #[test]
    fn dispatcher_future_is_send() {
        fn assert_send(_: impl Send) {}

        fn check<D: EventDispatcher<FooEvent>>(dispatcher: &D, events: Vec<FooEvent>) {
            assert_send(dispatcher.dispatch(events));
        }

        let dispatcher = RecordingDispatcher::new();

        check(&dispatcher, vec![FooEvent::new("created")]);
    }
}
