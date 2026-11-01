#![allow(clippy::type_complexity)]

use crate::cqrs::OperationState;
use crate::{
    AggregateRoot, AsyncDomainEventPublisher, AsyncRepository, Entity, LifecycleCompletionError,
    LifecycleOperationError, dispatch_events_async,
};
use std::future::Future;
use std::pin::Pin;

struct AttemptGuard<'a> {
    state: &'a mut OperationState,
    succeeded: bool,
}

impl<'a> AttemptGuard<'a> {
    fn new(state: &'a mut OperationState) -> Self {
        Self {
            state,
            succeeded: false,
        }
    }

    fn succeed(&mut self) {
        *self.state = OperationState::Succeeded;
        self.succeeded = true;
    }
}

impl Drop for AttemptGuard<'_> {
    fn drop(&mut self) {
        if !self.succeeded {
            *self.state = OperationState::FailedConsumed;
        }
    }
}

/// **English:** Coordinates asynchronous aggregate persistence and event dispatch. Each operation may be attempted once; dropping an in-progress future consumes that attempt.
///
/// **日本語:** 非同期の集約永続化とイベント配信を調整します。各操作は一度だけ試行でき、実行中の future を破棄すると試行は消費済みになります。
pub struct AsyncLifecycleContext<'storage, A, R, P>
where
    A: AggregateRoot + Send + 'storage,
    R: AsyncRepository<Aggregate = A> + Send + 'storage,
    P: AsyncDomainEventPublisher<Event = A::Event> + Send + 'storage,
    A::Event: Send + Sync + 'storage,
    R::Error: Send + 'storage,
    P::Error: Send + 'storage,
{
    aggregate: &'storage mut A,
    repository: &'storage mut R,
    publisher: &'storage mut P,
    save_state: OperationState,
    dispatch_state: OperationState,
}
impl<'storage, A, R, P> AsyncLifecycleContext<'storage, A, R, P>
where
    A: AggregateRoot + Send + 'storage,
    R: AsyncRepository<Aggregate = A> + Send + 'storage,
    P: AsyncDomainEventPublisher<Event = A::Event> + Send + 'storage,
    A::Event: Send + Sync + 'storage,
    R::Error: Send + 'storage,
    P::Error: Send + 'storage,
{
    /// **English:** Creates a lifecycle context borrowing the aggregate, repository, and publisher mutably.
    ///
    /// **日本語:** 集約、リポジトリ、パブリッシャーを可変借用するライフサイクルコンテキストを作成します。
    pub fn new(
        aggregate: &'storage mut A,
        repository: &'storage mut R,
        publisher: &'storage mut P,
    ) -> Self {
        Self {
            aggregate,
            repository,
            publisher,
            save_state: OperationState::Pending,
            dispatch_state: OperationState::Pending,
        }
    }

    /// **English:** Attempts repository persistence once; adapter errors are returned and the attempt cannot be retried.
    ///
    /// **日本語:** リポジトリへの保存を一度試行します。アダプターエラーを返し、再試行はできません。
    pub fn save<'ctx>(
        &'ctx mut self,
    ) -> Pin<Box<dyn Future<Output = Result<(), LifecycleOperationError<R::Error>>> + Send + 'ctx>>
    where
        'storage: 'ctx,
        A: Send + 'ctx,
        R: Send + 'ctx,
        R::Error: Send + 'ctx,
    {
        if self.save_state != OperationState::Pending {
            return Box::pin(async { Err(LifecycleOperationError::AlreadySaved) });
        }
        self.save_state = OperationState::FailedConsumed;
        let state = &mut self.save_state;
        let repository: &'ctx mut R = &mut *self.repository;
        let aggregate: &'ctx mut A = &mut *self.aggregate;
        Box::pin(async move {
            let mut guard = AttemptGuard::new(state);
            match repository.save(&*aggregate).await {
                Ok(()) => {
                    guard.succeed();
                    Ok(())
                }
                Err(error) => Err(LifecycleOperationError::Adapter(error)),
            }
        })
    }

    /// **English:** Attempts event dispatch once; adapter errors are returned and the attempt cannot be retried.
    ///
    /// **日本語:** イベント配信を一度試行します。アダプターエラーを返し、再試行はできません。
    pub fn dispatch<'ctx>(
        &'ctx mut self,
    ) -> Pin<Box<dyn Future<Output = Result<(), LifecycleOperationError<P::Error>>> + Send + 'ctx>>
    where
        'storage: 'ctx,
        A: Send + 'ctx,
        P: Send + 'ctx,
        A::Event: Send + Sync + 'ctx,
        P::Error: Send + 'ctx,
        P::Error: 'ctx,
    {
        if self.dispatch_state != OperationState::Pending {
            return Box::pin(async { Err(LifecycleOperationError::AlreadyDispatched) });
        }
        self.dispatch_state = OperationState::FailedConsumed;
        let state = &mut self.dispatch_state;
        let aggregate: &'ctx mut A = &mut *self.aggregate;
        let publisher: &'ctx mut P = &mut *self.publisher;
        Box::pin(async move {
            let mut guard = AttemptGuard::new(state);
            match dispatch_events_async(aggregate, publisher).await {
                Ok(()) => {
                    guard.succeed();
                    Ok(())
                }
                Err(error) => Err(LifecycleOperationError::Adapter(error)),
            }
        })
    }

    /// **English:** Consumes the context and succeeds only when both save and dispatch succeeded.
    ///
    /// **日本語:** コンテキストを消費し、保存と配信の両方が成功した場合にのみ成功します。
    pub fn complete(self) -> Result<(), LifecycleCompletionError> {
        match (self.save_state, self.dispatch_state) {
            (OperationState::Succeeded, OperationState::Succeeded) => Ok(()),
            (OperationState::FailedConsumed, _) => {
                Err(LifecycleCompletionError::SaveFailedConsumed)
            }
            (_, OperationState::FailedConsumed) => {
                Err(LifecycleCompletionError::DispatchFailedConsumed)
            }
            (OperationState::Pending, _) => Err(LifecycleCompletionError::MissingSave),
            (_, OperationState::Pending) => Err(LifecycleCompletionError::MissingDispatch),
        }
    }
}

/// **English:** Defines asynchronous work performed with a mutable asynchronous lifecycle context.
///
/// **日本語:** 非同期ライフサイクルコンテキストを使って実行する非同期処理を定義します。
pub trait AsyncLifecycleCallback<A, R, P>
where
    A: AggregateRoot + Send,
    R: AsyncRepository<Aggregate = A> + Send,
    P: AsyncDomainEventPublisher<Event = A::Event> + Send,
    A::Event: Send + Sync,
    R::Error: Send,
    P::Error: Send,
{
    type Output;
    type Error;

    fn call<'storage, 'callback>(
        self,
        context: &'callback mut AsyncLifecycleContext<'storage, A, R, P>,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send + 'callback>>
    where
        'storage: 'callback,
        A: 'storage,
        R: 'storage,
        P: 'storage,
        A::Event: 'storage,
        R::Error: 'storage,
        P::Error: 'storage,
        Self: Send + 'callback,
        Self::Output: Send + 'callback,
        Self::Error: Send + 'callback;
}

/// **English:** Asynchronously handles a command against an aggregate using a lifecycle callback.
///
/// **日本語:** ライフサイクルコールバックを使って集約に対するコマンドを非同期に処理します。
pub trait AsyncCommandHandler {
    type Command;
    type Aggregate: AggregateRoot;
    type Repository: AsyncRepository<Aggregate = Self::Aggregate>;
    type Publisher: AsyncDomainEventPublisher<Event = <Self::Aggregate as AggregateRoot>::Event>;
    type Output;
    type Error;

    fn handle<'a, F>(
        &'a mut self,
        id: &'a <Self::Aggregate as Entity>::Id,
        command: Self::Command,
        callback: F,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send + 'a>>
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
            + 'a;
}

/// **English:** Asynchronously creates an aggregate from a command and runs the lifecycle callback.
///
/// **日本語:** コマンドから集約を非同期に作成し、ライフサイクルコールバックを実行します。
pub trait AsyncAggregateCreator {
    type Command;
    type Aggregate: AggregateRoot;
    type Repository: AsyncRepository<Aggregate = Self::Aggregate>;
    type Publisher: AsyncDomainEventPublisher<Event = <Self::Aggregate as AggregateRoot>::Event>;
    type Output;
    type Error;

    fn create<'a, F>(
        &'a mut self,
        command: Self::Command,
        callback: F,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send + 'a>>
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
            + 'a;
}

/// **English:** Handles a query asynchronously without requiring aggregate lifecycle operations.
///
/// **日本語:** 集約ライフサイクル操作を伴わずにクエリを非同期処理します。
pub trait AsyncQueryHandler {
    type Query;
    type Result;
    type Error;

    fn handle<'a>(
        &'a self,
        query: Self::Query,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Result, Self::Error>> + Send + 'a>>
    where
        Self: Sync + 'a,
        Self::Query: Send + 'a,
        Self::Result: Send + 'a,
        Self::Error: Send + 'a;
}
