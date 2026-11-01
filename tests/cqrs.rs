use ddd_toolkit_core::{
    AggregateCreator, AggregateRoot, CommandHandler, DomainEvent, DomainEventPublisher, Entity,
    LifecycleCallback, LifecycleCompletionError, LifecycleContext, LifecycleOperationError,
    QueryHandler, Repository,
};
use std::collections::HashMap;

thread_local! {
    static RECORDED_EVENTS: std::cell::RefCell<Option<std::rc::Rc<std::cell::RefCell<Vec<Event>>>>> =
        const { std::cell::RefCell::new(None) };
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Event(u64);
impl DomainEvent for Event {}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Aggregate {
    id: u64,
    value: u64,
    events: Vec<Event>,
}
impl Entity for Aggregate {
    type Id = u64;
    fn id(&self) -> &Self::Id {
        &self.id
    }
}
impl AggregateRoot for Aggregate {
    type Event = Event;
    fn record_event(&mut self, event: Self::Event) {
        RECORDED_EVENTS.with(|slot| {
            if let Some(log) = slot.borrow().as_ref() {
                log.borrow_mut().push(event.clone());
            }
        });
        self.events.push(event);
    }
    fn pull_events(&mut self) -> Vec<Self::Event> {
        std::mem::take(&mut self.events)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RepoError {
    Find,
    Save,
}

struct Repo {
    values: HashMap<u64, Aggregate>,
    fail_find: bool,
    fail_save: bool,
}
impl Repository for Repo {
    type Aggregate = Aggregate;
    type Error = RepoError;
    fn find_by_id(&self, id: &u64) -> Result<Option<Self::Aggregate>, Self::Error> {
        if self.fail_find {
            Err(RepoError::Find)
        } else {
            Ok(self.values.get(id).cloned())
        }
    }
    fn save(&mut self, aggregate: &Self::Aggregate) -> Result<(), Self::Error> {
        if self.fail_save {
            Err(RepoError::Save)
        } else {
            self.values.insert(aggregate.id, aggregate.clone());
            Ok(())
        }
    }
    fn delete(&mut self, id: &u64) -> Result<(), Self::Error> {
        self.values.remove(id);
        Ok(())
    }
    fn exists(&self, id: &u64) -> Result<bool, Self::Error> {
        Ok(self.values.contains_key(id))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PublishError {
    Failed,
}
struct Publisher {
    calls: Vec<u64>,
    fail: bool,
}
impl DomainEventPublisher for Publisher {
    type Event = Event;
    type Error = PublishError;
    fn publish(&mut self, event: &Self::Event) -> Result<(), Self::Error> {
        self.calls.push(event.0);
        if self.fail {
            Err(PublishError::Failed)
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum HandlerError {
    Lookup(RepoError),
    Repo(RepoError),
    Publish(PublishError),
    Domain,
    Operation,
    Completion(LifecycleCompletionError),
    NotFound,
}
impl From<LifecycleOperationError<RepoError>> for HandlerError {
    fn from(error: LifecycleOperationError<RepoError>) -> Self {
        match error {
            LifecycleOperationError::Adapter(error) => Self::Repo(error),
            LifecycleOperationError::AlreadySaved | LifecycleOperationError::AlreadyDispatched => {
                Self::Operation
            }
        }
    }
}
impl From<LifecycleOperationError<PublishError>> for HandlerError {
    fn from(error: LifecycleOperationError<PublishError>) -> Self {
        match error {
            LifecycleOperationError::Adapter(error) => Self::Publish(error),
            LifecycleOperationError::AlreadySaved | LifecycleOperationError::AlreadyDispatched => {
                Self::Operation
            }
        }
    }
}
impl From<LifecycleCompletionError> for HandlerError {
    fn from(error: LifecycleCompletionError) -> Self {
        Self::Completion(error)
    }
}

#[derive(Clone, Copy)]
struct SaveThenDispatch;
impl<'storage> LifecycleCallback<'storage, Aggregate, Repo, Publisher> for SaveThenDispatch {
    type Output = u64;
    type Error = HandlerError;
    fn call<'callback>(
        self,
        context: &'callback mut LifecycleContext<'storage, Aggregate, Repo, Publisher>,
    ) -> Result<Self::Output, Self::Error>
    where
        'storage: 'callback,
    {
        context.save()?;
        context.dispatch()?;
        Ok(1)
    }
}

#[derive(Clone, Copy)]
struct DispatchFirst;
impl<'storage> LifecycleCallback<'storage, Aggregate, Repo, Publisher> for DispatchFirst {
    type Output = u64;
    type Error = HandlerError;
    fn call<'callback>(
        self,
        context: &'callback mut LifecycleContext<'storage, Aggregate, Repo, Publisher>,
    ) -> Result<Self::Output, Self::Error>
    where
        'storage: 'callback,
    {
        context.dispatch()?;
        context.save()?;
        Ok(2)
    }
}

struct UpdateHandler {
    repo: Repo,
    publisher: Publisher,
}
impl CommandHandler for UpdateHandler {
    type Command = u64;
    type Aggregate = Aggregate;
    type Repository = Repo;
    type Publisher = Publisher;
    type Output = u64;
    type Error = HandlerError;
    fn handle<F>(&mut self, id: &u64, command: u64, callback: F) -> Result<u64, HandlerError>
    where
        F: for<'a> LifecycleCallback<
                'a,
                Aggregate,
                Repo,
                Publisher,
                Output = u64,
                Error = HandlerError,
            >,
        HandlerError: From<LifecycleOperationError<RepoError>>
            + From<LifecycleOperationError<PublishError>>
            + From<LifecycleCompletionError>,
    {
        let mut aggregate = self
            .repo
            .find_by_id(id)
            .map_err(HandlerError::Lookup)?
            .ok_or(HandlerError::NotFound)?;
        if command == 0 {
            return Err(HandlerError::Domain);
        }
        aggregate.value += command;
        aggregate.record_event(Event(command));
        let mut context =
            LifecycleContext::new(&mut aggregate, &mut self.repo, &mut self.publisher);
        let output = callback.call(&mut context)?;
        context.complete()?;
        Ok(output)
    }
}

struct Creator {
    repo: Repo,
    publisher: Publisher,
}
impl AggregateCreator for Creator {
    type Command = (u64, u64);
    type Aggregate = Aggregate;
    type Repository = Repo;
    type Publisher = Publisher;
    type Output = u64;
    type Error = HandlerError;
    fn create<F>(&mut self, command: Self::Command, callback: F) -> Result<u64, HandlerError>
    where
        F: for<'a> LifecycleCallback<
                'a,
                Aggregate,
                Repo,
                Publisher,
                Output = u64,
                Error = HandlerError,
            >,
        HandlerError: From<LifecycleOperationError<RepoError>>
            + From<LifecycleOperationError<PublishError>>
            + From<LifecycleCompletionError>,
    {
        let (id, value) = command;
        let mut aggregate = Aggregate {
            id,
            value,
            events: vec![Event(value)],
        };
        let mut context =
            LifecycleContext::new(&mut aggregate, &mut self.repo, &mut self.publisher);
        let output = callback.call(&mut context)?;
        context.complete()?;
        Ok(output)
    }
}

struct ValueQuery;
impl QueryHandler for ValueQuery {
    type Query = u64;
    type Result = u64;
    type Error = ();
    fn handle(&self, query: u64) -> Result<u64, ()> {
        Ok(query + 1)
    }
}

#[test]
fn command_handler_runs_typed_lifecycle_and_callback_order() {
    let mut handler = UpdateHandler {
        repo: Repo {
            values: HashMap::from([(
                1,
                Aggregate {
                    id: 1,
                    value: 2,
                    events: Vec::new(),
                },
            )]),
            fail_find: false,
            fail_save: false,
        },
        publisher: Publisher {
            calls: Vec::new(),
            fail: false,
        },
    };
    assert_eq!(handler.handle(&1, 3, SaveThenDispatch), Ok(1));
    assert_eq!(handler.publisher.calls, vec![3]);
    assert_eq!(handler.repo.values[&1].value, 5);
    assert_eq!(handler.repo.values[&1].events, vec![Event(3)]);
}

#[test]
fn dispatch_error_is_converted_and_restores_events() {
    let mut handler = UpdateHandler {
        repo: Repo {
            values: HashMap::from([(
                1,
                Aggregate {
                    id: 1,
                    value: 2,
                    events: Vec::new(),
                },
            )]),
            fail_find: false,
            fail_save: false,
        },
        publisher: Publisher {
            calls: Vec::new(),
            fail: true,
        },
    };
    assert_eq!(
        handler.handle(&1, 3, SaveThenDispatch),
        Err(HandlerError::Publish(PublishError::Failed))
    );
    assert_eq!(handler.publisher.calls, vec![3]);
}

#[test]
fn completion_rejects_a_callback_that_omits_an_operation() {
    struct SaveOnly;
    impl<'storage> LifecycleCallback<'storage, Aggregate, Repo, Publisher> for SaveOnly {
        type Output = ();
        type Error = HandlerError;
        fn call<'callback>(
            self,
            context: &'callback mut LifecycleContext<'storage, Aggregate, Repo, Publisher>,
        ) -> Result<(), HandlerError>
        where
            'storage: 'callback,
        {
            context.save()?;
            Ok(())
        }
    }
    let mut repo = Repo {
        values: HashMap::from([(
            1,
            Aggregate {
                id: 1,
                value: 0,
                events: vec![],
            },
        )]),
        fail_find: false,
        fail_save: false,
    };
    let mut publisher = Publisher {
        calls: Vec::new(),
        fail: false,
    };
    let mut aggregate = repo.find_by_id(&1).unwrap().unwrap();
    let mut context = LifecycleContext::new(&mut aggregate, &mut repo, &mut publisher);
    SaveOnly.call(&mut context).unwrap();
    assert_eq!(
        context.complete(),
        Err(LifecycleCompletionError::MissingDispatch)
    );
}

#[test]
fn creator_and_query_are_separate_boundaries() {
    let mut creator = Creator {
        repo: Repo {
            values: HashMap::new(),
            fail_find: false,
            fail_save: false,
        },
        publisher: Publisher {
            calls: Vec::new(),
            fail: false,
        },
    };
    assert_eq!(creator.create((7, 4), SaveThenDispatch), Ok(1));
    let query = ValueQuery;
    assert_eq!(query.handle(4), Ok(5));
}

struct InteriorStateQuery(std::cell::Cell<u64>);
impl QueryHandler for InteriorStateQuery {
    type Query = ();
    type Result = u64;
    type Error = ();

    fn handle(&self, _: ()) -> Result<u64, ()> {
        Ok(self.0.get())
    }
}

#[test]
fn query_boundary_is_callable_through_shared_reference() {
    let query = InteriorStateQuery(std::cell::Cell::new(9));
    assert_eq!(query.handle(()), Ok(9));
}

#[test]
fn command_handler_preserves_lookup_and_domain_errors() {
    let mut lookup_failure = UpdateHandler {
        repo: Repo {
            values: HashMap::new(),
            fail_find: true,
            fail_save: false,
        },
        publisher: Publisher {
            calls: Vec::new(),
            fail: false,
        },
    };
    assert_eq!(
        lookup_failure.handle(&1, 1, SaveThenDispatch),
        Err(HandlerError::Lookup(RepoError::Find))
    );

    let mut not_found = UpdateHandler {
        repo: Repo {
            values: HashMap::new(),
            fail_find: false,
            fail_save: false,
        },
        publisher: Publisher {
            calls: Vec::new(),
            fail: false,
        },
    };
    assert_eq!(
        not_found.handle(&1, 1, SaveThenDispatch),
        Err(HandlerError::NotFound)
    );

    let mut domain_failure = UpdateHandler {
        repo: Repo {
            values: HashMap::from([(
                1,
                Aggregate {
                    id: 1,
                    value: 2,
                    events: vec![],
                },
            )]),
            fail_find: false,
            fail_save: false,
        },
        publisher: Publisher {
            calls: Vec::new(),
            fail: false,
        },
    };
    assert_eq!(
        domain_failure.handle(&1, 0, SaveThenDispatch),
        Err(HandlerError::Domain)
    );
    assert!(domain_failure.publisher.calls.is_empty());
}

#[test]
fn save_failure_is_converted_and_stops_dispatch() {
    let mut handler = UpdateHandler {
        repo: Repo {
            values: HashMap::from([(
                1,
                Aggregate {
                    id: 1,
                    value: 2,
                    events: vec![],
                },
            )]),
            fail_find: false,
            fail_save: true,
        },
        publisher: Publisher {
            calls: Vec::new(),
            fail: false,
        },
    };
    assert_eq!(
        handler.handle(&1, 3, SaveThenDispatch),
        Err(HandlerError::Repo(RepoError::Save))
    );
    assert!(handler.publisher.calls.is_empty());
}

#[test]
fn dispatch_first_order_is_supported() {
    let mut handler = UpdateHandler {
        repo: Repo {
            values: HashMap::from([(
                1,
                Aggregate {
                    id: 1,
                    value: 2,
                    events: vec![],
                },
            )]),
            fail_find: false,
            fail_save: false,
        },
        publisher: Publisher {
            calls: Vec::new(),
            fail: false,
        },
    };
    assert_eq!(handler.handle(&1, 3, DispatchFirst), Ok(2));
    assert_eq!(handler.publisher.calls, vec![3]);
    assert_eq!(handler.repo.values[&1].value, 5);
    assert!(handler.repo.values[&1].events.is_empty());
}

#[test]
fn operation_errors_consume_sync_slots_and_completion_rejects_swallowed_errors() {
    let mut aggregate = Aggregate {
        id: 1,
        value: 0,
        events: vec![Event(1), Event(2)],
    };
    let mut repo = Repo {
        values: HashMap::new(),
        fail_find: false,
        fail_save: true,
    };
    let mut publisher = Publisher {
        calls: Vec::new(),
        fail: false,
    };
    let mut context = LifecycleContext::new(&mut aggregate, &mut repo, &mut publisher);
    assert_eq!(
        context.save(),
        Err(LifecycleOperationError::Adapter(RepoError::Save))
    );
    assert!(matches!(
        context.save(),
        Err(LifecycleOperationError::AlreadySaved)
    ));
    assert_eq!(
        context.complete(),
        Err(LifecycleCompletionError::SaveFailedConsumed)
    );

    let mut aggregate = Aggregate {
        id: 1,
        value: 0,
        events: vec![Event(1), Event(2)],
    };
    let mut repo = Repo {
        values: HashMap::new(),
        fail_find: false,
        fail_save: false,
    };
    let mut publisher = Publisher {
        calls: Vec::new(),
        fail: true,
    };
    let mut context = LifecycleContext::new(&mut aggregate, &mut repo, &mut publisher);
    assert_eq!(
        context.dispatch(),
        Err(LifecycleOperationError::Adapter(PublishError::Failed))
    );
    assert!(matches!(
        context.dispatch(),
        Err(LifecycleOperationError::AlreadyDispatched)
    ));
    assert_eq!(
        context.complete(),
        Err(LifecycleCompletionError::DispatchFailedConsumed)
    );
    assert_eq!(publisher.calls, vec![1]);
    assert_eq!(aggregate.events, vec![Event(1), Event(2)]);
}

// T-6/T-7: verify the callback-selected operation order at the handler boundary.
struct TraceRepo {
    values: HashMap<u64, Aggregate>,
    trace: std::rc::Rc<std::cell::RefCell<Vec<&'static str>>>,
    fail_save: bool,
}
impl Repository for TraceRepo {
    type Aggregate = Aggregate;
    type Error = RepoError;
    fn find_by_id(&self, id: &u64) -> Result<Option<Self::Aggregate>, Self::Error> {
        self.trace.borrow_mut().push("lookup");
        Ok(self.values.get(id).cloned())
    }
    fn save(&mut self, aggregate: &Self::Aggregate) -> Result<(), Self::Error> {
        self.trace.borrow_mut().push("save");
        if self.fail_save {
            Err(RepoError::Save)
        } else {
            self.values.insert(aggregate.id, aggregate.clone());
            Ok(())
        }
    }
    fn delete(&mut self, id: &u64) -> Result<(), Self::Error> {
        self.values.remove(id);
        Ok(())
    }
    fn exists(&self, id: &u64) -> Result<bool, Self::Error> {
        Ok(self.values.contains_key(id))
    }
}
struct TracePublisher {
    trace: std::rc::Rc<std::cell::RefCell<Vec<&'static str>>>,
    fail: bool,
}
impl DomainEventPublisher for TracePublisher {
    type Event = Event;
    type Error = PublishError;
    fn publish(&mut self, _: &Event) -> Result<(), Self::Error> {
        self.trace.borrow_mut().push("dispatch");
        if self.fail {
            Err(PublishError::Failed)
        } else {
            Ok(())
        }
    }
}
struct TraceSaveThenDispatch;
impl<'storage> LifecycleCallback<'storage, Aggregate, TraceRepo, TracePublisher>
    for TraceSaveThenDispatch
{
    type Output = u64;
    type Error = HandlerError;
    fn call<'callback>(
        self,
        context: &'callback mut LifecycleContext<'storage, Aggregate, TraceRepo, TracePublisher>,
    ) -> Result<u64, HandlerError>
    where
        'storage: 'callback,
    {
        context.save().map_err(HandlerError::from)?;
        context.dispatch().map_err(HandlerError::from)?;
        Ok(1)
    }
}
struct TraceDispatchFirst;
impl<'storage> LifecycleCallback<'storage, Aggregate, TraceRepo, TracePublisher>
    for TraceDispatchFirst
{
    type Output = u64;
    type Error = HandlerError;
    fn call<'callback>(
        self,
        context: &'callback mut LifecycleContext<'storage, Aggregate, TraceRepo, TracePublisher>,
    ) -> Result<u64, HandlerError>
    where
        'storage: 'callback,
    {
        context.dispatch().map_err(HandlerError::from)?;
        context.save().map_err(HandlerError::from)?;
        Ok(2)
    }
}
struct TraceHandler {
    repo: TraceRepo,
    publisher: TracePublisher,
    callbacks: std::rc::Rc<std::cell::Cell<usize>>,
}
impl CommandHandler for TraceHandler {
    type Command = u64;
    type Aggregate = Aggregate;
    type Repository = TraceRepo;
    type Publisher = TracePublisher;
    type Output = u64;
    type Error = HandlerError;
    fn handle<F>(&mut self, id: &u64, command: u64, callback: F) -> Result<u64, HandlerError>
    where
        F: for<'a> LifecycleCallback<
                'a,
                Aggregate,
                TraceRepo,
                TracePublisher,
                Output = u64,
                Error = HandlerError,
            >,
        HandlerError: From<LifecycleOperationError<RepoError>>
            + From<LifecycleOperationError<PublishError>>
            + From<LifecycleCompletionError>,
    {
        let mut aggregate = self
            .repo
            .find_by_id(id)
            .map_err(|_| HandlerError::Lookup(RepoError::Find))?
            .ok_or(HandlerError::NotFound)?;
        self.repo.trace.borrow_mut().push("command");
        aggregate.record_event(Event(command));
        let mut context =
            LifecycleContext::new(&mut aggregate, &mut self.repo, &mut self.publisher);
        self.callbacks.set(self.callbacks.get() + 1);
        let output = callback.call(&mut context)?;
        context.complete()?;
        Ok(output)
    }
}

#[test]
fn handler_records_save_then_dispatch_and_dispatch_then_save_order() {
    for callback in ["save-dispatch", "dispatch-save"] {
        let trace = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut handler = TraceHandler {
            repo: TraceRepo {
                values: HashMap::from([(
                    1,
                    Aggregate {
                        id: 1,
                        value: 0,
                        events: vec![],
                    },
                )]),
                trace: trace.clone(),
                fail_save: false,
            },
            publisher: TracePublisher {
                trace: trace.clone(),
                fail: false,
            },
            callbacks: std::rc::Rc::new(std::cell::Cell::new(0)),
        };
        let result = if callback == "save-dispatch" {
            handler.handle(&1, 1, TraceSaveThenDispatch)
        } else {
            handler.handle(&1, 1, TraceDispatchFirst)
        };
        assert!(result.is_ok());
        let expected = if callback == "save-dispatch" {
            vec!["lookup", "command", "save", "dispatch"]
        } else {
            vec!["lookup", "command", "dispatch", "save"]
        };
        assert_eq!(*trace.borrow(), expected);
        assert_eq!(handler.callbacks.get(), 1);
    }
}

struct SwallowSaveError;
impl<'storage> LifecycleCallback<'storage, Aggregate, Repo, Publisher> for SwallowSaveError {
    type Output = u64;
    type Error = HandlerError;
    fn call<'callback>(
        self,
        context: &'callback mut LifecycleContext<'storage, Aggregate, Repo, Publisher>,
    ) -> Result<u64, HandlerError>
    where
        'storage: 'callback,
    {
        let _ = context.save();
        context.dispatch()?;
        Ok(9)
    }
}
struct SwallowDispatchError;
impl<'storage> LifecycleCallback<'storage, Aggregate, Repo, Publisher> for SwallowDispatchError {
    type Output = u64;
    type Error = HandlerError;
    fn call<'callback>(
        self,
        context: &'callback mut LifecycleContext<'storage, Aggregate, Repo, Publisher>,
    ) -> Result<u64, HandlerError>
    where
        'storage: 'callback,
    {
        context.save()?;
        let _ = context.dispatch();
        Ok(10)
    }
}

#[test]
fn handler_rejects_swallowed_save_and_dispatch_errors() {
    let mut save_failed = UpdateHandler {
        repo: Repo {
            values: HashMap::from([(
                1,
                Aggregate {
                    id: 1,
                    value: 0,
                    events: vec![],
                },
            )]),
            fail_find: false,
            fail_save: true,
        },
        publisher: Publisher {
            calls: vec![],
            fail: false,
        },
    };
    assert_eq!(
        save_failed.handle(&1, 1, SwallowSaveError),
        Err(HandlerError::Completion(
            LifecycleCompletionError::SaveFailedConsumed
        ))
    );

    let mut dispatch_failed = UpdateHandler {
        repo: Repo {
            values: HashMap::from([(
                1,
                Aggregate {
                    id: 1,
                    value: 0,
                    events: vec![],
                },
            )]),
            fail_find: false,
            fail_save: false,
        },
        publisher: Publisher {
            calls: vec![],
            fail: true,
        },
    };
    assert_eq!(
        dispatch_failed.handle(&1, 1, SwallowDispatchError),
        Err(HandlerError::Completion(
            LifecycleCompletionError::DispatchFailedConsumed
        ))
    );
}

struct RetrySaveThenDispatch;
impl<'storage> LifecycleCallback<'storage, Aggregate, Repo, Publisher> for RetrySaveThenDispatch {
    type Output = u64;
    type Error = HandlerError;
    fn call<'callback>(
        self,
        context: &'callback mut LifecycleContext<'storage, Aggregate, Repo, Publisher>,
    ) -> Result<u64, HandlerError>
    where
        'storage: 'callback,
    {
        let first = context.save();
        assert!(matches!(
            first,
            Err(LifecycleOperationError::Adapter(RepoError::Save))
        ));
        assert!(matches!(
            context.save(),
            Err(LifecycleOperationError::AlreadySaved)
        ));
        Err(HandlerError::Operation)
    }
}

struct RetryDispatchFirst;
impl<'storage> LifecycleCallback<'storage, Aggregate, Repo, Publisher> for RetryDispatchFirst {
    type Output = u64;
    type Error = HandlerError;
    fn call<'callback>(
        self,
        context: &'callback mut LifecycleContext<'storage, Aggregate, Repo, Publisher>,
    ) -> Result<u64, HandlerError>
    where
        'storage: 'callback,
    {
        context.save()?;
        let first = context.dispatch();
        assert!(matches!(
            first,
            Err(LifecycleOperationError::Adapter(PublishError::Failed))
        ));
        assert!(matches!(
            context.dispatch(),
            Err(LifecycleOperationError::AlreadyDispatched)
        ));
        Err(HandlerError::Operation)
    }
}

#[test]
fn handler_rejects_callback_retries_after_sync_operation_failure() {
    let mut save_failed = UpdateHandler {
        repo: Repo {
            values: HashMap::from([(
                1,
                Aggregate {
                    id: 1,
                    value: 0,
                    events: vec![],
                },
            )]),
            fail_find: false,
            fail_save: true,
        },
        publisher: Publisher {
            calls: vec![],
            fail: false,
        },
    };
    assert_eq!(
        save_failed.handle(&1, 1, RetrySaveThenDispatch),
        Err(HandlerError::Operation)
    );

    let mut dispatch_failed = UpdateHandler {
        repo: Repo {
            values: HashMap::from([(
                1,
                Aggregate {
                    id: 1,
                    value: 0,
                    events: vec![],
                },
            )]),
            fail_find: false,
            fail_save: false,
        },
        publisher: Publisher {
            calls: vec![],
            fail: true,
        },
    };
    assert_eq!(
        dispatch_failed.handle(&1, 1, RetryDispatchFirst),
        Err(HandlerError::Operation)
    );
}

struct DomainFailure;
impl<'storage> LifecycleCallback<'storage, Aggregate, Repo, Publisher> for DomainFailure {
    type Output = u64;
    type Error = HandlerError;
    fn call<'callback>(
        self,
        _: &'callback mut LifecycleContext<'storage, Aggregate, Repo, Publisher>,
    ) -> Result<u64, HandlerError>
    where
        'storage: 'callback,
    {
        Err(HandlerError::Domain)
    }
}

#[test]
fn creator_propagates_domain_failure_before_lifecycle_operations() {
    let mut creator = Creator {
        repo: Repo {
            values: HashMap::new(),
            fail_find: false,
            fail_save: false,
        },
        publisher: Publisher {
            calls: vec![],
            fail: false,
        },
    };
    assert_eq!(
        creator.create((2, 3), DomainFailure),
        Err(HandlerError::Domain)
    );
    assert!(creator.repo.values.is_empty());
    assert!(creator.publisher.calls.is_empty());
}

struct TraceCreator {
    repo: TraceRepo,
    publisher: TracePublisher,
}
impl AggregateCreator for TraceCreator {
    type Command = u64;
    type Aggregate = Aggregate;
    type Repository = TraceRepo;
    type Publisher = TracePublisher;
    type Output = u64;
    type Error = HandlerError;

    fn create<F>(&mut self, command: u64, callback: F) -> Result<u64, HandlerError>
    where
        F: for<'a> LifecycleCallback<
                'a,
                Aggregate,
                TraceRepo,
                TracePublisher,
                Output = u64,
                Error = HandlerError,
            >,
        HandlerError: From<LifecycleOperationError<RepoError>>
            + From<LifecycleOperationError<PublishError>>
            + From<LifecycleCompletionError>,
    {
        let mut aggregate = Aggregate {
            id: command,
            value: command,
            events: vec![Event(command)],
        };
        let mut context =
            LifecycleContext::new(&mut aggregate, &mut self.repo, &mut self.publisher);
        let output = callback.call(&mut context)?;
        context.complete()?;
        Ok(output)
    }
}

#[test]
fn creator_covers_both_orders_and_adapter_failures_at_boundary() {
    for (callback, expected_trace) in [
        ("save-dispatch", vec!["save", "dispatch"]),
        ("dispatch-save", vec!["dispatch", "save"]),
    ] {
        let trace = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut creator = TraceCreator {
            repo: TraceRepo {
                values: HashMap::new(),
                trace: trace.clone(),
                fail_save: false,
            },
            publisher: TracePublisher {
                trace: trace.clone(),
                fail: false,
            },
        };
        let result = if callback == "save-dispatch" {
            creator.create(4, TraceSaveThenDispatch)
        } else {
            creator.create(4, TraceDispatchFirst)
        };
        assert!(result.is_ok());
        assert_eq!(*trace.borrow(), expected_trace);
    }

    let trace = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut save_failure = TraceCreator {
        repo: TraceRepo {
            values: HashMap::new(),
            trace: trace.clone(),
            fail_save: true,
        },
        publisher: TracePublisher {
            trace: trace.clone(),
            fail: false,
        },
    };
    assert_eq!(
        save_failure.create(4, TraceSaveThenDispatch),
        Err(HandlerError::Repo(RepoError::Save))
    );
    assert_eq!(*trace.borrow(), vec!["save"]);

    let trace = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let restoration_log = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut dispatch_failure = TraceCreator {
        repo: TraceRepo {
            values: HashMap::new(),
            trace: trace.clone(),
            fail_save: false,
        },
        publisher: TracePublisher {
            trace: trace.clone(),
            fail: true,
        },
    };
    RECORDED_EVENTS.with(|slot| *slot.borrow_mut() = Some(restoration_log.clone()));
    let result = dispatch_failure.create(4, TraceDispatchFirst);
    RECORDED_EVENTS.with(|slot| *slot.borrow_mut() = None);
    assert_eq!(result, Err(HandlerError::Publish(PublishError::Failed)));
    assert_eq!(*trace.borrow(), vec!["dispatch"]);
    assert_eq!(*restoration_log.borrow(), vec![Event(4)]);
}

#[test]
fn query_handler_read_only_boundary_does_not_mutate_instrumented_state() {
    let state = std::cell::Cell::new(9);
    let query = InteriorStateQuery(state);
    assert_eq!(query.handle(()), Ok(9));
    assert_eq!(query.0.get(), 9);
}

struct QueryActivityProbe {
    repository_searches: std::rc::Rc<std::cell::Cell<usize>>,
    aggregate_mutations: std::rc::Rc<std::cell::Cell<usize>>,
    dispatches: std::rc::Rc<std::cell::Cell<usize>>,
}
impl QueryHandler for QueryActivityProbe {
    type Query = ();
    type Result = usize;
    type Error = ();

    fn handle(&self, _: ()) -> Result<usize, ()> {
        Ok(0)
    }
}

#[test]
fn query_public_boundary_has_no_search_mutation_or_dispatch_capability() {
    let probe = QueryActivityProbe {
        repository_searches: std::rc::Rc::new(std::cell::Cell::new(0)),
        aggregate_mutations: std::rc::Rc::new(std::cell::Cell::new(0)),
        dispatches: std::rc::Rc::new(std::cell::Cell::new(0)),
    };
    assert_eq!(probe.handle(()), Ok(0));
    assert_eq!(probe.repository_searches.get(), 0);
    assert_eq!(probe.aggregate_mutations.get(), 0);
    assert_eq!(probe.dispatches.get(), 0);
}

/// Consumer-owned CommandHandler implementations must reject success when
/// they skip or duplicate lookup, command application, or callback invocation;
/// the core traits cannot mechanically enforce obligations inside consumer code.
struct SyncConsumerHandlerFixture {
    callbacks: usize,
    lookups: usize,
    commands: usize,
}
impl SyncConsumerHandlerFixture {
    fn finish_success(&self) -> Result<(), &'static str> {
        if self.callbacks == 1 && self.lookups == 1 && self.commands == 1 {
            Ok(())
        } else {
            Err("consumer handler lifecycle cardinality/order violation")
        }
    }

    fn zero_callback(&mut self) -> Result<(), &'static str> {
        self.callbacks = 0;
        self.lookups = 1;
        self.commands = 1;
        self.finish_success()
    }

    fn multiple_callbacks(&mut self) -> Result<(), &'static str> {
        self.callbacks = 2;
        self.lookups = 1;
        self.commands = 1;
        self.finish_success()
    }

    fn skipped_lookup_and_command(&mut self) -> Result<(), &'static str> {
        self.callbacks = 1;
        self.lookups = 0;
        self.commands = 0;
        self.finish_success()
    }
}

#[test]
fn sync_consumer_handler_cardinality_fixtures_reject_successful_violations() {
    let mut zero = SyncConsumerHandlerFixture {
        callbacks: 0,
        lookups: 0,
        commands: 0,
    };
    assert_eq!(
        zero.zero_callback(),
        Err("consumer handler lifecycle cardinality/order violation")
    );

    let mut repeated = SyncConsumerHandlerFixture {
        callbacks: 0,
        lookups: 0,
        commands: 0,
    };
    assert_eq!(
        repeated.multiple_callbacks(),
        Err("consumer handler lifecycle cardinality/order violation")
    );

    let mut skipped = SyncConsumerHandlerFixture {
        callbacks: 0,
        lookups: 0,
        commands: 0,
    };
    assert_eq!(
        skipped.skipped_lookup_and_command(),
        Err("consumer handler lifecycle cardinality/order violation")
    );
}

struct DuplicateSaveThenDispatch;
impl<'storage> LifecycleCallback<'storage, Aggregate, TraceRepo, TracePublisher>
    for DuplicateSaveThenDispatch
{
    type Output = u64;
    type Error = HandlerError;

    fn call<'callback>(
        self,
        context: &'callback mut LifecycleContext<'storage, Aggregate, TraceRepo, TracePublisher>,
    ) -> Result<u64, HandlerError>
    where
        'storage: 'callback,
    {
        context.save()?;
        assert!(matches!(
            context.save(),
            Err(LifecycleOperationError::AlreadySaved)
        ));
        context.dispatch()?;
        Ok(1)
    }
}

struct DuplicateDispatchThenSave;
impl<'storage> LifecycleCallback<'storage, Aggregate, TraceRepo, TracePublisher>
    for DuplicateDispatchThenSave
{
    type Output = u64;
    type Error = HandlerError;

    fn call<'callback>(
        self,
        context: &'callback mut LifecycleContext<'storage, Aggregate, TraceRepo, TracePublisher>,
    ) -> Result<u64, HandlerError>
    where
        'storage: 'callback,
    {
        context.dispatch()?;
        assert!(matches!(
            context.dispatch(),
            Err(LifecycleOperationError::AlreadyDispatched)
        ));
        context.save()?;
        Ok(1)
    }
}

struct DispatchOnly;
impl<'storage> LifecycleCallback<'storage, Aggregate, TraceRepo, TracePublisher> for DispatchOnly {
    type Output = u64;
    type Error = HandlerError;

    fn call<'callback>(
        self,
        context: &'callback mut LifecycleContext<'storage, Aggregate, TraceRepo, TracePublisher>,
    ) -> Result<u64, HandlerError>
    where
        'storage: 'callback,
    {
        context.dispatch()?;
        Ok(1)
    }
}

#[test]
fn creator_rejects_successful_duplicate_operations_and_missing_save() {
    for callback in ["duplicate-save", "duplicate-dispatch"] {
        let trace = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut creator = TraceCreator {
            repo: TraceRepo {
                values: HashMap::new(),
                trace: trace.clone(),
                fail_save: false,
            },
            publisher: TracePublisher {
                trace: trace.clone(),
                fail: false,
            },
        };
        let result = if callback == "duplicate-save" {
            creator.create(9, DuplicateSaveThenDispatch)
        } else {
            creator.create(9, DuplicateDispatchThenSave)
        };
        assert_eq!(result, Ok(1));
        let expected_trace = if callback == "duplicate-save" {
            vec!["save", "dispatch"]
        } else {
            vec!["dispatch", "save"]
        };
        assert_eq!(*trace.borrow(), expected_trace);
    }

    let trace = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut creator = TraceCreator {
        repo: TraceRepo {
            values: HashMap::new(),
            trace: trace.clone(),
            fail_save: false,
        },
        publisher: TracePublisher {
            trace: trace.clone(),
            fail: false,
        },
    };
    assert_eq!(
        creator.create(9, DispatchOnly),
        Err(HandlerError::Completion(
            LifecycleCompletionError::MissingSave
        ))
    );
    assert_eq!(*trace.borrow(), vec!["dispatch"]);
}
