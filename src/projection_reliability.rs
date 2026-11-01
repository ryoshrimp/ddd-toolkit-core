use crate::DomainEvent;

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

/// **English:** Successful outcome of an [`IdempotentProjection::apply_once`] call.
///
/// **日本語:** [`IdempotentProjection::apply_once`] 呼び出しの成功結果です。
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
