#![allow(clippy::type_complexity)]

use crate::{AggregateRoot, AppendError, AppendResult, AsyncTransactionalOutbox, ExpectedRevision};
use std::future::Future;
use std::pin::Pin;

/// **English:** Failure of an asynchronous enqueue attempt or rejection of a repeated attempt.
///
/// **日本語:** 非同期登録の試行による失敗、または繰り返しの試行に対する拒否です。
#[derive(Debug, PartialEq, Eq)]
pub enum AsyncTransactionalOutboxOperationError<E> {
    /// **English:** The outbox's append error, including validation, revision, or storage failures.
    /// Pulled events are not restored when this error is returned.
    ///
    /// **日本語:** 検証、リビジョン、ストレージの失敗を含むアウトボックスの追加エラーです。
    /// このエラーを返す際、取り出したイベントは復元しません。
    Adapter(AppendError<E>),

    /// **English:** A previous call started an attempt, even if its future was never polled.
    /// The rejected call does not pull events or invoke the adapter.
    ///
    /// **日本語:** 以前の呼び出しで試行を開始済みです。その Future が未ポーリングでも該当します。
    /// 拒否された呼び出しでは、イベントを取り出さず、アダプターも呼び出しません。
    AlreadyEnqueued,
}

/// **English:** Reason completion could not confirm a successful asynchronous enqueue.
///
/// **日本語:** 完了確認で非同期登録の成功を確認できなかった理由です。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsyncTransactionalOutboxCompletionError {
    /// **English:** No call to enqueue was made before completing the context.
    ///
    /// **日本語:** コンテキストの完了確認までに登録が呼び出されていません。
    MissingEnqueue,

    /// **English:** An attempt started without recorded success, including an error or dropped future.
    /// The state is set before pulling events, so a caught panic can also leave this result.
    ///
    /// **日本語:** エラーや Future の破棄などにより、試行開始後に成功が記録されていません。
    /// イベントの取り出し前にこの状態を設定するため、パニックを捕捉した後もこの結果になり得ます。
    EnqueueFailedConsumed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OperationState {
    Pending,
    Succeeded,
    FailedConsumed,
}

/// **English:** Single-attempt asynchronous enqueue tracking with exclusive aggregate and outbox borrows.
///
/// Calling [`Self::save_and_enqueue`] consumes the attempt and pulls events immediately, before
/// its future is polled. Failure or dropping that future does not restore events or enable retry.
/// Only a successful adapter result records success. [`Self::complete`] checks this record;
/// it does not commit a transaction. Dropping the context performs no check or rollback.
/// Transaction and cancellation effects on storage belong to the outbox adapter.
///
/// The aggregate must be `Send + Sync`, its event must be `Send`, and the outbox and error must
/// be `Send` to support the returned `Send` future. No executor or locking is provided.
///
/// **日本語:** 集約とアウトボックスを排他的に借用し、非同期登録の一回の試行を追跡します。
///
/// [`Self::save_and_enqueue`] の呼び出し時に、Future のポーリング前に試行を消費して
/// イベントを取り出します。失敗や Future の破棄でもイベントは復元せず、再試行もできません。
/// アダプターの成功結果だけが成功を記録します。[`Self::complete`] はこの記録を検査するだけで、
/// コミットしません。コンテキストの破棄でも検査やロールバックは行いません。
/// トランザクションとキャンセルがストレージに及ぼす効果はアダプターが定めます。
///
/// `Send` な Future のため、集約には `Send + Sync`、イベント、アウトボックス、エラーには
/// `Send` が必要です。実行器やロックは提供しません。
///
/// **English:** Await a successful enqueue before consuming the context to check completion.
/// The assertion follows from recording success after the adapter returns `Ok`.
///
/// **日本語:** 登録成功を待ってからコンテキストを消費して完了確認します。
/// アダプターが `Ok` を返した後に成功を記録するため、以下のアサーションが成立します。
///
/// ```rust
/// use ddd_toolkit_core::{
///     AggregateRoot, AppendResult, AsyncTransactionalOutbox,
///     AsyncTransactionalOutboxLifecycleContext, AsyncTransactionalOutboxOperationError,
///     ExpectedRevision,
/// };
///
/// async fn enqueue<A, O>(
///     aggregate: &mut A,
///     outbox: &mut O,
///     expected: ExpectedRevision,
/// ) -> Result<AppendResult, AsyncTransactionalOutboxOperationError<O::Error>>
/// where
///     A: AggregateRoot + Send + Sync,
///     A::Event: Send,
///     O: AsyncTransactionalOutbox<Aggregate = A> + Send,
/// {
///     let mut context = AsyncTransactionalOutboxLifecycleContext::new(aggregate, outbox, expected);
///     let result = context.save_and_enqueue().await?;
///     assert_eq!(context.complete(), Ok(()));
///     Ok(result)
/// }
/// ```
pub struct AsyncTransactionalOutboxLifecycleContext<'storage, A, O>
where
    A: AggregateRoot + Send + Sync,
    A::Event: Send,
    O: AsyncTransactionalOutbox<Aggregate = A> + Send,
    O::Error: Send,
{
    aggregate: &'storage mut A,
    outbox: &'storage mut O,
    expected: ExpectedRevision,
    state: OperationState,
}

impl<'storage, A, O> AsyncTransactionalOutboxLifecycleContext<'storage, A, O>
where
    A: AggregateRoot + Send + Sync,
    A::Event: Send,
    O: AsyncTransactionalOutbox<Aggregate = A> + Send,
    O::Error: Send,
{
    /// **English:** Borrows the aggregate and outbox for `'storage` and stores `expected` unchanged.
    /// The context starts pending; construction does not pull events or call the adapter.
    ///
    /// **日本語:** 集約とアウトボックスを `'storage` の間借用し、`expected` を変更せず保持します。
    /// 未試行で開始し、構築時にはイベントを取り出さず、アダプターも呼び出しません。
    pub fn new(
        aggregate: &'storage mut A,
        outbox: &'storage mut O,
        expected: ExpectedRevision,
    ) -> Self {
        Self {
            aggregate,
            outbox,
            expected,
            state: OperationState::Pending,
        }
    }

    /// **English:** Immediately reserves the attempt and pulls events, then returns a boxed `Send` future.
    ///
    /// The future borrows this context for `'ctx`. Polling it calls the adapter with the borrowed
    /// aggregate, stored revision, and owned batch, including an empty batch. It returns the
    /// [`AppendResult`] or wraps the unchanged [`AppendError`]. Dropping the future before its first
    /// poll discards the pulled batch without calling the adapter; dropping it while pending leaves
    /// the attempt unsuccessful, with storage effects adapter-defined. Neither path restores events.
    /// A later call returns a future yielding [`AsyncTransactionalOutboxOperationError::AlreadyEnqueued`].
    ///
    /// Panics from pulling events or invoking/polling the adapter propagate. State is marked
    /// unsuccessful before pulling; catching a panic does not enable another attempt.
    ///
    /// **日本語:** 呼び出し時に試行を確保してイベントを取り出し、ボックス化された `Send` Future を返します。
    ///
    /// Future はコンテキストを `'ctx` の間借用します。ポーリング時に、借用した集約、保存済み
    /// リビジョン、所有されたバッチをアダプターへ渡します。空バッチも渡します。
    /// [`AppendResult`] を返すか、変更していない [`AppendError`] を包んで返します。
    /// 初回ポーリング前の破棄ではアダプターを呼ばずにバッチを破棄します。待機中の破棄では
    /// 試行が未成功のままとなり、ストレージへの効果はアダプターに依存します。
    /// どちらもイベントを復元しません。後続の呼び出しは
    /// [`AsyncTransactionalOutboxOperationError::AlreadyEnqueued`] を返す Future を返します。
    ///
    /// イベントの取り出しやアダプターの呼び出し・ポーリングのパニックは伝播します。
    /// 取り出し前に未成功と記録するため、パニックを捕捉しても再試行できません。
    pub fn save_and_enqueue<'ctx>(
        &'ctx mut self,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<AppendResult, AsyncTransactionalOutboxOperationError<O::Error>>,
                > + Send
                + 'ctx,
        >,
    >
    where
        'storage: 'ctx,
        A: 'ctx,
        A::Event: 'ctx,
        O: 'ctx,
        O::Error: 'ctx,
    {
        if self.state != OperationState::Pending {
            return Box::pin(async {
                Err(AsyncTransactionalOutboxOperationError::AlreadyEnqueued)
            });
        }

        self.state = OperationState::FailedConsumed;
        let state = &mut self.state;
        let events = self.aggregate.pull_events();
        let aggregate: &'ctx A = &*self.aggregate;
        let outbox: &'ctx mut O = &mut *self.outbox;
        let expected = self.expected;

        Box::pin(async move {
            let mut guard = AttemptGuard::new(state);
            match outbox.save_and_enqueue(aggregate, expected, events).await {
                Ok(result) => {
                    guard.succeed();
                    Ok(result)
                }
                Err(error) => Err(AsyncTransactionalOutboxOperationError::Adapter(error)),
            }
        })
    }

    /// **English:** Consumes the context and returns `Ok(())` only after recorded adapter success.
    /// No enqueue call yields [`AsyncTransactionalOutboxCompletionError::MissingEnqueue`]; a call
    /// without recorded success yields [`AsyncTransactionalOutboxCompletionError::EnqueueFailedConsumed`].
    /// This makes no storage call. Rejection of a repeated attempt does not erase an earlier success.
    /// Any returned enqueue future must release its mutable borrow before this method is called.
    ///
    /// **日本語:** コンテキストを消費し、アダプターの成功を記録済みの場合のみ `Ok(())` を返します。
    /// 登録未呼び出しなら [`AsyncTransactionalOutboxCompletionError::MissingEnqueue`]、呼び出し済みで
    /// 未成功なら [`AsyncTransactionalOutboxCompletionError::EnqueueFailedConsumed`] を返します。
    /// ストレージは呼び出しません。再試行の拒否は以前の成功を消しません。
    /// 呼び出す前に、登録の Future が保持する可変借用を解放する必要があります。
    pub fn complete(self) -> Result<(), AsyncTransactionalOutboxCompletionError> {
        match self.state {
            OperationState::Succeeded => Ok(()),
            OperationState::Pending => Err(AsyncTransactionalOutboxCompletionError::MissingEnqueue),
            OperationState::FailedConsumed => {
                Err(AsyncTransactionalOutboxCompletionError::EnqueueFailedConsumed)
            }
        }
    }
}

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

/// **English:** Short alias for [`AsyncTransactionalOutboxLifecycleContext`] with the same cancellation rules.
///
/// **日本語:** キャンセルの規則も同じ [`AsyncTransactionalOutboxLifecycleContext`] の短い別名です。
pub type AsyncOutboxLifecycleContext<'a, A, O> = AsyncTransactionalOutboxLifecycleContext<'a, A, O>;

/// **English:** Alias for [`AsyncTransactionalOutboxOperationError`] covering enqueue and repeated-attempt errors.
///
/// **日本語:** 登録と再試行拒否のエラーを表す [`AsyncTransactionalOutboxOperationError`] の別名です。
pub type AsyncOutboxLifecycleOperationError<E> = AsyncTransactionalOutboxOperationError<E>;

/// **English:** Alias for [`AsyncTransactionalOutboxCompletionError`] used for the final state check.
///
/// **日本語:** 最終状態の確認に使う [`AsyncTransactionalOutboxCompletionError`] の別名です。
pub type AsyncOutboxLifecycleCompletionError = AsyncTransactionalOutboxCompletionError;
