use crate::{domain::AggregateRoot, port::PortError};

#[trait_variant::make(Send)]
pub trait Load<A: AggregateRoot> {
    async fn load(&self, id: &A::Id) -> Result<Option<A>, PortError>;
}

#[trait_variant::make(Send)]
pub trait Save<A: AggregateRoot> {
    async fn save(&self, aggregate: &mut A) -> Result<(), PortError>;
}

#[trait_variant::make(Send)]
pub trait Delete<A: AggregateRoot> {
    async fn delete(&self, id: &A::Id) -> Result<(), PortError>;
}

#[cfg(test)]
mod test {
    use std::{
        collections::HashMap,
        fmt::Display,
        future::Future,
        pin::pin,
        sync::Mutex,
        task::{Context, Poll, Waker},
    };

    use crate::{
        domain::{DomainEvent, Entity, EntityId, ValueObject},
        port::PortErrorKind,
    };

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

    #[derive(Debug)]
    struct Foo {
        id: FooId,
        name: String,
        events: Vec<FooEvent>,
    }

    impl Foo {
        fn new(id: &str, name: &str) -> Self {
            Self {
                id: FooId(id.to_string()),
                name: name.to_string(),
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

    struct InMemoryFooRepository {
        store: Mutex<HashMap<FooId, String>>,
        saved_events: Mutex<Vec<FooEvent>>,
    }

    impl InMemoryFooRepository {
        fn new() -> Self {
            Self {
                store: Mutex::new(HashMap::new()),
                saved_events: Mutex::new(Vec::new()),
            }
        }
    }

    impl Load<Foo> for InMemoryFooRepository {
        async fn load(&self, id: &FooId) -> Result<Option<Foo>, PortError> {
            Ok(self.store.lock().unwrap().get(id).map(|name| Foo {
                id: id.clone(),
                name: name.clone(),
                events: Vec::new(),
            }))
        }
    }

    impl Save<Foo> for InMemoryFooRepository {
        async fn save(&self, aggregate: &mut Foo) -> Result<(), PortError> {
            self.store
                .lock()
                .unwrap()
                .insert(aggregate.id().clone(), aggregate.name.clone());
            self.saved_events
                .lock()
                .unwrap()
                .extend(aggregate.take_events());
            Ok(())
        }
    }

    impl Delete<Foo> for InMemoryFooRepository {
        async fn delete(&self, id: &FooId) -> Result<(), PortError> {
            self.store.lock().unwrap().remove(id);
            Ok(())
        }
    }

    struct FailingRepository;

    impl Load<Foo> for FailingRepository {
        async fn load(&self, _id: &FooId) -> Result<Option<Foo>, PortError> {
            Err(PortError::unavailable("load failed"))
        }
    }

    impl Save<Foo> for FailingRepository {
        async fn save(&self, _aggregate: &mut Foo) -> Result<(), PortError> {
            Err(PortError::conflict("save failed"))
        }
    }

    impl Delete<Foo> for FailingRepository {
        async fn delete(&self, _id: &FooId) -> Result<(), PortError> {
            Err(PortError::other("delete failed"))
        }
    }

    #[test]
    fn save_new_aggregate_load_returns_some() {
        let repo = InMemoryFooRepository::new();
        let mut foo = Foo::new("foo-1", "alice");

        block_on(repo.save(&mut foo)).expect("save should succeed");

        let loaded = block_on(repo.load(&FooId("foo-1".to_string())))
            .expect("load should succeed")
            .expect("saved aggregate should be found");
        assert_eq!(loaded.id(), &FooId("foo-1".to_string()));
        assert_eq!(loaded.name, "alice");
    }

    #[test]
    fn load_missing_id_returns_none() {
        let repo = InMemoryFooRepository::new();

        let loaded =
            block_on(repo.load(&FooId("missing".to_string()))).expect("load should succeed");

        assert!(loaded.is_none());
    }

    #[test]
    fn save_existing_aggregate_overwrites_state() {
        let repo = InMemoryFooRepository::new();
        let mut foo = Foo::new("foo-1", "alice");
        block_on(repo.save(&mut foo)).expect("first save should succeed");

        foo.name = "bob".to_string();
        block_on(repo.save(&mut foo)).expect("second save should succeed");

        let loaded = block_on(repo.load(&FooId("foo-1".to_string())))
            .expect("load should succeed")
            .expect("saved aggregate should be found");
        assert_eq!(loaded.name, "bob");
    }

    #[test]
    fn save_can_drain_events_via_mut_ref() {
        let repo = InMemoryFooRepository::new();
        let mut foo = Foo::new("foo-1", "alice");
        foo.record(FooEvent {
            name: "created".to_string(),
        });

        block_on(repo.save(&mut foo)).expect("save should succeed");

        assert_eq!(
            *repo.saved_events.lock().unwrap(),
            vec![FooEvent {
                name: "created".to_string()
            }]
        );
        assert_eq!(foo.take_events(), vec![]);
    }

    #[test]
    fn delete_existing_id_load_returns_none() {
        let repo = InMemoryFooRepository::new();
        let mut foo = Foo::new("foo-1", "alice");
        block_on(repo.save(&mut foo)).expect("save should succeed");

        block_on(repo.delete(&FooId("foo-1".to_string()))).expect("delete should succeed");

        let loaded = block_on(repo.load(&FooId("foo-1".to_string()))).expect("load should succeed");
        assert!(loaded.is_none());
    }

    #[test]
    fn generic_fn_accepts_combined_repository_bounds() {
        fn roundtrip<R>(repo: &R, foo: &mut Foo) -> Option<Foo>
        where
            R: Load<Foo> + Save<Foo> + Delete<Foo>,
        {
            block_on(async {
                repo.save(foo).await.expect("save should succeed");
                let loaded = repo.load(foo.id()).await.expect("load should succeed");
                repo.delete(foo.id()).await.expect("delete should succeed");
                loaded
            })
        }

        let repo = InMemoryFooRepository::new();
        let mut foo = Foo::new("foo-1", "alice");

        let loaded = roundtrip(&repo, &mut foo);

        assert_eq!(
            loaded.expect("saved aggregate should be found").name,
            "alice"
        );
        assert!(repo.store.lock().unwrap().is_empty());
    }

    #[test]
    fn load_propagates_port_error() {
        let repo = FailingRepository;

        let error = block_on(repo.load(&FooId("foo-1".to_string()))).expect_err("load should fail");

        assert_eq!(error.kind(), PortErrorKind::Unavailable);
    }

    #[test]
    fn save_propagates_port_error() {
        let repo = FailingRepository;
        let mut foo = Foo::new("foo-1", "alice");

        let error = block_on(repo.save(&mut foo)).expect_err("save should fail");

        assert_eq!(error.kind(), PortErrorKind::Conflict);
    }

    #[test]
    fn delete_propagates_port_error() {
        let repo = FailingRepository;

        let error =
            block_on(repo.delete(&FooId("foo-1".to_string()))).expect_err("delete should fail");

        assert_eq!(error.kind(), PortErrorKind::Other);
    }

    // Compile-time only (no runtime assertion): the generic bounds accept the
    // futures as `Send` purely because `#[trait_variant::make(Send)]` promises
    // it at the trait level, not because of the concrete impl.
    #[test]
    fn repository_futures_are_send() {
        fn assert_send(_: impl Send) {}

        fn check<R>(repo: &R, id: &FooId, foo: &mut Foo)
        where
            R: Load<Foo> + Save<Foo> + Delete<Foo>,
        {
            assert_send(repo.load(id));
            assert_send(repo.save(foo));
            assert_send(repo.delete(id));
        }

        let repo = InMemoryFooRepository::new();
        let mut foo = Foo::new("foo-1", "alice");

        check(&repo, &FooId("foo-1".to_string()), &mut foo);
    }

    // The traits leave delete-on-missing-id semantics to implementors; this
    // pins down the in-memory double's idempotent choice, not a port contract.
    #[test]
    fn delete_missing_id_is_ok() {
        let repo = InMemoryFooRepository::new();

        let result = block_on(repo.delete(&FooId("missing".to_string())));

        assert!(result.is_ok());
    }
}
