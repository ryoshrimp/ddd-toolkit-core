use crate::{domain::DomainEvent, port::PortError};

#[trait_variant::make(Send)]
pub trait EventDispatcher<E: DomainEvent> {
    async fn dispatch(&self, events: Vec<E>) -> Result<(), PortError>;
}

#[cfg(test)]
mod test {
    use std::{
        future::Future,
        pin::pin,
        sync::Mutex,
        task::{Context, Poll, Waker},
    };

    use crate::port::PortErrorKind;

    use super::*;

    fn block_on<F: Future>(future: F) -> F::Output {
        let mut future = pin!(future);
        let mut cx = Context::from_waker(Waker::noop());
        loop {
            if let Poll::Ready(output) = future.as_mut().poll(&mut cx) {
                return output;
            }
        }
    }

    #[derive(Debug, PartialEq)]
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
        async fn dispatch(&self, events: Vec<FooEvent>) -> Result<(), PortError> {
            self.dispatched.lock().unwrap().extend(events);
            Ok(())
        }
    }

    struct FailingDispatcher;

    impl EventDispatcher<FooEvent> for FailingDispatcher {
        async fn dispatch(&self, _events: Vec<FooEvent>) -> Result<(), PortError> {
            Err(PortError::unavailable("dispatch failed"))
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
        ) -> Result<(), PortError> {
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

        assert_eq!(error.kind(), PortErrorKind::Unavailable);
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
