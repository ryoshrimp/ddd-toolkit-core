/// **English:** Explicit registration, commit, and rollback operations for a set of changes.
/// Implementations define transaction boundaries, atomicity, ordering, and whether repeated
/// commits or rollback after commit are valid. This trait tracks no lifecycle state and provides
/// no automatic rollback on drop or error. It imposes no `Send` or `Sync` bounds.
///
/// **日本語:** 変更の登録、コミット、ロールバックを明示的に行う操作です。
/// トランザクション境界、原子性、順序、コミットの繰り返しやコミット後のロールバックの可否は
/// 実装が定めます。このトレイトはライフサイクル状態を追跡せず、破棄やエラー時の自動ロールバックも
/// 提供しません。`Send` や `Sync` の境界は課しません。
pub trait UnitOfWork {
    /// **English:** Owned operation or data accepted for participation in this unit of work.
    ///
    /// **日本語:** この作業単位に参加させる、所有された操作またはデータです。
    type Change;

    /// **English:** Shared error type for registration, commit, and rollback failures.
    ///
    /// **日本語:** 登録、コミット、ロールバックの失敗に共通するエラー型です。
    type Error;

    /// **English:** Consumes `change` to register it, or reports a registration error.
    /// Whether the change takes effect immediately or is buffered, and what remains on error,
    /// are implementation-defined; ownership is not returned by this signature.
    ///
    /// **日本語:** `change` を消費して登録するか、登録エラーを返します。
    /// 即時に反映するかバッファするかと、エラー時に残る状態は実装が定めます。
    /// このシグネチャでは所有権を返しません。
    fn enlist(&mut self, change: Self::Change) -> Result<(), Self::Error>;

    /// **English:** Requests committing the registered work and returns success or a commit error.
    /// The implementation defines durability and partial-failure recovery; an error alone does
    /// not establish that no changes were persisted.
    ///
    /// **日本語:** 登録した作業のコミットを要求し、成功またはコミットエラーを返します。
    /// 永続性と部分失敗からの回復は実装が定めます。エラーだけでは、変更が一切永続化されていないと
    /// 判断できません。
    fn commit(&mut self) -> Result<(), Self::Error>;

    /// **English:** Requests discarding or undoing work, returning a rollback error if it fails.
    /// The implementation specifies which changes can be undone and whether another attempt is valid.
    ///
    /// **日本語:** 作業の破棄や取り消しを要求し、失敗時にはロールバックエラーを返します。
    /// 取り消せる変更の範囲と再試行の可否は実装が定めます。
    fn rollback(&mut self) -> Result<(), Self::Error>;
}
