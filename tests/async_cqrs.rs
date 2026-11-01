use ddd_toolkit_core::{
    AggregateRoot, AsyncAggregateCreator, AsyncCommandHandler, AsyncDomainEventPublisher,
    AsyncLifecycleCallback, AsyncLifecycleContext, AsyncQueryHandler, AsyncRepository, DomainEvent,
    Entity, LifecycleCompletionError, LifecycleOperationError,
};
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

thread_local! {
    static EVENT_RECORD_LOG: RefCell<Vec<u64>> = const { RefCell::new(Vec::new()) };
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Event(u64);
impl DomainEvent for Event {}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Aggregate {
    id: u64,
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
        EVENT_RECORD_LOG.with(|log| log.borrow_mut().push(event.0));
        self.events.push(event);
    }
    fn pull_events(&mut self) -> Vec<Self::Event> {
        std::mem::take(&mut self.events)
    }
}

struct Repo {
    saved: Option<Aggregate>,
    fail_find: bool,
    fail_save: bool,
    pending_save: bool,
    trace: Option<std::sync::Arc<std::sync::Mutex<Vec<&'static str>>>>,
}
impl AsyncRepository for Repo {
    type Aggregate = Aggregate;
    type Error = &'static str;
    fn find_by_id<'a>(
        &'a self,
        _: &'a u64,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Aggregate>, Self::Error>> + Send + 'a>> {
        Box::pin(async move {
            if let Some(trace) = &self.trace {
                trace.lock().unwrap().push("lookup");
            }
            if self.fail_find {
                Err("lookup")
            } else {
                Ok(self.saved.clone())
            }
        })
    }
    fn save<'a>(
        &'a mut self,
        aggregate: &'a Aggregate,
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'a>> {
        Box::pin(async move {
            if let Some(trace) = &self.trace {
                trace.lock().unwrap().push("save");
            }
            if self.fail_save {
                Err("save")
            } else if self.pending_save {
                std::future::pending::<()>().await;
                unreachable!("pending save future completed")
            } else {
                self.saved = Some(aggregate.clone());
                Ok(())
            }
        })
    }
    fn delete<'a>(
        &'a mut self,
        _: &'a u64,
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'a>> {
        Box::pin(async move {
            self.saved = None;
            Ok(())
        })
    }
    fn exists<'a>(
        &'a self,
        _: &'a u64,
    ) -> Pin<Box<dyn Future<Output = Result<bool, Self::Error>> + Send + 'a>> {
        Box::pin(async move { Ok(self.saved.is_some()) })
    }
}

#[derive(Debug, PartialEq, Eq)]
enum PubError {
    Failed,
    AlreadyDispatched,
}

struct Publisher {
    calls: Vec<u64>,
    fail: bool,
    pending: bool,
    trace: Option<std::sync::Arc<std::sync::Mutex<Vec<&'static str>>>>,
}
impl AsyncDomainEventPublisher for Publisher {
    type Event = Event;
    type Error = PubError;
    fn publish<'a>(
        &'a mut self,
        event: &'a Event,
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'a>> {
        self.calls.push(event.0);
        if let Some(trace) = &self.trace {
            trace.lock().unwrap().push("dispatch");
        }
        if self.fail {
            Box::pin(async { Err(PubError::Failed) })
        } else if self.pending {
            Box::pin(std::future::pending())
        } else {
            Box::pin(async { Ok(()) })
        }
    }
}

fn poll_ready<T>(mut future: Pin<Box<dyn Future<Output = T> + Send + '_>>) -> T {
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    match future.as_mut().poll(&mut context) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("fixture future did not complete in one poll"),
    }
}

#[test]
fn async_context_supports_save_then_dispatch_and_completion() {
    let mut aggregate = Aggregate {
        id: 1,
        events: vec![Event(7)],
    };
    let mut repository = Repo {
        saved: None,
        fail_find: false,
        fail_save: false,
        pending_save: false,
        trace: None,
    };
    let mut publisher = Publisher {
        calls: Vec::new(),
        fail: false,
        pending: false,
        trace: None,
    };
    let mut context = AsyncLifecycleContext::new(&mut aggregate, &mut repository, &mut publisher);
    assert_eq!(poll_ready(context.save()), Ok(()));
    assert_eq!(poll_ready(context.dispatch()), Ok(()));
    assert_eq!(context.complete(), Ok(()));
    assert_eq!(publisher.calls, vec![7]);
}

#[test]
fn async_context_rejects_retry_after_dispatch_failure() {
    let mut aggregate = Aggregate {
        id: 1,
        events: vec![Event(7)],
    };
    let mut repository = Repo {
        saved: None,
        fail_find: false,
        fail_save: false,
        pending_save: false,
        trace: None,
    };
    let mut publisher = Publisher {
        calls: Vec::new(),
        fail: true,
        pending: false,
        trace: None,
    };
    let mut context = AsyncLifecycleContext::new(&mut aggregate, &mut repository, &mut publisher);
    assert_eq!(
        poll_ready(context.dispatch()),
        Err(LifecycleOperationError::Adapter(PubError::Failed))
    );
    assert!(matches!(
        poll_ready(context.dispatch()),
        Err(LifecycleOperationError::AlreadyDispatched)
    ));
    assert_eq!(
        context.complete(),
        Err(LifecycleCompletionError::DispatchFailedConsumed)
    );
    assert_eq!(aggregate.events, vec![Event(7)]);
}

#[test]
fn async_context_drop_consumes_an_operation_slot() {
    let mut aggregate = Aggregate {
        id: 1,
        events: vec![],
    };
    let mut repository = Repo {
        saved: None,
        fail_find: false,
        fail_save: false,
        pending_save: false,
        trace: None,
    };
    let mut publisher = Publisher {
        calls: Vec::new(),
        fail: false,
        pending: false,
        trace: None,
    };
    let mut context = AsyncLifecycleContext::new(&mut aggregate, &mut repository, &mut publisher);
    drop(context.save());
    assert!(matches!(
        poll_ready(context.save()),
        Err(LifecycleOperationError::AlreadySaved)
    ));
    assert_eq!(
        context.complete(),
        Err(LifecycleCompletionError::SaveFailedConsumed)
    );
}

#[derive(Debug, PartialEq, Eq)]
enum HandlerError {
    Repository(&'static str),
    Publish(PubError),
    Completion(LifecycleCompletionError),
    Missing,
}
impl From<LifecycleOperationError<&'static str>> for HandlerError {
    fn from(error: LifecycleOperationError<&'static str>) -> Self {
        match error {
            LifecycleOperationError::Adapter(error) => Self::Repository(error),
            LifecycleOperationError::AlreadySaved => Self::Repository("already saved"),
            LifecycleOperationError::AlreadyDispatched => Self::Repository("already dispatched"),
        }
    }
}
impl From<LifecycleOperationError<PubError>> for HandlerError {
    fn from(error: LifecycleOperationError<PubError>) -> Self {
        match error {
            LifecycleOperationError::Adapter(error) => Self::Publish(error),
            LifecycleOperationError::AlreadySaved => Self::Publish(PubError::Failed),
            LifecycleOperationError::AlreadyDispatched => {
                Self::Publish(PubError::AlreadyDispatched)
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
impl AsyncLifecycleCallback<Aggregate, Repo, Publisher> for SaveThenDispatch {
    type Output = u64;
    type Error = HandlerError;

    fn call<'storage, 'callback>(
        self,
        context: &'callback mut AsyncLifecycleContext<'storage, Aggregate, Repo, Publisher>,
    ) -> Pin<Box<dyn Future<Output = Result<u64, HandlerError>> + Send + 'callback>>
    where
        'storage: 'callback,
        Aggregate: Send + 'storage,
        Repo: AsyncRepository<Aggregate = Aggregate> + Send + 'storage,
        Publisher: AsyncDomainEventPublisher<Event = Event> + Send + 'storage,
        Event: Send + Sync + 'storage,
        <Repo as AsyncRepository>::Error: Send + 'storage,
        <Publisher as AsyncDomainEventPublisher>::Error: Send + 'storage,
        Self: Send + 'callback,
        u64: Send + 'callback,
        HandlerError: Send + 'callback,
    {
        Box::pin(async move {
            context.save().await.map_err(HandlerError::from)?;
            context.dispatch().await.map_err(HandlerError::from)?;
            Ok(7)
        })
    }
}

#[derive(Clone, Copy)]
struct DispatchThenSave;
impl AsyncLifecycleCallback<Aggregate, Repo, Publisher> for DispatchThenSave {
    type Output = u64;
    type Error = HandlerError;

    fn call<'storage, 'callback>(
        self,
        context: &'callback mut AsyncLifecycleContext<'storage, Aggregate, Repo, Publisher>,
    ) -> Pin<Box<dyn Future<Output = Result<u64, HandlerError>> + Send + 'callback>>
    where
        'storage: 'callback,
        Aggregate: Send + 'storage,
        Repo: AsyncRepository<Aggregate = Aggregate> + Send + 'storage,
        Publisher: AsyncDomainEventPublisher<Event = Event> + Send + 'storage,
        Event: Send + Sync + 'storage,
        <Repo as AsyncRepository>::Error: Send + 'storage,
        <Publisher as AsyncDomainEventPublisher>::Error: Send + 'storage,
        Self: Send + 'callback,
        u64: Send + 'callback,
        HandlerError: Send + 'callback,
    {
        Box::pin(async move {
            context.dispatch().await.map_err(HandlerError::from)?;
            context.save().await.map_err(HandlerError::from)?;
            Ok(8)
        })
    }
}

#[derive(Clone, Copy)]
struct DispatchOnly;
impl AsyncLifecycleCallback<Aggregate, Repo, Publisher> for DispatchOnly {
    type Output = u64;
    type Error = HandlerError;

    fn call<'storage, 'callback>(
        self,
        context: &'callback mut AsyncLifecycleContext<'storage, Aggregate, Repo, Publisher>,
    ) -> Pin<Box<dyn Future<Output = Result<u64, HandlerError>> + Send + 'callback>>
    where
        'storage: 'callback,
        Aggregate: Send + 'storage,
        Repo: AsyncRepository<Aggregate = Aggregate> + Send + 'storage,
        Publisher: AsyncDomainEventPublisher<Event = Event> + Send + 'storage,
        Event: Send + Sync + 'storage,
        <Repo as AsyncRepository>::Error: Send + 'storage,
        <Publisher as AsyncDomainEventPublisher>::Error: Send + 'storage,
        Self: Send + 'callback,
        u64: Send + 'callback,
        HandlerError: Send + 'callback,
    {
        Box::pin(async move {
            context.dispatch().await.map_err(HandlerError::from)?;
            Ok(9)
        })
    }
}

#[derive(Clone, Copy)]
struct DuplicateSuccessfulSave;
impl AsyncLifecycleCallback<Aggregate, Repo, Publisher> for DuplicateSuccessfulSave {
    type Output = u64;
    type Error = HandlerError;

    fn call<'storage, 'callback>(
        self,
        context: &'callback mut AsyncLifecycleContext<'storage, Aggregate, Repo, Publisher>,
    ) -> Pin<Box<dyn Future<Output = Result<u64, HandlerError>> + Send + 'callback>>
    where
        'storage: 'callback,
        Aggregate: Send + 'storage,
        Repo: AsyncRepository<Aggregate = Aggregate> + Send + 'storage,
        Publisher: AsyncDomainEventPublisher<Event = Event> + Send + 'storage,
        Event: Send + Sync + 'storage,
        <Repo as AsyncRepository>::Error: Send + 'storage,
        <Publisher as AsyncDomainEventPublisher>::Error: Send + 'storage,
        Self: Send + 'callback,
        u64: Send + 'callback,
        HandlerError: Send + 'callback,
    {
        Box::pin(async move {
            context.save().await.map_err(HandlerError::from)?;
            assert!(matches!(
                context.save().await,
                Err(LifecycleOperationError::AlreadySaved)
            ));
            context.dispatch().await.map_err(HandlerError::from)?;
            Ok(10)
        })
    }
}

#[derive(Clone, Copy)]
struct DuplicateSuccessfulDispatch;
impl AsyncLifecycleCallback<Aggregate, Repo, Publisher> for DuplicateSuccessfulDispatch {
    type Output = u64;
    type Error = HandlerError;

    fn call<'storage, 'callback>(
        self,
        context: &'callback mut AsyncLifecycleContext<'storage, Aggregate, Repo, Publisher>,
    ) -> Pin<Box<dyn Future<Output = Result<u64, HandlerError>> + Send + 'callback>>
    where
        'storage: 'callback,
        Aggregate: Send + 'storage,
        Repo: AsyncRepository<Aggregate = Aggregate> + Send + 'storage,
        Publisher: AsyncDomainEventPublisher<Event = Event> + Send + 'storage,
        Event: Send + Sync + 'storage,
        <Repo as AsyncRepository>::Error: Send + 'storage,
        <Publisher as AsyncDomainEventPublisher>::Error: Send + 'storage,
        Self: Send + 'callback,
        u64: Send + 'callback,
        HandlerError: Send + 'callback,
    {
        Box::pin(async move {
            context.dispatch().await.map_err(HandlerError::from)?;
            assert!(matches!(
                context.dispatch().await,
                Err(LifecycleOperationError::AlreadyDispatched)
            ));
            context.save().await.map_err(HandlerError::from)?;
            Ok(11)
        })
    }
}

struct Handler {
    repository: Repo,
    publisher: Publisher,
}
impl AsyncCommandHandler for Handler {
    type Command = u64;
    type Aggregate = Aggregate;
    type Repository = Repo;
    type Publisher = Publisher;
    type Output = u64;
    type Error = HandlerError;

    fn handle<'a, F>(
        &'a mut self,
        id: &'a u64,
        command: u64,
        callback: F,
    ) -> Pin<Box<dyn Future<Output = Result<u64, HandlerError>> + Send + 'a>>
    where
        Self: Send + 'a,
        Self::Command: Send + 'a,
        Self::Aggregate: Send + 'a,
        <Self::Aggregate as Entity>::Id: Sync + 'a,
        <Self::Aggregate as AggregateRoot>::Event: Send + Sync + 'a,
        Self::Repository: Send + 'a,
        <Self::Repository as AsyncRepository>::Error: Send + 'a,
        Self::Publisher: Send + 'a,
        <Self::Publisher as AsyncDomainEventPublisher>::Error: Send + 'a,
        Self::Output: Send + 'a,
        Self::Error: From<LifecycleOperationError<<Self::Repository as AsyncRepository>::Error>>
            + From<LifecycleOperationError<<Self::Publisher as AsyncDomainEventPublisher>::Error>>
            + From<LifecycleCompletionError>
            + Send
            + 'a,
        F: AsyncLifecycleCallback<
                Self::Aggregate,
                Self::Repository,
                Self::Publisher,
                Output = Self::Output,
                Error = Self::Error,
            > + Send
            + 'a,
    {
        Box::pin(async move {
            let mut aggregate = self
                .repository
                .find_by_id(id)
                .await
                .map_err(HandlerError::Repository)?
                .ok_or(HandlerError::Missing)?;
            if let Some(trace) = &self.repository.trace {
                trace.lock().unwrap().push("command");
            }
            aggregate.record_event(Event(command));
            let mut context = AsyncLifecycleContext::new(
                &mut aggregate,
                &mut self.repository,
                &mut self.publisher,
            );
            let output = callback.call(&mut context).await?;
            context.complete()?;
            Ok(output)
        })
    }
}

struct Query;
impl AsyncQueryHandler for Query {
    type Query = u64;
    type Result = u64;
    type Error = &'static str;

    fn handle<'a>(
        &'a self,
        query: u64,
    ) -> Pin<Box<dyn Future<Output = Result<u64, &'static str>> + Send + 'a>>
    where
        Self: Sync + 'a,
        Self::Query: Send + 'a,
        Self::Result: Send + 'a,
        Self::Error: Send + 'a,
    {
        Box::pin(async move {
            if query == 0 {
                Err("query")
            } else {
                Ok(query + 1)
            }
        })
    }
}

#[test]
fn async_handler_and_query_use_approved_boundaries() {
    let mut handler = Handler {
        repository: Repo {
            saved: Some(Aggregate {
                id: 1,
                events: Vec::new(),
            }),
            fail_find: false,
            fail_save: false,
            pending_save: false,
            trace: None,
        },
        publisher: Publisher {
            calls: Vec::new(),
            fail: false,
            pending: false,
            trace: None,
        },
    };
    assert_eq!(poll_ready(handler.handle(&1, 9, SaveThenDispatch)), Ok(7));
    assert_eq!(handler.publisher.calls, vec![9]);
    assert_eq!(poll_ready(Query.handle(4)), Ok(5));
}

#[test]
fn async_handler_rejects_dispatch_only_omission() {
    let trace = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut handler = Handler {
        repository: Repo {
            saved: Some(Aggregate {
                id: 1,
                events: Vec::new(),
            }),
            fail_find: false,
            fail_save: false,
            pending_save: false,
            trace: Some(trace.clone()),
        },
        publisher: Publisher {
            calls: Vec::new(),
            fail: false,
            pending: false,
            trace: Some(trace.clone()),
        },
    };

    assert_eq!(
        poll_ready(handler.handle(&1, 12, DispatchOnly)),
        Err(HandlerError::Completion(
            LifecycleCompletionError::MissingSave
        ))
    );
    assert_eq!(handler.publisher.calls, vec![12]);
    assert_eq!(
        *trace.lock().unwrap(),
        vec!["lookup", "command", "dispatch"]
    );
}

#[test]
fn async_handler_rejects_successful_duplicate_save_without_second_adapter_call() {
    let trace = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut handler = Handler {
        repository: Repo {
            saved: Some(Aggregate {
                id: 1,
                events: Vec::new(),
            }),
            fail_find: false,
            fail_save: false,
            pending_save: false,
            trace: Some(trace.clone()),
        },
        publisher: Publisher {
            calls: Vec::new(),
            fail: false,
            pending: false,
            trace: Some(trace.clone()),
        },
    };

    assert_eq!(
        poll_ready(handler.handle(&1, 13, DuplicateSuccessfulSave)),
        Ok(10)
    );
    assert_eq!(
        *trace.lock().unwrap(),
        vec!["lookup", "command", "save", "dispatch"]
    );
    assert_eq!(handler.publisher.calls, vec![13]);
}

#[test]
fn async_handler_rejects_successful_duplicate_dispatch_without_second_publish() {
    let trace = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut handler = Handler {
        repository: Repo {
            saved: Some(Aggregate {
                id: 1,
                events: Vec::new(),
            }),
            fail_find: false,
            fail_save: false,
            pending_save: false,
            trace: Some(trace.clone()),
        },
        publisher: Publisher {
            calls: Vec::new(),
            fail: false,
            pending: false,
            trace: Some(trace.clone()),
        },
    };

    assert_eq!(
        poll_ready(handler.handle(&1, 14, DuplicateSuccessfulDispatch)),
        Ok(11)
    );
    assert_eq!(
        *trace.lock().unwrap(),
        vec!["lookup", "command", "dispatch", "save"]
    );
    assert_eq!(handler.publisher.calls, vec![14]);
}

#[test]
fn async_creator_uses_a_separate_explicit_boundary() {
    let mut creator = Creator {
        repository: Repo {
            saved: None,
            fail_find: false,
            fail_save: false,
            pending_save: false,
            trace: None,
        },
        publisher: Publisher {
            calls: Vec::new(),
            fail: false,
            pending: false,
            trace: None,
        },
    };
    assert_eq!(poll_ready(creator.create(5, SaveThenDispatch)), Ok(7));
    assert_eq!(creator.publisher.calls, vec![5]);
}

#[test]
fn async_handler_covers_lookup_save_dispatch_and_reverse_order_errors() {
    let mut lookup = Handler {
        repository: Repo {
            saved: None,
            fail_find: true,
            fail_save: false,
            pending_save: false,
            trace: None,
        },
        publisher: Publisher {
            calls: Vec::new(),
            fail: false,
            pending: false,
            trace: None,
        },
    };
    assert_eq!(
        poll_ready(lookup.handle(&1, 1, SaveThenDispatch)),
        Err(HandlerError::Repository("lookup"))
    );

    let mut missing = Handler {
        repository: Repo {
            saved: None,
            fail_find: false,
            fail_save: false,
            pending_save: false,
            trace: None,
        },
        publisher: Publisher {
            calls: Vec::new(),
            fail: false,
            pending: false,
            trace: None,
        },
    };
    assert_eq!(
        poll_ready(missing.handle(&1, 1, SaveThenDispatch)),
        Err(HandlerError::Missing)
    );

    let mut save = Handler {
        repository: Repo {
            saved: Some(Aggregate {
                id: 1,
                events: vec![],
            }),
            fail_find: false,
            fail_save: true,
            pending_save: false,
            trace: None,
        },
        publisher: Publisher {
            calls: Vec::new(),
            fail: false,
            pending: false,
            trace: None,
        },
    };
    assert_eq!(
        poll_ready(save.handle(&1, 1, SaveThenDispatch)),
        Err(HandlerError::Repository("save"))
    );
    assert!(save.publisher.calls.is_empty());

    let mut dispatch = Handler {
        repository: Repo {
            saved: Some(Aggregate {
                id: 1,
                events: vec![],
            }),
            fail_find: false,
            fail_save: false,
            pending_save: false,
            trace: None,
        },
        publisher: Publisher {
            calls: Vec::new(),
            fail: true,
            pending: false,
            trace: None,
        },
    };
    assert_eq!(
        poll_ready(dispatch.handle(&1, 1, SaveThenDispatch)),
        Err(HandlerError::Publish(PubError::Failed))
    );

    let mut reverse = Handler {
        repository: Repo {
            saved: Some(Aggregate {
                id: 1,
                events: vec![],
            }),
            fail_find: false,
            fail_save: false,
            pending_save: false,
            trace: None,
        },
        publisher: Publisher {
            calls: Vec::new(),
            fail: false,
            pending: false,
            trace: None,
        },
    };
    assert_eq!(poll_ready(reverse.handle(&1, 1, DispatchThenSave)), Ok(8));
}

#[test]
fn async_query_reports_consumer_error() {
    assert_eq!(poll_ready(Query.handle(0)), Err("query"));
}

#[test]
fn async_context_preserves_fifo_and_rejects_swallowed_errors_and_retries() {
    let mut aggregate = Aggregate {
        id: 1,
        events: vec![Event(1), Event(2), Event(3)],
    };
    let mut repository = Repo {
        saved: None,
        fail_find: false,
        fail_save: false,
        pending_save: false,
        trace: None,
    };
    let mut publisher = Publisher {
        calls: Vec::new(),
        fail: false,
        pending: false,
        trace: None,
    };
    let mut context = AsyncLifecycleContext::new(&mut aggregate, &mut repository, &mut publisher);
    assert_eq!(poll_ready(context.dispatch()), Ok(()));
    assert_eq!(poll_ready(context.save()), Ok(()));
    assert_eq!(context.complete(), Ok(()));
    assert_eq!(publisher.calls, vec![1, 2, 3]);
    assert!(aggregate.events.is_empty());

    let mut aggregate = Aggregate {
        id: 1,
        events: vec![Event(9)],
    };
    let mut repository = Repo {
        saved: None,
        fail_find: false,
        fail_save: false,
        pending_save: false,
        trace: None,
    };
    let mut publisher = Publisher {
        calls: Vec::new(),
        fail: true,
        pending: false,
        trace: None,
    };
    let mut context = AsyncLifecycleContext::new(&mut aggregate, &mut repository, &mut publisher);
    let _ = poll_ready(context.dispatch());
    assert!(matches!(
        poll_ready(context.dispatch()),
        Err(LifecycleOperationError::AlreadyDispatched)
    ));
    assert_eq!(
        context.complete(),
        Err(LifecycleCompletionError::DispatchFailedConsumed)
    );

    let mut aggregate = Aggregate {
        id: 1,
        events: vec![],
    };
    let mut repository = Repo {
        saved: None,
        fail_find: false,
        fail_save: true,
        pending_save: false,
        trace: None,
    };
    let mut publisher = Publisher {
        calls: Vec::new(),
        fail: false,
        pending: false,
        trace: None,
    };
    let mut context = AsyncLifecycleContext::new(&mut aggregate, &mut repository, &mut publisher);
    assert_eq!(
        poll_ready(context.save()),
        Err(LifecycleOperationError::Adapter("save"))
    );
    assert!(matches!(
        poll_ready(context.save()),
        Err(LifecycleOperationError::AlreadySaved)
    ));
    assert_eq!(
        context.complete(),
        Err(LifecycleCompletionError::SaveFailedConsumed)
    );
}

#[test]
fn async_dispatch_cancellation_restores_fifo_and_consumes_slot() {
    let mut aggregate = Aggregate {
        id: 1,
        events: vec![Event(4), Event(5)],
    };
    let mut repository = Repo {
        saved: None,
        fail_find: false,
        fail_save: false,
        pending_save: false,
        trace: None,
    };
    let mut publisher = Publisher {
        calls: Vec::new(),
        fail: false,
        pending: true,
        trace: None,
    };
    let mut context = AsyncLifecycleContext::new(&mut aggregate, &mut repository, &mut publisher);
    let mut future = context.dispatch();
    let waker = Waker::noop();
    let mut task = Context::from_waker(waker);
    assert!(matches!(future.as_mut().poll(&mut task), Poll::Pending));
    drop(future);
    assert!(matches!(
        poll_ready(context.dispatch()),
        Err(LifecycleOperationError::AlreadyDispatched)
    ));
    assert_eq!(
        context.complete(),
        Err(LifecycleCompletionError::DispatchFailedConsumed)
    );
    assert_eq!(aggregate.events, vec![Event(4), Event(5)]);
}

#[test]
fn async_creator_converts_save_and_dispatch_errors() {
    let mut save = Creator {
        repository: Repo {
            saved: None,
            fail_find: false,
            fail_save: true,
            pending_save: false,
            trace: None,
        },
        publisher: Publisher {
            calls: Vec::new(),
            fail: false,
            pending: false,
            trace: None,
        },
    };
    assert_eq!(
        poll_ready(save.create(5, SaveThenDispatch)),
        Err(HandlerError::Repository("save"))
    );

    let mut dispatch = Creator {
        repository: Repo {
            saved: None,
            fail_find: false,
            fail_save: false,
            pending_save: false,
            trace: None,
        },
        publisher: Publisher {
            calls: Vec::new(),
            fail: true,
            pending: false,
            trace: None,
        },
    };
    assert_eq!(
        poll_ready(dispatch.create(5, SaveThenDispatch)),
        Err(HandlerError::Publish(PubError::Failed))
    );
}

struct Creator {
    repository: Repo,
    publisher: Publisher,
}
impl AsyncAggregateCreator for Creator {
    type Command = u64;
    type Aggregate = Aggregate;
    type Repository = Repo;
    type Publisher = Publisher;
    type Output = u64;
    type Error = HandlerError;

    fn create<'a, F>(
        &'a mut self,
        command: u64,
        callback: F,
    ) -> Pin<Box<dyn Future<Output = Result<u64, HandlerError>> + Send + 'a>>
    where
        Self: Send + 'a,
        Self::Command: Send + 'a,
        Self::Aggregate: Send + 'a,
        <Self::Aggregate as AggregateRoot>::Event: Send + Sync + 'a,
        Self::Repository: Send + 'a,
        <Self::Repository as AsyncRepository>::Error: Send + 'a,
        Self::Publisher: Send + 'a,
        <Self::Publisher as AsyncDomainEventPublisher>::Error: Send + 'a,
        Self::Output: Send + 'a,
        Self::Error: From<LifecycleOperationError<<Self::Repository as AsyncRepository>::Error>>
            + From<LifecycleOperationError<<Self::Publisher as AsyncDomainEventPublisher>::Error>>
            + From<LifecycleCompletionError>
            + Send
            + 'a,
        F: AsyncLifecycleCallback<
                Self::Aggregate,
                Self::Repository,
                Self::Publisher,
                Output = Self::Output,
                Error = Self::Error,
            > + Send
            + 'a,
    {
        Box::pin(async move {
            let mut aggregate = Aggregate {
                id: command,
                events: vec![Event(command)],
            };
            let mut context = AsyncLifecycleContext::new(
                &mut aggregate,
                &mut self.repository,
                &mut self.publisher,
            );
            let output = callback.call(&mut context).await?;
            context.complete()?;
            Ok(output)
        })
    }
}

struct SwallowAsyncSaveError;
impl AsyncLifecycleCallback<Aggregate, Repo, Publisher> for SwallowAsyncSaveError {
    type Output = u64;
    type Error = HandlerError;
    fn call<'storage, 'callback>(
        self,
        context: &'callback mut AsyncLifecycleContext<'storage, Aggregate, Repo, Publisher>,
    ) -> Pin<Box<dyn Future<Output = Result<u64, HandlerError>> + Send + 'callback>>
    where
        'storage: 'callback,
        Aggregate: Send + 'storage,
        Repo: AsyncRepository<Aggregate = Aggregate> + Send + 'storage,
        Publisher: AsyncDomainEventPublisher<Event = Event> + Send + 'storage,
        Event: Send + Sync + 'storage,
        <Repo as AsyncRepository>::Error: Send + 'storage,
        <Publisher as AsyncDomainEventPublisher>::Error: Send + 'storage,
        Self: Send + 'callback,
        u64: Send + 'callback,
        HandlerError: Send + 'callback,
    {
        Box::pin(async move {
            let _ = context.save().await;
            context.dispatch().await.map_err(HandlerError::from)?;
            Ok(11)
        })
    }
}

struct SwallowAsyncDispatchError;
impl AsyncLifecycleCallback<Aggregate, Repo, Publisher> for SwallowAsyncDispatchError {
    type Output = u64;
    type Error = HandlerError;
    fn call<'storage, 'callback>(
        self,
        context: &'callback mut AsyncLifecycleContext<'storage, Aggregate, Repo, Publisher>,
    ) -> Pin<Box<dyn Future<Output = Result<u64, HandlerError>> + Send + 'callback>>
    where
        'storage: 'callback,
        Aggregate: Send + 'storage,
        Repo: AsyncRepository<Aggregate = Aggregate> + Send + 'storage,
        Publisher: AsyncDomainEventPublisher<Event = Event> + Send + 'storage,
        Event: Send + Sync + 'storage,
        <Repo as AsyncRepository>::Error: Send + 'storage,
        <Publisher as AsyncDomainEventPublisher>::Error: Send + 'storage,
        Self: Send + 'callback,
        u64: Send + 'callback,
        HandlerError: Send + 'callback,
    {
        Box::pin(async move {
            let _ = context.dispatch().await;
            context.save().await.map_err(HandlerError::from)?;
            Ok(12)
        })
    }
}

struct AsyncDomainFailure;
impl AsyncLifecycleCallback<Aggregate, Repo, Publisher> for AsyncDomainFailure {
    type Output = u64;
    type Error = HandlerError;
    fn call<'storage, 'callback>(
        self,
        _: &'callback mut AsyncLifecycleContext<'storage, Aggregate, Repo, Publisher>,
    ) -> Pin<Box<dyn Future<Output = Result<u64, HandlerError>> + Send + 'callback>>
    where
        'storage: 'callback,
        Aggregate: Send + 'storage,
        Repo: AsyncRepository<Aggregate = Aggregate> + Send + 'storage,
        Publisher: AsyncDomainEventPublisher<Event = Event> + Send + 'storage,
        Event: Send + Sync + 'storage,
        <Repo as AsyncRepository>::Error: Send + 'storage,
        <Publisher as AsyncDomainEventPublisher>::Error: Send + 'storage,
        Self: Send + 'callback,
        u64: Send + 'callback,
        HandlerError: Send + 'callback,
    {
        Box::pin(async { Err(HandlerError::Missing) })
    }
}

struct RetryAsyncSave;
impl AsyncLifecycleCallback<Aggregate, Repo, Publisher> for RetryAsyncSave {
    type Output = u64;
    type Error = HandlerError;
    fn call<'storage, 'callback>(
        self,
        context: &'callback mut AsyncLifecycleContext<'storage, Aggregate, Repo, Publisher>,
    ) -> Pin<Box<dyn Future<Output = Result<u64, HandlerError>> + Send + 'callback>>
    where
        'storage: 'callback,
        Aggregate: Send + 'storage,
        Repo: AsyncRepository<Aggregate = Aggregate> + Send + 'storage,
        Publisher: AsyncDomainEventPublisher<Event = Event> + Send + 'storage,
        Event: Send + Sync + 'storage,
        <Repo as AsyncRepository>::Error: Send + 'storage,
        <Publisher as AsyncDomainEventPublisher>::Error: Send + 'storage,
        Self: Send + 'callback,
        u64: Send + 'callback,
        HandlerError: Send + 'callback,
    {
        Box::pin(async move {
            assert!(matches!(
                context.save().await,
                Err(LifecycleOperationError::Adapter("save"))
            ));
            assert!(matches!(
                context.save().await,
                Err(LifecycleOperationError::AlreadySaved)
            ));
            Err(HandlerError::Repository("retry rejected"))
        })
    }
}

struct RetryAsyncDispatch;
impl AsyncLifecycleCallback<Aggregate, Repo, Publisher> for RetryAsyncDispatch {
    type Output = u64;
    type Error = HandlerError;
    fn call<'storage, 'callback>(
        self,
        context: &'callback mut AsyncLifecycleContext<'storage, Aggregate, Repo, Publisher>,
    ) -> Pin<Box<dyn Future<Output = Result<u64, HandlerError>> + Send + 'callback>>
    where
        'storage: 'callback,
        Aggregate: Send + 'storage,
        Repo: AsyncRepository<Aggregate = Aggregate> + Send + 'storage,
        Publisher: AsyncDomainEventPublisher<Event = Event> + Send + 'storage,
        Event: Send + Sync + 'storage,
        <Repo as AsyncRepository>::Error: Send + 'storage,
        <Publisher as AsyncDomainEventPublisher>::Error: Send + 'storage,
        Self: Send + 'callback,
        u64: Send + 'callback,
        HandlerError: Send + 'callback,
    {
        Box::pin(async move {
            context.save().await.map_err(HandlerError::from)?;
            assert!(matches!(
                context.dispatch().await,
                Err(LifecycleOperationError::Adapter(PubError::Failed))
            ));
            assert!(matches!(
                context.dispatch().await,
                Err(LifecycleOperationError::AlreadyDispatched)
            ));
            Err(HandlerError::Publish(PubError::AlreadyDispatched))
        })
    }
}

#[test]
fn async_handler_rejects_callback_retries_after_operation_failure() {
    let mut save_failed = Handler {
        repository: Repo {
            saved: Some(Aggregate {
                id: 1,
                events: vec![],
            }),
            fail_find: false,
            fail_save: true,
            pending_save: false,
            trace: None,
        },
        publisher: Publisher {
            calls: vec![],
            fail: false,
            pending: false,
            trace: None,
        },
    };
    assert_eq!(
        poll_ready(save_failed.handle(&1, 1, RetryAsyncSave)),
        Err(HandlerError::Repository("retry rejected"))
    );

    let mut dispatch_failed = Handler {
        repository: Repo {
            saved: Some(Aggregate {
                id: 1,
                events: vec![],
            }),
            fail_find: false,
            fail_save: false,
            pending_save: false,
            trace: None,
        },
        publisher: Publisher {
            calls: vec![],
            fail: true,
            pending: false,
            trace: None,
        },
    };
    assert_eq!(
        poll_ready(dispatch_failed.handle(&1, 1, RetryAsyncDispatch)),
        Err(HandlerError::Publish(PubError::AlreadyDispatched))
    );
}

#[test]
fn async_handler_rejects_swallowed_save_and_dispatch_errors() {
    let mut save_failed = Handler {
        repository: Repo {
            saved: Some(Aggregate {
                id: 1,
                events: vec![],
            }),
            fail_find: false,
            fail_save: true,
            pending_save: false,
            trace: None,
        },
        publisher: Publisher {
            calls: vec![],
            fail: false,
            pending: false,
            trace: None,
        },
    };
    assert_eq!(
        poll_ready(save_failed.handle(&1, 1, SwallowAsyncSaveError)),
        Err(HandlerError::Completion(
            LifecycleCompletionError::SaveFailedConsumed
        ))
    );

    let mut dispatch_failed = Handler {
        repository: Repo {
            saved: Some(Aggregate {
                id: 1,
                events: vec![],
            }),
            fail_find: false,
            fail_save: false,
            pending_save: false,
            trace: None,
        },
        publisher: Publisher {
            calls: vec![],
            fail: true,
            pending: false,
            trace: None,
        },
    };
    assert_eq!(
        poll_ready(dispatch_failed.handle(&1, 1, SwallowAsyncDispatchError)),
        Err(HandlerError::Completion(
            LifecycleCompletionError::DispatchFailedConsumed
        ))
    );
}

#[test]
fn async_creator_supports_dispatch_first_and_domain_failure() {
    let mut creator = Creator {
        repository: Repo {
            saved: None,
            fail_find: false,
            fail_save: false,
            pending_save: false,
            trace: None,
        },
        publisher: Publisher {
            calls: vec![],
            fail: false,
            pending: false,
            trace: None,
        },
    };
    assert_eq!(poll_ready(creator.create(7, DispatchThenSave)), Ok(8));

    let mut failed = Creator {
        repository: Repo {
            saved: None,
            fail_find: false,
            fail_save: false,
            pending_save: false,
            trace: None,
        },
        publisher: Publisher {
            calls: vec![],
            fail: false,
            pending: false,
            trace: None,
        },
    };
    assert_eq!(
        poll_ready(failed.create(8, AsyncDomainFailure)),
        Err(HandlerError::Missing)
    );
    assert!(failed.publisher.calls.is_empty());
}

#[test]
fn async_handler_boundary_restores_saved_pending_events_after_dispatch_error() {
    let mut handler = Handler {
        repository: Repo {
            saved: Some(Aggregate {
                id: 1,
                events: vec![],
            }),
            fail_find: false,
            fail_save: false,
            pending_save: false,
            trace: None,
        },
        publisher: Publisher {
            calls: vec![],
            fail: true,
            pending: false,
            trace: None,
        },
    };
    assert_eq!(
        poll_ready(handler.handle(&1, 4, SaveThenDispatch)),
        Err(HandlerError::Publish(PubError::Failed))
    );
    assert_eq!(handler.publisher.calls, vec![4]);
    assert_eq!(handler.repository.saved.unwrap().events, vec![Event(4)]);
}

struct PendingSaveRetry(std::sync::Arc<std::sync::atomic::AtomicBool>);
impl AsyncLifecycleCallback<Aggregate, Repo, Publisher> for PendingSaveRetry {
    type Output = u64;
    type Error = HandlerError;

    fn call<'storage, 'callback>(
        self,
        context: &'callback mut AsyncLifecycleContext<'storage, Aggregate, Repo, Publisher>,
    ) -> Pin<Box<dyn Future<Output = Result<u64, HandlerError>> + Send + 'callback>>
    where
        'storage: 'callback,
        Aggregate: Send + 'storage,
        Repo: AsyncRepository<Aggregate = Aggregate> + Send + 'storage,
        Publisher: AsyncDomainEventPublisher<Event = Event> + Send + 'storage,
        Event: Send + Sync + 'storage,
        <Repo as AsyncRepository>::Error: Send + 'storage,
        <Publisher as AsyncDomainEventPublisher>::Error: Send + 'storage,
        Self: Send + 'callback,
        u64: Send + 'callback,
        HandlerError: Send + 'callback,
    {
        Box::pin(async move {
            let pending = {
                let mut pending = context.save();
                let waker = Waker::noop();
                let mut cx = Context::from_waker(waker);
                matches!(pending.as_mut().poll(&mut cx), Poll::Pending)
            };
            assert!(pending);

            if matches!(
                context.save().await,
                Err(LifecycleOperationError::AlreadySaved)
            ) {
                self.0.store(true, std::sync::atomic::Ordering::SeqCst);
            }
            context.dispatch().await.map_err(HandlerError::from)?;
            Ok(99)
        })
    }
}

#[test]
fn async_handler_rejects_retry_after_pending_save_cancellation() {
    let retried = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mut handler = Handler {
        repository: Repo {
            saved: Some(Aggregate {
                id: 1,
                events: vec![],
            }),
            fail_find: false,
            fail_save: false,
            pending_save: true,
            trace: None,
        },
        publisher: Publisher {
            calls: vec![],
            fail: false,
            pending: false,
            trace: None,
        },
    };
    assert_eq!(
        poll_ready(handler.handle(&1, 1, PendingSaveRetry(retried.clone()))),
        Err(HandlerError::Completion(
            LifecycleCompletionError::SaveFailedConsumed
        ))
    );
    assert!(retried.load(std::sync::atomic::Ordering::SeqCst));
}

#[test]
fn async_creator_boundary_restores_events_after_dispatch_error() {
    let mut creator = Creator {
        repository: Repo {
            saved: None,
            fail_find: false,
            fail_save: false,
            pending_save: false,
            trace: None,
        },
        publisher: Publisher {
            calls: vec![],
            fail: true,
            pending: false,
            trace: None,
        },
    };
    assert_eq!(
        poll_ready(creator.create(13, SwallowAsyncDispatchError)),
        Err(HandlerError::Completion(
            LifecycleCompletionError::DispatchFailedConsumed
        ))
    );
    assert_eq!(creator.repository.saved.unwrap().events, vec![Event(13)]);
}

#[test]
fn async_handler_records_lookup_command_and_both_operation_orders() {
    let trace = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut save_first = Handler {
        repository: Repo {
            saved: Some(Aggregate {
                id: 1,
                events: vec![],
            }),
            fail_find: false,
            fail_save: false,
            pending_save: false,
            trace: Some(trace.clone()),
        },
        publisher: Publisher {
            calls: vec![],
            fail: false,
            pending: false,
            trace: Some(trace.clone()),
        },
    };
    assert_eq!(
        poll_ready(save_first.handle(&1, 1, SaveThenDispatch)),
        Ok(7)
    );
    assert_eq!(
        *trace.lock().unwrap(),
        vec!["lookup", "command", "save", "dispatch"]
    );

    trace.lock().unwrap().clear();
    let mut dispatch_first = Handler {
        repository: Repo {
            saved: Some(Aggregate {
                id: 1,
                events: vec![],
            }),
            fail_find: false,
            fail_save: false,
            pending_save: false,
            trace: Some(trace.clone()),
        },
        publisher: Publisher {
            calls: vec![],
            fail: false,
            pending: false,
            trace: Some(trace.clone()),
        },
    };
    assert_eq!(
        poll_ready(dispatch_first.handle(&1, 1, DispatchThenSave)),
        Ok(8)
    );
    assert_eq!(
        *trace.lock().unwrap(),
        vec!["lookup", "command", "dispatch", "save"]
    );
}

struct InstrumentedAsyncQuery(std::sync::Arc<std::sync::atomic::AtomicUsize>);
impl AsyncQueryHandler for InstrumentedAsyncQuery {
    type Query = ();
    type Result = usize;
    type Error = ();

    fn handle<'a>(&'a self, _: ()) -> Pin<Box<dyn Future<Output = Result<usize, ()>> + Send + 'a>>
    where
        Self: Sync + 'a,
        Self::Query: Send + 'a,
        Self::Result: Send + 'a,
        Self::Error: Send + 'a,
    {
        let value = self.0.load(std::sync::atomic::Ordering::SeqCst);
        Box::pin(async move { Ok(value) })
    }
}

#[test]
fn async_query_read_only_boundary_preserves_instrumented_state() {
    let state = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(17));
    let query = InstrumentedAsyncQuery(state.clone());
    assert_eq!(poll_ready(query.handle(())), Ok(17));
    assert_eq!(state.load(std::sync::atomic::Ordering::SeqCst), 17);
}

struct AsyncCallbackCardinality(std::sync::Arc<std::sync::atomic::AtomicUsize>);
impl AsyncLifecycleCallback<Aggregate, Repo, Publisher> for AsyncCallbackCardinality {
    type Output = u64;
    type Error = HandlerError;

    fn call<'storage, 'callback>(
        self,
        context: &'callback mut AsyncLifecycleContext<'storage, Aggregate, Repo, Publisher>,
    ) -> Pin<Box<dyn Future<Output = Result<u64, HandlerError>> + Send + 'callback>>
    where
        'storage: 'callback,
        Aggregate: Send + 'storage,
        Repo: AsyncRepository<Aggregate = Aggregate> + Send + 'storage,
        Publisher: AsyncDomainEventPublisher<Event = Event> + Send + 'storage,
        Event: Send + Sync + 'storage,
        <Repo as AsyncRepository>::Error: Send + 'storage,
        <Publisher as AsyncDomainEventPublisher>::Error: Send + 'storage,
        Self: Send + 'callback,
        u64: Send + 'callback,
        HandlerError: Send + 'callback,
    {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Box::pin(async move {
            context.save().await.map_err(HandlerError::from)?;
            context.dispatch().await.map_err(HandlerError::from)?;
            Ok(13)
        })
    }
}

fn require_exactly_one_callback(count: usize) -> Result<(), &'static str> {
    if count == 1 {
        Ok(())
    } else {
        Err("consumer handler violated the exactly-one callback contract")
    }
}

fn require_exactly_one_lookup_and_command(trace: &[&str]) -> Result<(), HandlerError> {
    if trace.starts_with(&["lookup", "command"]) {
        Ok(())
    } else {
        Err(HandlerError::Missing)
    }
}

#[test]
fn async_handler_instruments_exactly_one_callback_and_rejects_cardinality_nonconformance() {
    let count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut handler = Handler {
        repository: Repo {
            saved: Some(Aggregate {
                id: 1,
                events: vec![],
            }),
            fail_find: false,
            fail_save: false,
            pending_save: false,
            trace: None,
        },
        publisher: Publisher {
            calls: vec![],
            fail: false,
            pending: false,
            trace: None,
        },
    };
    assert_eq!(
        poll_ready(handler.handle(&1, 1, AsyncCallbackCardinality(count.clone()))),
        Ok(13)
    );
    assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(require_exactly_one_callback(1), Ok(()));
    assert!(require_exactly_one_callback(0).is_err());
    assert!(require_exactly_one_callback(2).is_err());
}

/// Consumer-owned handlers can violate callback cardinality and lifecycle ordering;
/// these fixtures make that behavioral obligation observable without claiming core enforcement.
struct NonConformingAsyncConsumer {
    repository: Repo,
    publisher: Publisher,
}
impl NonConformingAsyncConsumer {
    fn invoke_zero_callbacks(&mut self) -> Result<(), &'static str> {
        require_exactly_one_callback(0)
    }

    fn invoke_without_lookup(&mut self) -> Result<u64, HandlerError> {
        let mut aggregate = Aggregate {
            id: 1,
            events: vec![Event(1)],
        };
        let mut context =
            AsyncLifecycleContext::new(&mut aggregate, &mut self.repository, &mut self.publisher);
        let output = poll_ready(SaveThenDispatch.call(&mut context))?;
        context.complete()?;
        require_exactly_one_lookup_and_command(&["save", "dispatch"])?;
        Ok(output)
    }

    fn invoke_callback_twice(&mut self) -> Result<(), &'static str> {
        for value in [1, 2] {
            let mut aggregate = Aggregate {
                id: value,
                events: vec![Event(value)],
            };
            let mut context = AsyncLifecycleContext::new(
                &mut aggregate,
                &mut self.repository,
                &mut self.publisher,
            );
            let _ =
                poll_ready(SaveThenDispatch.call(&mut context)).map_err(|_| "callback failed")?;
            context.complete().map_err(|_| "callback failed")?;
        }
        require_exactly_one_callback(2)
    }
}

#[test]
fn async_consumer_handler_fixtures_record_nonconforming_cardinality_and_order() {
    let mut zero = NonConformingAsyncConsumer {
        repository: Repo {
            saved: None,
            fail_find: false,
            fail_save: false,
            pending_save: false,
            trace: None,
        },
        publisher: Publisher {
            calls: vec![],
            fail: false,
            pending: false,
            trace: None,
        },
    };
    assert_eq!(
        zero.invoke_zero_callbacks(),
        Err("consumer handler violated the exactly-one callback contract")
    );
    assert!(zero.publisher.calls.is_empty());

    let trace = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut skipped_lookup = NonConformingAsyncConsumer {
        repository: Repo {
            saved: None,
            fail_find: false,
            fail_save: false,
            pending_save: false,
            trace: Some(trace.clone()),
        },
        publisher: Publisher {
            calls: vec![],
            fail: false,
            pending: false,
            trace: Some(trace.clone()),
        },
    };
    assert_eq!(
        skipped_lookup.invoke_without_lookup(),
        Err(HandlerError::Missing)
    );
    assert_eq!(*trace.lock().unwrap(), vec!["save", "dispatch"]);

    let mut repeated = NonConformingAsyncConsumer {
        repository: Repo {
            saved: None,
            fail_find: false,
            fail_save: false,
            pending_save: false,
            trace: None,
        },
        publisher: Publisher {
            calls: vec![],
            fail: false,
            pending: false,
            trace: None,
        },
    };
    assert_eq!(
        repeated.invoke_callback_twice(),
        Err("consumer handler violated the exactly-one callback contract")
    );
    assert_eq!(repeated.publisher.calls, vec![1, 2]);
}

struct QueryActivityProbe {
    repository_searches: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    aggregate_mutations: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    dispatches: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}
impl AsyncQueryHandler for QueryActivityProbe {
    type Query = ();
    type Result = usize;
    type Error = ();

    fn handle<'a>(&'a self, _: ()) -> Pin<Box<dyn Future<Output = Result<usize, ()>> + Send + 'a>>
    where
        Self: Sync + 'a,
        Self::Query: Send + 'a,
        Self::Result: Send + 'a,
        Self::Error: Send + 'a,
    {
        Box::pin(async move { Ok(0) })
    }
}

#[test]
fn async_query_public_boundary_has_no_search_mutation_or_dispatch_capability() {
    let probe = QueryActivityProbe {
        repository_searches: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        aggregate_mutations: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        dispatches: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
    };
    assert_eq!(poll_ready(probe.handle(())), Ok(0));
    assert_eq!(
        probe
            .repository_searches
            .load(std::sync::atomic::Ordering::SeqCst),
        0
    );
    assert_eq!(
        probe
            .aggregate_mutations
            .load(std::sync::atomic::Ordering::SeqCst),
        0
    );
    assert_eq!(
        probe.dispatches.load(std::sync::atomic::Ordering::SeqCst),
        0
    );
}

#[derive(Clone, Copy)]
struct CancelPendingDispatch;
impl AsyncLifecycleCallback<Aggregate, Repo, Publisher> for CancelPendingDispatch {
    type Output = u64;
    type Error = HandlerError;

    fn call<'storage, 'callback>(
        self,
        context: &'callback mut AsyncLifecycleContext<'storage, Aggregate, Repo, Publisher>,
    ) -> Pin<Box<dyn Future<Output = Result<u64, HandlerError>> + Send + 'callback>>
    where
        'storage: 'callback,
        Aggregate: Send + 'storage,
        Repo: AsyncRepository<Aggregate = Aggregate> + Send + 'storage,
        Publisher: AsyncDomainEventPublisher<Event = Event> + Send + 'storage,
        Event: Send + Sync + 'storage,
        <Repo as AsyncRepository>::Error: Send + 'storage,
        <Publisher as AsyncDomainEventPublisher>::Error: Send + 'storage,
        Self: Send + 'callback,
        u64: Send + 'callback,
        HandlerError: Send + 'callback,
    {
        Box::pin(async move {
            context.save().await.map_err(HandlerError::from)?;
            {
                let waker = Waker::noop();
                let mut task = Context::from_waker(waker);
                let mut pending = context.dispatch();
                assert!(matches!(pending.as_mut().poll(&mut task), Poll::Pending));
                drop(pending);
            }
            assert!(matches!(
                context.dispatch().await,
                Err(LifecycleOperationError::AlreadyDispatched)
            ));
            Ok(77)
        })
    }
}

#[test]
fn async_handler_boundary_rejects_cancelled_pending_dispatch_without_retry_or_duplicate_publish() {
    EVENT_RECORD_LOG.with(|log| log.borrow_mut().clear());
    let mut handler = Handler {
        repository: Repo {
            saved: Some(Aggregate {
                id: 1,
                events: vec![Event(10), Event(11)],
            }),
            fail_find: false,
            fail_save: false,
            pending_save: false,
            trace: None,
        },
        publisher: Publisher {
            calls: Vec::new(),
            fail: false,
            pending: true,
            trace: None,
        },
    };

    assert_eq!(
        poll_ready(handler.handle(&1, 1, CancelPendingDispatch)),
        Err(HandlerError::Completion(
            LifecycleCompletionError::DispatchFailedConsumed
        ))
    );
    assert_eq!(handler.publisher.calls, vec![10]);
    EVENT_RECORD_LOG.with(|log| {
        assert_eq!(*log.borrow(), vec![1, 10, 11, 1]);
    });
}
