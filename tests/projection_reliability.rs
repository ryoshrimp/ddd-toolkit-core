use ddd_toolkit_core::{
    AsyncIdempotentProjection, AsyncProjectionCheckpointStore, DomainEvent, EventConsumer,
    IdempotentProjection, Projection, ProjectionApplication, ProjectionCheckpoint,
    ProjectionCheckpointStore,
};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

struct Event;
impl DomainEvent for Event {}

struct Checkpoints {
    value: Option<ProjectionCheckpoint<u64>>,
}
impl ProjectionCheckpointStore for Checkpoints {
    type Cursor = u64;
    type Error = &'static str;

    fn load(&self) -> Result<Option<ProjectionCheckpoint<Self::Cursor>>, Self::Error> {
        Ok(self.value.clone())
    }

    fn save(&mut self, checkpoint: ProjectionCheckpoint<Self::Cursor>) -> Result<(), Self::Error> {
        self.value = Some(checkpoint);
        Ok(())
    }
}

struct ReadModel {
    applied: bool,
}
impl Projection for ReadModel {
    type Event = Event;
    type Error = &'static str;

    fn project(&mut self, _: &Self::Event) -> Result<(), Self::Error> {
        self.applied = true;
        Ok(())
    }
}
impl IdempotentProjection for ReadModel {
    type Event = Event;
    type Cursor = u64;
    type Error = &'static str;

    fn apply_once(
        &mut self,
        _: &Self::Event,
        checkpoint: &ProjectionCheckpoint<Self::Cursor>,
    ) -> Result<ProjectionApplication, Self::Error> {
        if self.applied || *checkpoint.cursor() == 4 {
            return Ok(ProjectionApplication::AlreadyApplied);
        }
        self.applied = true;
        Ok(ProjectionApplication::Applied)
    }
}

struct AsyncCheckpoints {
    value: Option<ProjectionCheckpoint<u64>>,
    fail: bool,
}
impl AsyncProjectionCheckpointStore for AsyncCheckpoints {
    type Cursor = u64;
    type Error = &'static str;

    fn load<'a>(
        &'a self,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<Option<ProjectionCheckpoint<Self::Cursor>>, Self::Error>>
                + Send
                + 'a,
        >,
    > {
        let result = if self.fail {
            Err("load failed")
        } else {
            Ok(self.value.clone())
        };
        Box::pin(async move { result })
    }

    fn save<'a>(
        &'a mut self,
        checkpoint: ProjectionCheckpoint<Self::Cursor>,
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'a>> {
        let fail = self.fail;
        Box::pin(async move {
            if fail {
                Err("save failed")
            } else {
                self.value = Some(checkpoint);
                Ok(())
            }
        })
    }
}

struct AsyncReadModel {
    applied: bool,
    fail: bool,
}
impl AsyncIdempotentProjection for AsyncReadModel {
    type Event = Event;
    type Cursor = u64;
    type Error = &'static str;

    fn apply_once<'a>(
        &'a mut self,
        _: &'a Self::Event,
        checkpoint: &'a ProjectionCheckpoint<Self::Cursor>,
    ) -> Pin<Box<dyn Future<Output = Result<ProjectionApplication, Self::Error>> + Send + 'a>> {
        let already_applied = self.applied || *checkpoint.cursor() == 4;
        let fail = self.fail;
        Box::pin(async move {
            if fail {
                Err("projection failed")
            } else if already_applied {
                Ok(ProjectionApplication::AlreadyApplied)
            } else {
                self.applied = true;
                Ok(ProjectionApplication::Applied)
            }
        })
    }
}

fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = Box::pin(future);
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    match future.as_mut().poll(&mut context) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("test future unexpectedly pending"),
    }
}

struct Consumer;
impl EventConsumer for Consumer {
    type Event = Event;
    type Error = &'static str;
    fn consume(&mut self, _: &Self::Event) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[test]
fn checkpoint_store_preserves_typed_cursor_ownership() {
    let mut store = Checkpoints { value: None };
    store.save(ProjectionCheckpoint::new(3)).unwrap();
    assert_eq!(store.load().unwrap().unwrap().into_cursor(), 3);
}

#[test]
fn idempotent_projection_is_optional_and_distinguishes_duplicate_application() {
    let event = Event;
    let mut model = ReadModel { applied: false };
    assert_eq!(
        model.apply_once(&event, &ProjectionCheckpoint::new(1)),
        Ok(ProjectionApplication::Applied)
    );
    assert_eq!(
        model.apply_once(&event, &ProjectionCheckpoint::new(1)),
        Ok(ProjectionApplication::AlreadyApplied)
    );
    assert_eq!(model.project(&event), Ok(()));
    let mut consumer = Consumer;
    assert_eq!(consumer.consume(&event), Ok(()));
}

#[test]
fn async_ports_preserve_typed_success_duplicate_and_failure_results() {
    let event = Event;
    let mut store = AsyncCheckpoints {
        value: None,
        fail: false,
    };
    block_on(store.save(ProjectionCheckpoint::new(7))).unwrap();
    assert_eq!(block_on(store.load()).unwrap().unwrap().into_cursor(), 7);

    let mut projection = AsyncReadModel {
        applied: false,
        fail: false,
    };
    assert_eq!(
        block_on(projection.apply_once(&event, &ProjectionCheckpoint::new(1))),
        Ok(ProjectionApplication::Applied)
    );
    assert_eq!(
        block_on(projection.apply_once(&event, &ProjectionCheckpoint::new(1))),
        Ok(ProjectionApplication::AlreadyApplied)
    );

    store.fail = true;
    assert_eq!(block_on(store.load()), Err("load failed"));
    let mut failing_projection = AsyncReadModel {
        applied: false,
        fail: true,
    };
    assert_eq!(
        block_on(failing_projection.apply_once(&event, &ProjectionCheckpoint::new(1))),
        Err("projection failed")
    );
}

#[test]
fn async_future_drop_is_the_cancellation_boundary() {
    let event = Event;
    let mut projection = AsyncReadModel {
        applied: false,
        fail: false,
    };
    let checkpoint = ProjectionCheckpoint::new(1);
    let future = projection.apply_once(&event, &checkpoint);
    drop(future);
    assert!(!projection.applied);
}
