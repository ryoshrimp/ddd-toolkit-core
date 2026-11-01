use ddd_toolkit_core::{
    DomainEvent, EventConsumer, IdempotentProjection, Projection, ProjectionApplication,
    ProjectionCheckpoint, ProjectionCheckpointStore,
};

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
