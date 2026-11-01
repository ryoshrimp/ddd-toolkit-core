#![allow(clippy::type_complexity)]
use crate::DomainEvent;
use std::future::Future;
use std::pin::Pin;

/// **English:** Owned cursor identifying progress in a projection's input stream.
///
/// The cursor type and meaning belong to the application. This container performs no ordering,
/// monotonicity, or validity checks and does not store progress by itself.
///
/// **日本語:** プロジェクションの入力ストリームにおける進捗を示す、所有されたカーソルです。
///
/// カーソルの型と意味はアプリケーションが定めます。このコンテナーは順序、単調性、妥当性を
/// 検証せず、単体では進捗を保存しません。
///
/// **English:** Borrow a cursor for inspection, then recover its ownership without cloning.
///
/// **日本語:** カーソルを借用して確認した後、複製せずに所有権を取り出せます。
///
/// ```rust
/// use ddd_toolkit_core::ProjectionCheckpoint;
///
/// let checkpoint = ProjectionCheckpoint::new(String::from("partition-2:17"));
/// assert_eq!(checkpoint.cursor(), "partition-2:17");
/// let cursor = checkpoint.into_cursor();
/// assert_eq!(cursor, "partition-2:17");
/// ```
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ProjectionCheckpoint<C> {
    cursor: C,
}

impl<C> ProjectionCheckpoint<C> {
    /// **English:** Wraps `cursor` without validating its position or comparing previous progress.
    ///
    /// **日本語:** 位置の検証や以前の進捗との比較をせずに `cursor` を包みます。
    pub fn new(cursor: C) -> Self {
        Self { cursor }
    }

    /// **English:** Borrows the stored cursor without advancing it.
    ///
    /// **日本語:** 進めることなく、保存されたカーソルを借用します。
    pub fn cursor(&self) -> &C {
        &self.cursor
    }

    /// **English:** Consumes the checkpoint and returns its cursor by value.
    ///
    /// **日本語:** チェックポイントを消費し、カーソルを値として返します。
    pub fn into_cursor(self) -> C {
        self.cursor
    }
}

/// **English:** Successful outcome of [`IdempotentProjection::apply_once`] or
/// [`AsyncIdempotentProjection::apply_once`], distinguishing application from a recognized duplicate.
///
/// **日本語:** [`IdempotentProjection::apply_once`] または [`AsyncIdempotentProjection::apply_once`] の
/// 成功結果であり、適用と認識済みの重複を区別します。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectionApplication {
    /// **English:** The implementation reports that this call applied the event.
    ///
    /// **日本語:** この呼び出しでイベントを適用したことを実装が報告しています。
    Applied,

    /// **English:** The implementation recognized an already-applied input and skipped reapplication.
    ///
    /// **日本語:** 適用済みの入力を実装が認識し、再適用を省略しました。
    AlreadyApplied,
}

/// **English:** Synchronous persistence port for a projection's checkpoint.
///
/// With no projection identifier parameter, the store instance determines the checkpoint scope.
/// Implementations define replacement, durability, concurrency, panic conditions, and whether
/// saving progress is atomic with read-model updates. This trait supplies no monotonicity check.
///
/// **日本語:** プロジェクションのチェックポイントを同期永続化するポートです。
///
/// プロジェクション識別子の引数がないため、保存対象の範囲はストアのインスタンスが定めます。
/// 置換、永続性、並行処理、パニック条件、読み取りモデル更新との原子的な進捗保存は
/// 実装が定めます。このトレイトは単調性の検証を提供しません。
pub trait ProjectionCheckpointStore {
    /// **English:** Application-defined position representation persisted by this store.
    ///
    /// **日本語:** このストアが永続化する、アプリケーション定義の位置表現です。
    type Cursor;
    /// **English:** Checkpoint storage failure reported by the adapter.
    ///
    /// **日本語:** アダプターが報告するチェックポイント保存先の障害です。
    type Error;

    /// **English:** Returns stored progress, `Ok(None)` for absence, or a storage error.
    /// Absence is distinct from failing to read the checkpoint.
    ///
    /// **日本語:** 保存済み進捗、不在を表す `Ok(None)`、またはストレージエラーを返します。
    /// 不在とチェックポイントの読み込み失敗は区別します。
    fn load(&self) -> Result<Option<ProjectionCheckpoint<Self::Cursor>>, Self::Error>;

    /// **English:** Consumes `checkpoint` to persist progress, returning `Ok(())` or a storage error.
    /// The value is not returned on failure; retry and partial-write behavior depend on the adapter.
    ///
    /// **日本語:** `checkpoint` を消費して進捗を保存し、`Ok(())` またはストレージエラーを返します。
    /// 失敗時に値は返されません。再試行と部分書き込みの挙動はアダプターに依存します。
    fn save(&mut self, checkpoint: ProjectionCheckpoint<Self::Cursor>) -> Result<(), Self::Error>;
}

/// **English:** Opt-in projection contract distinguishing new application from duplicate input.
///
/// This trait is independent of [`crate::Projection`] and [`ProjectionCheckpointStore`].
/// Implementations supply duplicate detection, input identity, read-model changes, and any
/// atomic checkpoint updates or locking. No deduplication algorithm is implemented here;
/// adapters must define failure, retry, and panic behavior for their persistence boundary.
///
/// **日本語:** 新規適用と重複入力を区別する、任意のプロジェクション契約です。
///
/// このトレイトは [`crate::Projection`] と [`ProjectionCheckpointStore`] から独立しています。
/// 重複検出、入力の識別、読み取りモデルの変更、原子的なチェックポイント更新やロックは
/// 実装が提供します。ここでは重複排除アルゴリズムを実装しません。永続化境界における
/// 失敗、再試行、パニック時の挙動はアダプターが定める必要があります。
pub trait IdempotentProjection {
    /// **English:** Domain event inspected for application to the read model.
    ///
    /// **日本語:** 読み取りモデルへの適用を判断するドメインイベントです。
    type Event: DomainEvent;
    /// **English:** Position type used to associate input with projection progress.
    ///
    /// **日本語:** 入力とプロジェクションの進捗を関連付ける位置の型です。
    type Cursor;
    /// **English:** Error raised while detecting duplicates or applying an event.
    ///
    /// **日本語:** 重複の検出やイベント適用中に生じるエラーです。
    type Error;

    /// **English:** Borrows `event` and `checkpoint` and reports application or a recognized duplicate.
    /// An error does not specify whether partial updates occurred. This call does not automatically
    /// invoke a separate checkpoint store; checkpoint meaning and persistence belong to the implementation.
    ///
    /// **日本語:** `event` と `checkpoint` を借用し、適用または認識済みの重複を報告します。
    /// エラーだけでは部分更新の有無は決まりません。別のチェックポイントストアを自動では
    /// 呼び出さず、チェックポイントの意味と永続化は実装が定めます。
    fn apply_once(
        &mut self,
        event: &Self::Event,
        checkpoint: &ProjectionCheckpoint<Self::Cursor>,
    ) -> Result<ProjectionApplication, Self::Error>;
}

/// **English:** Checkpoint persistence port returning boxed `Send` futures tied to the store borrow.
///
/// The store instance determines the projection scope, as in [`ProjectionCheckpointStore`].
/// The store must be `Send + Sync`, the cursor `Send + Sync`, and the error `Send`; these bounds
/// do not provide locking, monotonic progress checks, or atomic read-model updates. Implementations
/// define durability, concurrent replacement, partial failures, and panic conditions.
///
/// Dropping a returned future ends that operation's polling, but does not promise rollback or that
/// no I/O occurred. Implementations may perform work before returning a future and must specify
/// cancellation and retry behavior. This trait supplies no executor or transaction coordinator.
///
/// **日本語:** ストアの借用期間に結び付いた、ボックス化された `Send` Future を返す進捗永続化ポートです。
///
/// [`ProjectionCheckpointStore`] と同様に、プロジェクションの範囲はストアのインスタンスが定めます。
/// ストアには `Send + Sync`、カーソルには `Send + Sync`、エラーには `Send` が必要ですが、
/// これらの境界はロック、単調な進捗の検査、読み取りモデルの原子的更新を提供しません。
/// 永続性、並行する置換、部分失敗、パニック条件は実装が定めます。
///
/// 返された Future の破棄はその操作のポーリングを終了しますが、ロールバックや I/O が未実行であることを
/// 保証しません。実装は Future を返す前にも処理でき、キャンセルと再試行の挙動を定める必要があります。
/// このトレイトは実行器やトランザクション調整器を提供しません。
///
/// **English:** Await a write and then read its result; the adapter determines read-after-write consistency.
/// This helper returns `None` if the subsequent load reports no stored checkpoint.
///
/// **日本語:** 書き込みを待ってから結果を読み込みます。書き込み後の読み取り整合性はアダプターが定めます。
/// 後続の読み込みがチェックポイントの不在を報告した場合、このヘルパーは `None` を返します。
///
/// ```rust
/// use ddd_toolkit_core::{AsyncProjectionCheckpointStore, ProjectionCheckpoint};
///
/// async fn save_then_load<S: AsyncProjectionCheckpointStore>(
///     store: &mut S,
///     cursor: S::Cursor,
/// ) -> Result<Option<S::Cursor>, S::Error> {
///     store.save(ProjectionCheckpoint::new(cursor)).await?;
///     Ok(store.load().await?.map(ProjectionCheckpoint::into_cursor))
/// }
/// ```
pub trait AsyncProjectionCheckpointStore: Send + Sync {
    /// **English:** Owned progress representation transferable and shareable across threads.
    /// No ordering or clone bound is required.
    ///
    /// **日本語:** スレッド間で移動・共有できる、所有された進捗表現です。
    /// 順序や複製の境界は要求しません。
    type Cursor: Send + Sync;
    /// **English:** Storage failure transferable with the future's result; no error trait bound is imposed.
    ///
    /// **日本語:** Future の結果とともに移動できるストレージの失敗です。エラートレイトの境界は課しません。
    type Error: Send;

    /// **English:** Borrows the store for `'a` and returns a future yielding owned stored progress.
    /// `Ok(None)` means absence, while `Err` reports a read failure. Shared access permits multiple
    /// load borrows but does not define snapshot isolation or freshness. Drop effects are adapter-defined.
    ///
    /// **日本語:** ストアを `'a` の間借用し、所有された保存済み進捗を返す Future を返します。
    /// `Ok(None)` は不在を表し、`Err` は読み込み失敗を表します。共有参照で複数の読み込みを借用できますが、
    /// スナップショット分離や鮮度は定めません。破棄による効果はアダプターが定めます。
    fn load<'a>(
        &'a self,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<Option<ProjectionCheckpoint<Self::Cursor>>, Self::Error>>
                + Send
                + 'a,
        >,
    >;

    /// **English:** Takes ownership of `checkpoint` and exclusively borrows the store for `'a`.
    /// The returned future reports `Ok(())` or a write failure, without returning the checkpoint
    /// on error or cancellation. No rollback, retry, or atomic coordination with a projection is added.
    ///
    /// **日本語:** `checkpoint` の所有権を受け取り、ストアを `'a` の間排他的に借用します。
    /// 返す Future は `Ok(())` または書き込み失敗を報告し、エラーやキャンセル時にチェックポイントを
    /// 返しません。ロールバック、再試行、プロジェクションとの原子的な連携は追加しません。
    fn save<'a>(
        &'a mut self,
        checkpoint: ProjectionCheckpoint<Self::Cursor>,
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'a>>;
}

/// **English:** Asynchronous duplicate-aware projection independent of [`crate::AsyncProjection`]
/// and [`AsyncProjectionCheckpointStore`].
///
/// Implementations provide input identity, duplicate detection, read-model updates, and any atomic
/// checkpoint persistence. The trait itself implements no deduplication. The projection must be `Send`,
/// its event `Sync`, its cursor `Send + Sync`, and its error `Send` for the boxed `Send` future.
/// The projection need not be `Sync`; mutable borrowing prevents overlapping calls on the same value.
///
/// Dropping a future stops polling but does not establish whether changes occurred or were rolled back,
/// even before the first poll: implementations may act during the method call. Adapters must document
/// cancellation, retry, partial-update, and panic behavior at their persistence boundary.
///
/// **日本語:** [`crate::AsyncProjection`] と [`AsyncProjectionCheckpointStore`] から独立した、
/// 重複を区別する非同期プロジェクションです。
///
/// 入力の識別、重複検出、読み取りモデル更新、原子的なチェックポイント永続化は実装が提供します。
/// トレイト自体は重複排除を実装しません。ボックス化された `Send` Future のため、
/// プロジェクションには `Send`、イベントには `Sync`、カーソルには `Send + Sync`、エラーには `Send` が必要です。
/// プロジェクション自体に `Sync` は不要です。同じ値への呼び出しの重複は可変借用が防ぎます。
///
/// Future の破棄はポーリングを止めますが、変更の有無やロールバックを保証しません。
/// メソッド呼び出し中にも処理できるため、初回ポーリング前でも同様です。永続化境界における
/// キャンセル、再試行、部分更新、パニックの挙動はアダプターが文書化する必要があります。
pub trait AsyncIdempotentProjection: Send {
    /// **English:** Domain event shareable through its borrowed reference in a `Send` future.
    ///
    /// **日本語:** `Send` Future 内の借用参照を通して共有できるドメインイベントです。
    type Event: DomainEvent + Sync;
    /// **English:** Progress representation used to associate the borrowed checkpoint with input.
    ///
    /// **日本語:** 借用したチェックポイントを入力に関連付ける進捗表現です。
    type Cursor: Send + Sync;
    /// **English:** Transferable failure from duplicate detection or read-model application.
    ///
    /// **日本語:** 重複検出や読み取りモデルへの適用で生じる、移動可能な失敗です。
    type Error: Send;

    /// **English:** Borrows the projection exclusively and the event and checkpoint jointly for `'a`.
    /// The future reports [`ProjectionApplication::Applied`], [`ProjectionApplication::AlreadyApplied`],
    /// or an error that does not rule out partial updates. It does not automatically call a checkpoint
    /// store. Checkpoint interpretation and whether progress is persisted belong to the implementation.
    ///
    /// **日本語:** プロジェクションを排他的に、イベントとチェックポイントを共有で、いずれも `'a` の間借用します。
    /// Future は [`ProjectionApplication::Applied`]、[`ProjectionApplication::AlreadyApplied`]、
    /// または部分更新の可能性を排除しないエラーを返します。チェックポイントストアは自動では呼び出しません。
    /// チェックポイントの解釈と進捗を永続化するかどうかは実装が定めます。
    fn apply_once<'a>(
        &'a mut self,
        event: &'a Self::Event,
        checkpoint: &'a ProjectionCheckpoint<Self::Cursor>,
    ) -> Pin<Box<dyn Future<Output = Result<ProjectionApplication, Self::Error>> + Send + 'a>>;
}
