use crate::DomainEvent;

/// **English:** Handles a borrowed domain event synchronously.
///
/// Implementations define side effects, error and panic conditions, and duplicate handling.
/// This port supplies no acknowledgement, retry, ordering, or deduplication mechanism.
///
/// **日本語:** 借用したドメインイベントを同期的に処理します。
///
/// 副作用、エラーとパニックの条件、重複の扱いは実装が定めます。
/// このポートは確認応答、再試行、順序制御、重複排除の機構を提供しません。
pub trait EventConsumer {
    /// **English:** Domain event accepted by this consumer.
    ///
    /// **日本語:** このコンシューマーが受け付けるドメインイベントです。
    type Event: DomainEvent;
    /// **English:** Failure reported by the consuming implementation.
    ///
    /// **日本語:** 処理の実装が報告する失敗です。
    type Error;

    /// **English:** Processes `event`, returning `Ok(())` on completion or the handler's error.
    /// An error does not imply that earlier side effects were rolled back.
    ///
    /// **日本語:** `event` を処理し、完了時に `Ok(())`、失敗時にハンドラーのエラーを返します。
    /// エラーは、それ以前の副作用のロールバックを意味しません。
    fn consume(&mut self, event: &Self::Event) -> Result<(), Self::Error>;
}

/// **English:** Applies domain events to a read model synchronously.
///
/// Storage, event ordering, idempotency, and panic conditions belong to the implementation;
/// this trait does not replay or persist events itself.
///
/// **日本語:** ドメインイベントを読み取りモデルへ同期的に適用します。
///
/// 保存方法、イベント順序、冪等性、パニック条件は実装が定めます。
/// このトレイト自体はイベントの再生や永続化を行いません。
pub trait Projection {
    /// **English:** Event type used to update the read model.
    ///
    /// **日本語:** 読み取りモデルの更新に使うイベント型です。
    type Event: DomainEvent;
    /// **English:** Failure produced while updating the projection.
    ///
    /// **日本語:** プロジェクションの更新中に生じる失敗です。
    type Error;

    /// **English:** Updates the projection from `event` or returns an implementation error.
    /// Rollback of partial updates is not specified by this signature.
    ///
    /// **日本語:** `event` からプロジェクションを更新するか、実装のエラーを返します。
    /// 部分的な更新のロールバックは、このシグネチャでは規定しません。
    fn project(&mut self, event: &Self::Event) -> Result<(), Self::Error>;
}
