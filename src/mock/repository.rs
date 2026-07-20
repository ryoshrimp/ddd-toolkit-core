use std::{collections::HashMap, sync::Mutex};

use crate::{
    domain::AggregateRoot,
    port::{
        PortError,
        repository::{Delete, Load, Save},
    },
};

#[derive(Debug)]
struct StoreState<A: AggregateRoot> {
    aggregates: HashMap<A::Id, A>,
    events: Vec<A::Event>,
}

#[derive(Debug)]
pub struct InMemoryStore<A: AggregateRoot> {
    state: Mutex<StoreState<A>>,
}

impl<A: AggregateRoot> InMemoryStore<A> {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(StoreState {
                aggregates: HashMap::new(),
                events: Vec::new(),
            }),
        }
    }

    pub fn take_recorded_events(&self) -> Vec<A::Event> {
        std::mem::take(&mut self.state.lock().unwrap_or_else(|e| e.into_inner()).events)
    }

    pub fn len(&self) -> usize {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .aggregates
            .len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<A: AggregateRoot> Default for InMemoryStore<A> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A: AggregateRoot + Clone> Load<A> for InMemoryStore<A> {
    async fn load(&self, id: &A::Id) -> Result<Option<A>, PortError> {
        Ok(self
            .state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .aggregates
            .get(id)
            .cloned())
    }
}

impl<A: AggregateRoot + Clone> Save<A> for InMemoryStore<A> {
    async fn save(&self, aggregate: &mut A) -> Result<(), PortError> {
        let events = aggregate.take_events();
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.events.extend(events);
        state
            .aggregates
            .insert(aggregate.id().clone(), aggregate.clone());
        Ok(())
    }
}

impl<A: AggregateRoot + Clone> Delete<A> for InMemoryStore<A> {
    async fn delete(&self, id: &A::Id) -> Result<(), PortError> {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .aggregates
            .remove(id);
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::fmt::Display;

    use crate::{
        domain::{DomainEvent, Entity, EntityId, ValueObject},
        testing::block_on,
    };

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

    #[derive(Debug, Clone, PartialEq)]
    struct FooEvent {
        name: String,
    }

    impl DomainEvent for FooEvent {}

    fn event(name: &str) -> FooEvent {
        FooEvent {
            name: name.to_string(),
        }
    }

    #[derive(Debug, Clone)]
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

    #[test]
    fn new_store_is_empty() {
        let store = InMemoryStore::<Foo>::new();

        assert_eq!(store.len(), 0);
        assert!(store.is_empty());
    }

    #[test]
    fn default_store_is_empty() {
        let store = InMemoryStore::<Foo>::default();

        assert_eq!(store.len(), 0);
        assert!(store.is_empty());
    }

    #[test]
    fn save_new_aggregate_load_returns_stored_state() {
        let store = InMemoryStore::new();
        let mut foo = Foo::new("foo-1", "alice");

        block_on(store.save(&mut foo)).expect("save should succeed");

        let loaded = block_on(store.load(&FooId("foo-1".to_string())))
            .expect("load should succeed")
            .expect("saved aggregate should be found");
        assert_eq!(loaded.id(), &FooId("foo-1".to_string()));
        assert_eq!(loaded.name, "alice");
    }

    #[test]
    fn load_missing_id_returns_none() {
        let store = InMemoryStore::<Foo>::new();

        let loaded =
            block_on(store.load(&FooId("missing".to_string()))).expect("load should succeed");

        assert!(loaded.is_none());
    }

    #[test]
    fn load_returns_clone_detached_from_store() {
        let store = InMemoryStore::new();
        let mut foo = Foo::new("foo-1", "alice");
        block_on(store.save(&mut foo)).expect("save should succeed");

        let mut loaded = block_on(store.load(&FooId("foo-1".to_string())))
            .expect("load should succeed")
            .expect("saved aggregate should be found");
        loaded.name = "bob".to_string();

        let reloaded = block_on(store.load(&FooId("foo-1".to_string())))
            .expect("load should succeed")
            .expect("saved aggregate should be found");
        assert_eq!(reloaded.name, "alice");
    }

    #[test]
    fn save_existing_id_overwrites_state() {
        let store = InMemoryStore::new();
        let mut foo = Foo::new("foo-1", "alice");
        block_on(store.save(&mut foo)).expect("first save should succeed");

        foo.name = "bob".to_string();
        block_on(store.save(&mut foo)).expect("second save should succeed");

        let loaded = block_on(store.load(&FooId("foo-1".to_string())))
            .expect("load should succeed")
            .expect("saved aggregate should be found");
        assert_eq!(loaded.name, "bob");
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn save_two_aggregates_len_counts_distinct_ids() {
        let store = InMemoryStore::new();

        block_on(store.save(&mut Foo::new("foo-1", "alice"))).expect("save should succeed");
        block_on(store.save(&mut Foo::new("foo-2", "bob"))).expect("save should succeed");

        assert_eq!(store.len(), 2);
        assert!(!store.is_empty());
    }

    #[test]
    fn save_drains_events_from_aggregate_into_store() {
        let store = InMemoryStore::new();
        let mut foo = Foo::new("foo-1", "alice");
        foo.record(event("created"));
        foo.record(event("updated"));

        block_on(store.save(&mut foo)).expect("save should succeed");

        assert_eq!(foo.take_events(), vec![]);
        assert_eq!(
            store.take_recorded_events(),
            vec![event("created"), event("updated")]
        );
    }

    // Pins the drain-before-clone order in `save`: the stored clone must not
    // carry pending events, or they would be recorded twice on the next save.
    #[test]
    fn save_stores_clone_without_pending_events() {
        let store = InMemoryStore::new();
        let mut foo = Foo::new("foo-1", "alice");
        foo.record(event("created"));

        block_on(store.save(&mut foo)).expect("save should succeed");

        let mut loaded = block_on(store.load(&FooId("foo-1".to_string())))
            .expect("load should succeed")
            .expect("saved aggregate should be found");
        assert_eq!(loaded.take_events(), vec![]);
    }

    #[test]
    fn saves_accumulate_events_in_order() {
        let store = InMemoryStore::new();
        let mut foo = Foo::new("foo-1", "alice");
        let mut bar = Foo::new("foo-2", "bob");

        foo.record(event("foo-created"));
        block_on(store.save(&mut foo)).expect("save should succeed");
        bar.record(event("bar-created"));
        block_on(store.save(&mut bar)).expect("save should succeed");
        foo.record(event("foo-updated"));
        block_on(store.save(&mut foo)).expect("save should succeed");

        assert_eq!(
            store.take_recorded_events(),
            vec![
                event("foo-created"),
                event("bar-created"),
                event("foo-updated")
            ]
        );
    }

    #[test]
    fn take_recorded_events_clears_buffer() {
        let store = InMemoryStore::new();
        let mut foo = Foo::new("foo-1", "alice");
        foo.record(event("created"));
        block_on(store.save(&mut foo)).expect("save should succeed");

        store.take_recorded_events();

        assert_eq!(store.take_recorded_events(), vec![]);
    }

    #[test]
    fn delete_existing_id_removes_aggregate() {
        let store = InMemoryStore::new();
        let mut foo = Foo::new("foo-1", "alice");
        block_on(store.save(&mut foo)).expect("save should succeed");

        block_on(store.delete(&FooId("foo-1".to_string()))).expect("delete should succeed");

        let loaded =
            block_on(store.load(&FooId("foo-1".to_string()))).expect("load should succeed");
        assert!(loaded.is_none());
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn debug_format_contains_type_name() {
        let store = InMemoryStore::<Foo>::new();

        assert!(format!("{store:?}").contains("InMemoryStore"));
    }

    #[test]
    fn take_recorded_events_on_fresh_store_returns_empty_vec() {
        let store = InMemoryStore::<Foo>::new();

        assert_eq!(store.take_recorded_events(), vec![]);
    }

    // Pins down the `Delete` port contract: deleting a missing id is
    // idempotent and must not be treated as an error.
    #[test]
    fn delete_missing_id_is_ok() {
        let store = InMemoryStore::<Foo>::new();

        let result = block_on(store.delete(&FooId("missing".to_string())));

        assert!(result.is_ok());
    }

    // A panic while the lock is held poisons the Mutex; subsequent calls
    // must recover the guard instead of panicking forever.
    #[test]
    fn store_recovers_from_poisoned_mutex() {
        let store = InMemoryStore::new();
        let mut foo = Foo::new("foo-1", "alice");
        block_on(store.save(&mut foo)).expect("save should succeed");

        let poisoned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = store.state.lock().unwrap();
            panic!("simulated panic while holding the lock");
        }));
        assert!(poisoned.is_err());
        assert!(store.state.is_poisoned());

        let loaded = block_on(store.load(&FooId("foo-1".to_string())))
            .expect("load should recover from the poisoned mutex")
            .expect("aggregate should still be present");
        assert_eq!(loaded.name, "alice");
    }
}
