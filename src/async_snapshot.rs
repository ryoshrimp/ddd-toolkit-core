#![allow(clippy::type_complexity)]

use crate::{AggregateRoot, AggregateSnapshot, Entity, SnapshotLoadError, SnapshotSaveError};
use std::future::Future;
use std::pin::Pin;

/// **English:** Asynchronous storage for snapshots, separate from event-stream appends.
///
/// Futures are boxed and `Send`, borrowing the store and identifier for `'a`.
/// Adapters define replacement, durability, concurrency, panic conditions, and cancellation
/// outcomes. This port does not verify snapshots against an event stream or provide an executor.
///
/// **日本語:** イベントストリームへの追加とは別にスナップショットを非同期保存します。
///
/// Future はボックス化された `Send` で、ストアと識別子を `'a` の間借用します。
/// 置換、永続性、並行処理、パニック条件、キャンセルの結果はアダプターが定めます。
/// このポートはストリームに対するスナップショット検証や実行器を提供しません。
pub trait AsyncSnapshotStore
where
    Self::State: Send,
    Self::Error: Send,
    <Self::Aggregate as Entity>::Id: Sync,
{
    /// **English:** Aggregate defining the snapshot identifier, whose type must be `Sync`.
    ///
    /// **日本語:** スナップショット識別子を定める集約です。識別子の型には `Sync` が必要です。
    type Aggregate: AggregateRoot + Send;

    /// **English:** Owned snapshot state transferable across threads.
    ///
    /// **日本語:** スレッド間で受け渡せる所有されたスナップショット状態です。
    type State: Send;

    /// **English:** Adapter failure carried by snapshot load and save errors.
    ///
    /// **日本語:** スナップショットの読み込み・保存エラーが保持するアダプター障害です。
    type Error: Send;

    /// **English:** Returns a future loading the snapshot for `id`; `Ok(None)` means absence.
    /// Storage failures are reported through [`SnapshotLoadError::Adapter`].
    ///
    /// **日本語:** `id` のスナップショットを読み込む Future を返し、`Ok(None)` は不在を表します。
    /// ストレージ障害は [`SnapshotLoadError::Adapter`] で報告します。
    fn load<'a>(
        &'a self,
        id: &'a <Self::Aggregate as Entity>::Id,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        Option<AggregateSnapshot<Self::State>>,
                        SnapshotLoadError<Self::Error>,
                    >,
                > + Send
                + 'a,
        >,
    >;

    /// **English:** Takes ownership of `snapshot` and returns a future saving it for `id`.
    /// Completion is `Ok(())` or [`SnapshotSaveError::Adapter`]; the snapshot is not returned
    /// on failure. Neither an error nor cancellation specifies whether storage changed.
    ///
    /// **日本語:** `snapshot` の所有権を受け取り、`id` に対して保存する Future を返します。
    /// 結果は `Ok(())` または [`SnapshotSaveError::Adapter`] で、失敗時に値は返されません。
    /// エラーやキャンセルだけではストレージが変更されたかどうかは決まりません。
    fn save<'a>(
        &'a mut self,
        id: &'a <Self::Aggregate as Entity>::Id,
        snapshot: AggregateSnapshot<Self::State>,
    ) -> Pin<Box<dyn Future<Output = Result<(), SnapshotSaveError<Self::Error>>> + Send + 'a>>;
}
