#![allow(clippy::type_complexity)]

use crate::{
    AggregateRoot, DomainEventPublisher, Entity, LifecycleCompletionError, LifecycleOperationError,
    Repository, cqrs::OperationState, dispatch_events,
};

/// **English:** Coordinates one synchronous aggregate save and event dispatch sequence.
///
/// **日本語:** 同期の集約保存とイベント配信を一連で調整します。
pub struct LifecycleContext<'a, A, R, P>
where
    A: AggregateRoot,
    R: Repository<Aggregate = A>,
    P: DomainEventPublisher<Event = A::Event>,
{
    aggregate: &'a mut A,
    repository: &'a mut R,
    publisher: &'a mut P,
    save_state: OperationState,
    dispatch_state: OperationState,
}

impl<'a, A, R, P> LifecycleContext<'a, A, R, P>
where
    A: AggregateRoot,
    R: Repository<Aggregate = A>,
    P: DomainEventPublisher<Event = A::Event>,
{
    /// **English:** Creates a lifecycle context borrowing the aggregate, repository, and publisher mutably.
    ///
    /// **日本語:** 集約、リポジトリ、パブリッシャーを可変借用するライフサイクルコンテキストを作成します。
    pub fn new(aggregate: &'a mut A, repository: &'a mut R, publisher: &'a mut P) -> Self {
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
    pub fn save(&mut self) -> Result<(), LifecycleOperationError<R::Error>> {
        if self.save_state != OperationState::Pending {
            return Err(LifecycleOperationError::AlreadySaved);
        }
        self.save_state = OperationState::FailedConsumed;
        match self.repository.save(self.aggregate) {
            Ok(()) => {
                self.save_state = OperationState::Succeeded;
                Ok(())
            }
            Err(error) => Err(LifecycleOperationError::Adapter(error)),
        }
    }

    /// **English:** Attempts event dispatch once; adapter errors are returned and the attempt cannot be retried.
    ///
    /// **日本語:** イベント配信を一度試行します。アダプターエラーを返し、再試行はできません。
    pub fn dispatch(&mut self) -> Result<(), LifecycleOperationError<P::Error>> {
        if self.dispatch_state != OperationState::Pending {
            return Err(LifecycleOperationError::AlreadyDispatched);
        }
        self.dispatch_state = OperationState::FailedConsumed;
        match dispatch_events(self.aggregate, self.publisher) {
            Ok(()) => {
                self.dispatch_state = OperationState::Succeeded;
                Ok(())
            }
            Err(error) => Err(LifecycleOperationError::Adapter(error)),
        }
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

/// **English:** Defines work performed with a mutable lifecycle context. The callback result and error become the handler result and error.
///
/// **日本語:** 可変ライフサイクルコンテキストを使って実行する処理を定義します。コールバックの出力とエラーがハンドラーの出力とエラーになります。
pub trait LifecycleCallback<'storage, A, R, P>
where
    A: AggregateRoot,
    R: Repository<Aggregate = A>,
    P: DomainEventPublisher<Event = A::Event>,
{
    type Output;
    type Error;

    fn call<'callback>(
        self,
        context: &'callback mut LifecycleContext<'storage, A, R, P>,
    ) -> Result<Self::Output, Self::Error>
    where
        'storage: 'callback;
}

/// **English:** Loads an aggregate for a command, runs its lifecycle callback, and returns the callback output or an error.
///
/// **日本語:** コマンド対象の集約を読み込み、ライフサイクルコールバックを実行し、その出力またはエラーを返します。
pub trait CommandHandler {
    type Command;
    type Aggregate: AggregateRoot;
    type Repository: Repository<Aggregate = Self::Aggregate>;
    type Publisher: DomainEventPublisher<Event = <Self::Aggregate as AggregateRoot>::Event>;
    type Output;
    type Error;

    fn handle<F>(
        &mut self,
        id: &<Self::Aggregate as Entity>::Id,
        command: Self::Command,
        callback: F,
    ) -> Result<Self::Output, Self::Error>
    where
        F: for<'a> LifecycleCallback<
                'a,
                Self::Aggregate,
                Self::Repository,
                Self::Publisher,
                Output = Self::Output,
                Error = Self::Error,
            >,
        Self::Error: From<LifecycleOperationError<<Self::Repository as Repository>::Error>>
            + From<LifecycleOperationError<<Self::Publisher as DomainEventPublisher>::Error>>
            + From<LifecycleCompletionError>;
}

/// **English:** Creates an aggregate from a command and runs the lifecycle callback before returning its output.
///
/// **日本語:** コマンドから集約を作成し、ライフサイクルコールバックを実行してから出力を返します。
pub trait AggregateCreator {
    type Command;
    type Aggregate: AggregateRoot;
    type Repository: Repository<Aggregate = Self::Aggregate>;
    type Publisher: DomainEventPublisher<Event = <Self::Aggregate as AggregateRoot>::Event>;
    type Output;
    type Error;

    fn create<F>(
        &mut self,
        command: Self::Command,
        callback: F,
    ) -> Result<Self::Output, Self::Error>
    where
        F: for<'a> LifecycleCallback<
                'a,
                Self::Aggregate,
                Self::Repository,
                Self::Publisher,
                Output = Self::Output,
                Error = Self::Error,
            >,
        Self::Error: From<LifecycleOperationError<<Self::Repository as Repository>::Error>>
            + From<LifecycleOperationError<<Self::Publisher as DomainEventPublisher>::Error>>
            + From<LifecycleCompletionError>;
}

/// **English:** Handles a query without requiring aggregate lifecycle operations.
///
/// **日本語:** 集約ライフサイクル操作を伴わずにクエリを処理します。
pub trait QueryHandler {
    type Query;
    type Result;
    type Error;

    fn handle(&self, query: Self::Query) -> Result<Self::Result, Self::Error>;
}
