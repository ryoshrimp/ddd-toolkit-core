use std::collections::HashMap;

use ddd_toolkit_core::{AggregateRoot, DomainEvent, Entity, Repository};

#[derive(Clone, Debug, PartialEq, Eq)]
struct Renamed;

impl DomainEvent for Renamed {}

#[derive(Clone, Debug, PartialEq, Eq)]
struct User {
    id: u64,
    events: Vec<Renamed>,
}

impl Entity for User {
    type Id = u64;

    fn id(&self) -> &Self::Id {
        &self.id
    }
}

impl AggregateRoot for User {
    type Event = Renamed;

    fn record_event(&mut self, event: Self::Event) {
        self.events.push(event);
    }

    fn pull_events(&mut self) -> Vec<Self::Event> {
        std::mem::take(&mut self.events)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Operation {
    Find,
    Save,
    Delete,
    Exists,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RepositoryError {
    Find,
    Save,
    Delete,
    Exists,
}

struct InMemoryRepository {
    users: HashMap<u64, User>,
    failure: Option<Operation>,
}

impl InMemoryRepository {
    fn new() -> Self {
        Self {
            users: HashMap::new(),
            failure: None,
        }
    }

    fn fail_with(&mut self, operation: Operation) {
        self.failure = Some(operation);
    }

    fn should_fail(&self, operation: Operation) -> bool {
        self.failure == Some(operation)
    }
}

impl Repository for InMemoryRepository {
    type Aggregate = User;
    type Error = RepositoryError;

    fn find_by_id(
        &self,
        id: &<Self::Aggregate as Entity>::Id,
    ) -> Result<Option<Self::Aggregate>, Self::Error> {
        if self.should_fail(Operation::Find) {
            return Err(RepositoryError::Find);
        }
        Ok(self.users.get(id).cloned())
    }

    fn save(&mut self, aggregate: &Self::Aggregate) -> Result<(), Self::Error> {
        if self.should_fail(Operation::Save) {
            return Err(RepositoryError::Save);
        }
        self.users.insert(*aggregate.id(), aggregate.clone());
        Ok(())
    }

    fn delete(&mut self, id: &<Self::Aggregate as Entity>::Id) -> Result<(), Self::Error> {
        if self.should_fail(Operation::Delete) {
            return Err(RepositoryError::Delete);
        }
        self.users.remove(id);
        Ok(())
    }

    fn exists(&self, id: &u64) -> Result<bool, Self::Error> {
        if self.should_fail(Operation::Exists) {
            return Err(RepositoryError::Exists);
        }
        Ok(self.users.contains_key(id))
    }
}

fn user(id: u64) -> User {
    User { id, events: vec![] }
}

#[test]
fn save_then_find_returns_the_aggregate_without_taking_ownership() {
    let mut repository = InMemoryRepository::new();
    let aggregate = user(1);

    assert_eq!(repository.save(&aggregate), Ok(()));
    assert_eq!(aggregate.id(), &1);
    assert_eq!(repository.find_by_id(&1), Ok(Some(aggregate)));
}

#[test]
fn find_by_id_returns_none_for_an_unknown_id() {
    let repository = InMemoryRepository::new();

    assert_eq!(repository.find_by_id(&1), Ok(None));
}

#[test]
fn exists_reports_present_and_missing_ids() {
    let mut repository = InMemoryRepository::new();
    let aggregate = user(1);
    repository.save(&aggregate).unwrap();

    assert_eq!(repository.exists(&1), Ok(true));
    assert_eq!(repository.exists(&2), Ok(false));
}

#[test]
fn delete_removes_only_the_requested_aggregate() {
    let mut repository = InMemoryRepository::new();
    let first = user(1);
    let second = user(2);
    repository.save(&first).unwrap();
    repository.save(&second).unwrap();

    assert_eq!(repository.delete(&1), Ok(()));
    assert_eq!(repository.find_by_id(&1), Ok(None));
    assert_eq!(repository.exists(&2), Ok(true));
}

#[test]
fn operation_errors_are_returned_unchanged() {
    let mut repository = InMemoryRepository::new();
    let aggregate = user(1);

    repository.fail_with(Operation::Find);
    assert_eq!(repository.find_by_id(&1), Err(RepositoryError::Find));

    repository.fail_with(Operation::Save);
    assert_eq!(repository.save(&aggregate), Err(RepositoryError::Save));

    repository.fail_with(Operation::Delete);
    assert_eq!(repository.delete(&1), Err(RepositoryError::Delete));

    repository.fail_with(Operation::Exists);
    assert_eq!(repository.exists(&1), Err(RepositoryError::Exists));
}

#[test]
fn save_does_not_consume_pending_events() {
    let mut repository = InMemoryRepository::new();
    let mut aggregate = user(1);
    aggregate.record_event(Renamed);

    repository.save(&aggregate).unwrap();

    assert_eq!(aggregate.pull_events(), vec![Renamed]);
}
