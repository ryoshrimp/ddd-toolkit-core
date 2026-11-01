#![allow(clippy::type_complexity)]

use crate::{
    AggregateRoot, AppendError, AppendResult, ExpectedRevision, OutboxClaim, OutboxMessageId,
};
use std::future::Future;
use std::pin::Pin;

/// **English:** Asynchronous boundary for saving aggregates and staging events for delivery.
///
/// Operations return boxed `Send` futures without choosing an executor. The adapter supplies
/// transactions, durability, concurrent claim coordination, ordering, leases, and panic behavior.
/// Neither errors nor cancellation imply rollback; implementations must document retry safety.
/// Claims and acknowledgements are separate operations, not an exactly-once delivery mechanism.
/// Implementations may perform work before returning a future as well as while polling it.
///
/// **日本語:** 集約の保存と配信イベントの登録を行う非同期境界です。
///
/// 操作は実行器を選ばず、ボックス化された `Send` Future を返します。トランザクション、
/// 永続性、並行取得の調整、順序、リース、パニック時の挙動はアダプターが提供します。
/// エラーやキャンセルはロールバックを意味せず、実装は再試行の安全性を文書化する必要があります。
/// 取得と確認応答は別操作であり、厳密に一度だけ配信する機構ではありません。
/// 実装はポーリング中だけでなく、Future を返す前にも処理を行う場合があります。
pub trait AsyncTransactionalOutbox {
    /// **English:** Aggregate borrowed during saving; `Sync` permits sharing its reference across threads.
    ///
    /// **日本語:** 保存時に借用する集約です。`Sync` により参照をスレッド間で共有できます。
    type Aggregate: AggregateRoot + Sync;
    /// **English:** Adapter failure transferable in the future's result.
    ///
    /// **日本語:** Future の結果としてスレッド間で受け渡せるアダプター障害です。
    type Error: Send;

    /// **English:** Borrows `aggregate` and takes ownership of `events` for saving and enqueueing
    /// under `expected`. The future returns the final revision or [`AppendError`] for an empty
    /// batch, conflict, revision exhaustion, or adapter failure. The batch is not returned on
    /// failure. This signature does not pull pending events from the aggregate; transaction scope
    /// and partial-write behavior are defined by the adapter.
    ///
    /// **日本語:** `aggregate` を借用し、`events` の所有権を受け取り、`expected` を条件に保存・登録します。
    /// Future は最終リビジョン、または空バッチ、競合、リビジョン枯渇、アダプター障害を示す
    /// [`AppendError`] を返します。失敗時にバッチは返されません。このシグネチャは集約から
    /// 保留イベントを取り出しません。トランザクション範囲と部分書き込みの挙動はアダプターが定めます。
    fn save_and_enqueue<'a>(
        &'a mut self,
        aggregate: &'a Self::Aggregate,
        expected: ExpectedRevision,
        events: Vec<<Self::Aggregate as AggregateRoot>::Event>,
    ) -> Pin<Box<dyn Future<Output = Result<AppendResult, AppendError<Self::Error>>> + Send + 'a>>;

    /// **English:** Returns a future claiming an available message as an owned [`OutboxClaim`].
    /// `Ok(None)` means none is available to this call; `Self::Error` reports adapter failure.
    /// Reservation duration and redelivery after cancellation follow the adapter's policy.
    ///
    /// **日本語:** 利用可能なメッセージを所有された [`OutboxClaim`] として取得する Future を返します。
    /// `Ok(None)` はこの呼び出しで取得可能なものがないこと、`Self::Error` は障害を表します。
    /// 予約期間とキャンセル後の再配信はアダプターの方針に従います。
    fn claim<'a>(
        &'a mut self,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        Option<OutboxClaim<<Self::Aggregate as AggregateRoot>::Event>>,
                        Self::Error,
                    >,
                > + Send
                + 'a,
        >,
    >;

    /// **English:** Borrows `message_id` and returns a future reporting completed handling.
    /// The result is `Ok(())` or an adapter error. Unknown IDs, repeated acknowledgements,
    /// retention, and cancellation effects are adapter-defined; no claim token is required here.
    ///
    /// **日本語:** `message_id` を借用し、処理完了を通知する Future を返します。
    /// 結果は `Ok(())` またはアダプターエラーです。未知の識別子、確認応答の繰り返し、
    /// 保持、キャンセルの効果はアダプターが定めます。ここでは取得トークンを要求しません。
    fn acknowledge<'a>(
        &'a mut self,
        message_id: &'a OutboxMessageId,
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'a>>;
}
