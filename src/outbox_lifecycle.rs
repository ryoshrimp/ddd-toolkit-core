#![allow(clippy::type_complexity)]

use crate::{AggregateRoot, AppendError, AppendResult, ExpectedRevision, TransactionalOutbox};

/// **English:** Failure from an enqueue attempt or rejection of a repeated attempt.
///
/// **日本語:** 登録の試行による失敗、または繰り返しの試行に対する拒否です。
#[derive(Debug, PartialEq, Eq)]
pub enum TransactionalOutboxOperationError<E> {
    /// **English:** The outbox's append error, preserved without restoring pulled events.
    /// This includes validation and revision errors, not only storage failures.
    ///
    /// **日本語:** 取り出したイベントを復元せずに保持するアウトボックスの追加エラーです。
    /// ストレージ障害だけでなく、検証やリビジョンのエラーも含みます。
    Adapter(AppendError<E>),

    /// **English:** An attempt has already started in this context, whether it succeeded or failed.
    /// The rejected call neither pulls events nor invokes the outbox again.
    ///
    /// **日本語:** このコンテキストで試行が開始済みであることを示します。成功・失敗は問いません。
    /// 拒否された呼び出しは、イベントの取り出しもアウトボックスの再呼び出しも行いません。
    AlreadyEnqueued,
}

/// **English:** Reason that consuming a lifecycle context did not confirm successful enqueueing.
///
/// **日本語:** ライフサイクルコンテキストの消費時に登録成功を確認できなかった理由です。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionalOutboxCompletionError {
    /// **English:** No enqueue attempt was made before completion.
    ///
    /// **日本語:** 完了確認までに登録が試行されていません。
    MissingEnqueue,

    /// **English:** An attempt started but did not record success; pulled events are not restored.
    /// This state is set before pulling events, so it also covers an unwound attempt if the caller
    /// catches a panic and later completes the context.
    ///
    /// **日本語:** 試行が開始されましたが、成功が記録されていません。取り出したイベントは復元しません。
    /// この状態はイベントの取り出し前に設定するため、呼び出し側がパニックを捕捉して後で
    /// 完了確認した場合は、巻き戻された試行も含みます。
    EnqueueFailedConsumed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OperationState {
    Pending,
    Succeeded,
    FailedConsumed,
}

/// **English:** Tracks one save-and-enqueue attempt while exclusively borrowing an aggregate and outbox.
///
/// [`Self::save_and_enqueue`] pulls pending events and passes ownership to the outbox. An error
/// does not restore them, and a second attempt in the same context is rejected. The supplied
/// expected revision is forwarded unchanged. Transactions, durability, and concurrent-writer
/// coordination remain the responsibility of [`TransactionalOutbox`]'s implementation.
/// Calling [`Self::complete`] checks recorded success; it does not commit a transaction.
/// Dropping this context performs no completion check, rollback, or event restoration.
///
/// **日本語:** 集約とアウトボックスを排他的に借用し、保存・登録の一回の試行を追跡します。
///
/// [`Self::save_and_enqueue`] は保留イベントを取り出してアウトボックスへ所有権を渡します。
/// エラー時も復元せず、同じコンテキストでの二回目の試行は拒否します。指定された期待
/// リビジョンは変更せずに渡します。トランザクション、永続性、並行書き込みの調整は
/// [`TransactionalOutbox`] の実装の責務です。[`Self::complete`] は記録された成功を
/// 確認するだけで、コミットは行いません。コンテキストの破棄では完了確認、ロールバック、
/// イベント復元のいずれも行いません。
///
/// **English:** A helper can propagate an enqueue error and check completion only after success.
/// The assertion below follows from the context recording a successful outbox return.
///
/// **日本語:** ヘルパーでは登録エラーを伝播し、成功後にのみ完了確認できます。
/// 以下のアサーションは、アウトボックスが成功を返したことが記録されるため成立します。
///
/// ```rust
/// use ddd_toolkit_core::{
///     AggregateRoot, AppendResult, ExpectedRevision, TransactionalOutbox,
///     TransactionalOutboxLifecycleContext, TransactionalOutboxOperationError,
/// };
///
/// fn enqueue<A, O>(
///     aggregate: &mut A,
///     outbox: &mut O,
///     expected: ExpectedRevision,
/// ) -> Result<AppendResult, TransactionalOutboxOperationError<O::Error>>
/// where
///     A: AggregateRoot,
///     O: TransactionalOutbox<Aggregate = A>,
/// {
///     let mut context = TransactionalOutboxLifecycleContext::new(aggregate, outbox, expected);
///     let result = context.save_and_enqueue()?;
///     assert_eq!(context.complete(), Ok(()));
///     Ok(result)
/// }
/// ```
pub struct TransactionalOutboxLifecycleContext<'a, A, O>
where
    A: AggregateRoot,
    O: TransactionalOutbox<Aggregate = A>,
{
    aggregate: &'a mut A,
    outbox: &'a mut O,
    expected: ExpectedRevision,
    state: OperationState,
}

impl<'a, A, O> TransactionalOutboxLifecycleContext<'a, A, O>
where
    A: AggregateRoot,
    O: TransactionalOutbox<Aggregate = A>,
{
    /// **English:** Borrows `aggregate` and `outbox` with no attempt yet recorded.
    /// Construction neither pulls events nor calls the outbox; `expected` is saved for the attempt.
    ///
    /// **日本語:** `aggregate` と `outbox` を借用し、未試行の状態で作成します。
    /// 構築時にはイベントを取り出さず、アウトボックスも呼び出しません。`expected` は試行用に保持します。
    pub fn new(aggregate: &'a mut A, outbox: &'a mut O, expected: ExpectedRevision) -> Self {
        Self {
            aggregate,
            outbox,
            expected,
            state: OperationState::Pending,
        }
    }

    /// **English:** Pulls events once and calls the outbox with the aggregate, revision, and batch.
    ///
    /// Returns the outbox's [`AppendResult`] or wraps its [`AppendError`] without changing it.
    /// Even an empty batch is passed to the outbox for validation. Once an attempt starts,
    /// later calls return [`TransactionalOutboxOperationError::AlreadyEnqueued`], including
    /// after failure. Pulled events are not recorded back into the aggregate on any error.
    /// Panics from [`AggregateRoot::pull_events`] or the outbox propagate; the context marks
    /// the attempt as unsuccessful before either call and provides no panic recovery.
    ///
    /// **日本語:** イベントを一回取り出し、集約、リビジョン、バッチをアウトボックスへ渡します。
    ///
    /// アウトボックスの [`AppendResult`] を返すか、[`AppendError`] を変更せずに包んで返します。
    /// 空バッチも検証のためにアウトボックスへ渡します。一度試行を開始すると、失敗後も含め、
    /// 後続の呼び出しは [`TransactionalOutboxOperationError::AlreadyEnqueued`] を返します。
    /// いずれのエラーでも取り出したイベントは集約へ記録し直しません。
    /// [`AggregateRoot::pull_events`] やアウトボックスのパニックは伝播します。両呼び出しの前に
    /// 試行を未成功として記録しますが、パニックからの回復処理は提供しません。
    pub fn save_and_enqueue(
        &mut self,
    ) -> Result<AppendResult, TransactionalOutboxOperationError<O::Error>> {
        if self.state != OperationState::Pending {
            return Err(TransactionalOutboxOperationError::AlreadyEnqueued);
        }
        self.state = OperationState::FailedConsumed;
        let events = self.aggregate.pull_events();
        match self
            .outbox
            .save_and_enqueue(self.aggregate, self.expected, events)
        {
            Ok(result) => {
                self.state = OperationState::Succeeded;
                Ok(result)
            }
            Err(error) => Err(TransactionalOutboxOperationError::Adapter(error)),
        }
    }

    /// **English:** Consumes the context and reports whether its enqueue attempt succeeded.
    ///
    /// Returns `Ok(())` after success, [`TransactionalOutboxCompletionError::MissingEnqueue`]
    /// before any attempt, or [`TransactionalOutboxCompletionError::EnqueueFailedConsumed`]
    /// after an attempt without recorded success. No storage call or retry is performed;
    /// rejecting a second enqueue does not erase a previously recorded success.
    ///
    /// **日本語:** コンテキストを消費し、登録の試行が成功したかを報告します。
    ///
    /// 成功後は `Ok(())`、未試行なら [`TransactionalOutboxCompletionError::MissingEnqueue`]、
    /// 成功が記録されていない試行の後なら
    /// [`TransactionalOutboxCompletionError::EnqueueFailedConsumed`] を返します。
    /// ストレージの呼び出しや再試行は行いません。二回目の登録を拒否しても、記録済みの成功は消しません。
    pub fn complete(self) -> Result<(), TransactionalOutboxCompletionError> {
        match self.state {
            OperationState::Succeeded => Ok(()),
            OperationState::Pending => Err(TransactionalOutboxCompletionError::MissingEnqueue),
            OperationState::FailedConsumed => {
                Err(TransactionalOutboxCompletionError::EnqueueFailedConsumed)
            }
        }
    }
}

/// **English:** Short name for [`TransactionalOutboxLifecycleContext`] with identical borrow and retry rules.
///
/// **日本語:** 借用と再試行の規則が同じ [`TransactionalOutboxLifecycleContext`] の短い別名です。
pub type OutboxLifecycleContext<'a, A, O> = TransactionalOutboxLifecycleContext<'a, A, O>;

/// **English:** Alias for [`TransactionalOutboxOperationError`], including append failures and repeated attempts.
///
/// **日本語:** 追加失敗と繰り返しの試行を表す [`TransactionalOutboxOperationError`] の別名です。
pub type OutboxLifecycleOperationError<E> = TransactionalOutboxOperationError<E>;

/// **English:** Alias for [`TransactionalOutboxCompletionError`] used when checking lifecycle completion.
///
/// **日本語:** ライフサイクルの完了確認に使う [`TransactionalOutboxCompletionError`] の別名です。
pub type OutboxLifecycleCompletionError = TransactionalOutboxCompletionError;
