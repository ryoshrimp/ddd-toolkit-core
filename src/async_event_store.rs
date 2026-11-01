#![allow(clippy::type_complexity)]

use crate::{AggregateRoot, AppendError, AppendResult, Entity, ExpectedRevision, LoadResult};
use std::future::Future;
use std::pin::Pin;

/// **English:** Asynchronous event-stream storage using the result types of [`crate::EventStore`].
///
/// Returned futures are `Send` and borrow the store and identifier for `'a`; no executor or
/// synchronization is provided. Adapters define atomicity, durability, writer coordination,
/// panic conditions, and the outcome of cancelling an operation by dropping its future.
/// Cancellation does not by itself imply that writes were rolled back.
///
/// **日本語:** [`crate::EventStore`] と同じ結果型を使う非同期イベントストリームストアです。
///
/// 返される Future は `Send` で、ストアと識別子を `'a` の間借用します。
/// 実行器や同期機構は提供しません。原子性、永続性、書き込みの調整、パニック条件、
/// Future の破棄によるキャンセルの結果はアダプターが定めます。
/// キャンセル自体は書き込みのロールバックを意味しません。
pub trait AsyncEventStore
where
    <Self::Aggregate as AggregateRoot>::Event: Send,
    <Self::Aggregate as Entity>::Id: Sync,
{
    /// **English:** Aggregate supplying the stream identifier and transferable event type.
    /// Its identifier must be `Sync`, and its events must be `Send`.
    ///
    /// **日本語:** ストリーム識別子と受け渡し可能なイベント型を定める集約です。
    /// 識別子には `Sync`、イベントには `Send` が必要です。
    type Aggregate: AggregateRoot + Send;

    /// **English:** Storage error transferable with the future's result.
    ///
    /// **日本語:** Future の結果とともに受け渡せるストレージエラーです。
    type Error: Send;

    /// **English:** Loads the complete history for `id` or returns [`LoadResult::NoStream`].
    /// Storage failures use `Self::Error`, not the absent-stream result.
    ///
    /// **日本語:** `id` の完全な履歴、または [`LoadResult::NoStream`] を返します。
    /// ストレージ障害はストリーム不在ではなく `Self::Error` で報告します。
    fn load<'a>(
        &'a self,
        id: &'a <Self::Aggregate as Entity>::Id,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        LoadResult<<Self::Aggregate as AggregateRoot>::Event>,
                        Self::Error,
                    >,
                > + Send
                + 'a,
        >,
    >;

    /// **English:** Takes ownership of `events` to append in vector order under `expected`.
    /// The future returns the final revision or an [`AppendError`] for an empty batch,
    /// revision conflict, revision exhaustion, or adapter failure. The batch is not returned
    /// on error; partial-write and retry behavior must be documented by the adapter.
    ///
    /// **日本語:** `events` の所有権を受け取り、`expected` を条件にベクター順で追加します。
    /// Future は最終リビジョン、または空バッチ、リビジョン競合、リビジョン枯渇、
    /// アダプター障害を示す [`AppendError`] を返します。エラー時にバッチは返されません。
    /// 部分書き込みと再試行の挙動はアダプターが文書化する必要があります。
    fn append<'a>(
        &'a mut self,
        id: &'a <Self::Aggregate as Entity>::Id,
        expected: ExpectedRevision,
        events: Vec<<Self::Aggregate as AggregateRoot>::Event>,
    ) -> Pin<Box<dyn Future<Output = Result<AppendResult, AppendError<Self::Error>>> + Send + 'a>>;
}
