#![allow(clippy::type_complexity)]

use crate::{AggregateRoot, AppendError, AppendResult, ExpectedRevision};

/// **English:** Owned adapter-assigned identifier for an outbox message.
///
/// The string is not validated or generated here. It need not equal a domain event's
/// delivery identifier; uniqueness and identifier scope belong to the outbox adapter.
///
/// **日本語:** アダプターが割り当てる、所有されたアウトボックスメッセージ識別子です。
///
/// ここでは文字列を検証・生成しません。ドメインイベントの配信識別子と同じである
/// 必要はなく、一意性と識別子のスコープはアウトボックスアダプターが定めます。
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct OutboxMessageId(String);

impl OutboxMessageId {
    /// **English:** Converts `value` to the stored string, accepting empty strings as well.
    ///
    /// **日本語:** `value` を保存用の文字列へ変換します。空文字列も受け付けます。
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// **English:** Borrows the message identifier without cloning its string.
    ///
    /// **日本語:** 文字列を複製せずにメッセージ識別子を借用します。
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// **English:** Consumes the identifier and returns its owned string.
    ///
    /// **日本語:** 識別子を消費し、所有された文字列を返します。
    pub fn into_inner(self) -> String {
        self.0
    }
}

/// **English:** Owned message identifier and event returned by an outbox claim.
///
/// Constructing or dropping this container does not reserve, acknowledge, or release a message.
/// Leases, redelivery, and acknowledgement effects are adapter responsibilities.
///
/// **日本語:** アウトボックスの取得操作が返す、所有されたメッセージ識別子とイベントです。
///
/// このコンテナーの構築や破棄は、メッセージの予約、確認応答、解放を行いません。
/// リース、再配信、確認応答の効果はアダプターの責務です。
///
/// **English:** Split a claim to transfer its identifier and payload independently.
///
/// **日本語:** 識別子とペイロードを別々に渡すために取得結果を分解します。
///
/// ```rust
/// use ddd_toolkit_core::{OutboxClaim, OutboxMessageId};
///
/// let claim = OutboxClaim::new(OutboxMessageId::new("message-1"), 42_u32);
/// assert_eq!(claim.message_id().as_str(), "message-1");
/// assert_eq!(*claim.event(), 42);
/// let (id, event) = claim.into_parts();
/// assert_eq!(id.into_inner(), "message-1");
/// assert_eq!(event, 42);
/// ```
#[derive(Debug, Eq, PartialEq)]
pub struct OutboxClaim<E> {
    message_id: OutboxMessageId,
    event: E,
}

impl<E> OutboxClaim<E> {
    /// **English:** Pairs `message_id` with `event` without contacting an outbox adapter.
    ///
    /// **日本語:** アウトボックスアダプターにアクセスせず、`message_id` と `event` を組にします。
    pub fn new(message_id: OutboxMessageId, event: E) -> Self {
        Self { message_id, event }
    }

    /// **English:** Borrows the identifier used to acknowledge this message.
    ///
    /// **日本語:** このメッセージの確認応答に使う識別子を借用します。
    pub fn message_id(&self) -> &OutboxMessageId {
        &self.message_id
    }

    /// **English:** Borrows the claimed payload without removing it from this container.
    ///
    /// **日本語:** このコンテナーから取り出さずに、取得したペイロードを借用します。
    pub fn event(&self) -> &E {
        &self.event
    }

    /// **English:** Consumes the claim and returns the identifier and event without cloning.
    /// This does not acknowledge the message.
    ///
    /// **日本語:** 取得結果を消費し、複製せずに識別子とイベントを返します。
    /// この操作はメッセージの確認応答を行いません。
    pub fn into_parts(self) -> (OutboxMessageId, E) {
        (self.message_id, self.event)
    }
}

/// **English:** Persistence boundary for saving an aggregate and staging events for delivery.
///
/// An adapter supplies the transaction joining aggregate persistence and outbox insertion;
/// this trait contains no transaction implementation. Adapters must describe atomicity,
/// durability, concurrent claims, claim order, retry/lease policy, and panic conditions.
/// Claiming and acknowledging are separate from saving; no exactly-once delivery mechanism
/// is supplied, so consumers may need to handle duplicate events.
///
/// **日本語:** 集約の保存と配信対象イベントの登録を行う永続化境界です。
///
/// 集約の永続化とアウトボックス挿入を結ぶトランザクションはアダプターが提供し、
/// このトレイト自体には実装しません。原子性、永続性、並行取得、取得順序、再試行・
/// リース方針、パニック条件はアダプターが記述する必要があります。取得と確認応答は
/// 保存から分離されています。厳密に一度だけ配信する機構は提供しないため、
/// コンシューマー側で重複イベントへの対応が必要になる場合があります。
pub trait TransactionalOutbox {
    /// **English:** Aggregate whose state and associated events are staged together.
    ///
    /// **日本語:** 状態と関連イベントをまとめて登録する対象の集約です。
    type Aggregate: AggregateRoot;
    /// **English:** Adapter failure used by save, claim, and acknowledgement operations.
    ///
    /// **日本語:** 保存、取得、確認応答が使うアダプター障害です。
    type Error;

    /// **English:** Borrows `aggregate` and consumes `events` to save and enqueue under `expected`.
    /// Returns the resulting stream revision, or [`AppendError`] for an empty batch, conflict,
    /// revision exhaustion, or adapter failure. Events are supplied explicitly; this signature
    /// does not pull or clear the aggregate's pending events. The batch is not returned on error;
    /// consult the adapter's transaction contract before retrying.
    ///
    /// **日本語:** `aggregate` を借用し、`events` を消費して `expected` を条件に保存・登録します。
    /// 結果のストリームリビジョン、または空バッチ、競合、リビジョン枯渇、アダプター障害を
    /// 示す [`AppendError`] を返します。イベントは明示的に渡します。このシグネチャは
    /// 集約の保留イベントを取り出したり消去したりしません。エラー時にバッチは返されないため、
    /// 再試行する前にアダプターのトランザクション契約を確認してください。
    fn save_and_enqueue(
        &mut self,
        aggregate: &Self::Aggregate,
        expected: ExpectedRevision,
        events: Vec<<Self::Aggregate as AggregateRoot>::Event>,
    ) -> Result<AppendResult, AppendError<Self::Error>>;

    /// **English:** Claims an available message, returning its owned identifier and event.
    /// `Ok(None)` means no message is available to this call; adapter failures use `Self::Error`.
    /// Reservation duration and eligibility for redelivery are not represented by [`OutboxClaim`].
    ///
    /// **日本語:** 利用可能なメッセージを取得し、所有された識別子とイベントを返します。
    /// `Ok(None)` はこの呼び出しで取得できるものがないことを表し、障害は `Self::Error` です。
    /// 予約期間と再配信の条件は [`OutboxClaim`] では表現しません。
    fn claim(
        &mut self,
    ) -> Result<Option<OutboxClaim<<Self::Aggregate as AggregateRoot>::Event>>, Self::Error>;

    /// **English:** Reports completed handling of `message_id`, returning an adapter error on failure.
    /// Unknown IDs, repeated acknowledgements, and removal or retention of acknowledged messages
    /// follow the adapter's policy. No claim token or ownership proof is required by this API.
    ///
    /// **日本語:** `message_id` の処理完了を通知し、失敗時にアダプターエラーを返します。
    /// 未知の識別子、確認応答の繰り返し、応答済みメッセージの削除・保持はアダプターの
    /// 方針に従います。この API は取得トークンや所有権の証明を要求しません。
    fn acknowledge(&mut self, message_id: &OutboxMessageId) -> Result<(), Self::Error>;
}
