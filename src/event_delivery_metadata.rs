use crate::DomainEvent;

/// **English:** Owned delivery identifiers, independent of the domain event payload.
///
/// Strings are stored without format or emptiness validation. Providing an event identifier
/// does not enforce uniqueness or deduplication; correlation and causation are optional labels.
///
/// **日本語:** ドメインイベントのペイロードから独立した、所有された配信識別子です。
///
/// 文字列の形式や空文字列は検証しません。イベント識別子の指定は一意性や重複排除を
/// 強制しません。相関識別子と原因識別子は任意のラベルです。
///
/// **English:** Attach tracing labels without changing the event identifier.
///
/// **日本語:** イベント識別子を変えずにトレース用のラベルを付けます。
///
/// ```rust
/// use ddd_toolkit_core::DeliveryMetadata;
///
/// let metadata = DeliveryMetadata::new("event-1")
///     .with_correlation_id("request-7")
///     .with_causation_id("command-2");
/// assert_eq!(metadata.event_id(), "event-1");
/// assert_eq!(metadata.correlation_id(), Some("request-7"));
/// assert_eq!(metadata.causation_id(), Some("command-2"));
/// ```
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DeliveryMetadata {
    event_id: String,
    correlation_id: Option<String>,
    causation_id: Option<String>,
}

impl DeliveryMetadata {
    /// **English:** Stores `event_id` after conversion to `String`, leaving both optional IDs unset.
    /// No identifier validation or generation is performed.
    ///
    /// **日本語:** `event_id` を `String` に変換して保存し、二つの任意識別子は未設定にします。
    /// 識別子の検証や生成は行いません。
    pub fn new(event_id: impl Into<String>) -> Self {
        Self {
            event_id: event_id.into(),
            correlation_id: None,
            causation_id: None,
        }
    }

    /// **English:** Consumes this value and returns it with the correlation ID set to `id`.
    /// A previously set correlation ID is replaced.
    ///
    /// **日本語:** 自身を消費し、相関識別子を `id` に設定して返します。
    /// 設定済みの相関識別子は置き換えます。
    pub fn with_correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }

    /// **English:** Consumes this value and sets the causation ID to `id`, replacing any prior value.
    ///
    /// **日本語:** 自身を消費し、原因識別子を `id` に設定します。以前の値は置き換えます。
    pub fn with_causation_id(mut self, id: impl Into<String>) -> Self {
        self.causation_id = Some(id.into());
        self
    }

    /// **English:** Borrows the event identifier as stored, without allocating.
    ///
    /// **日本語:** 保存されたイベント識別子を、割り当てなしで借用します。
    pub fn event_id(&self) -> &str {
        &self.event_id
    }

    /// **English:** Borrows the correlation ID, or returns `None` if it has not been set.
    ///
    /// **日本語:** 相関識別子を借用し、未設定なら `None` を返します。
    pub fn correlation_id(&self) -> Option<&str> {
        self.correlation_id.as_deref()
    }

    /// **English:** Borrows the causation ID, or returns `None` when no cause label is stored.
    ///
    /// **日本語:** 原因識別子を借用し、原因のラベルが保存されていなければ `None` を返します。
    pub fn causation_id(&self) -> Option<&str> {
        self.causation_id.as_deref()
    }
}

/// **English:** Opt-in access to delivery labels on a [`DomainEvent`].
///
/// Implementing [`DomainEvent`] alone does not require these labels. This trait does not
/// assign identifiers or prescribe how metadata is stored or transported.
///
/// **日本語:** [`DomainEvent`] に配信ラベルへのアクセスを任意で追加します。
///
/// [`DomainEvent`] の実装だけではラベルは必須ではありません。このトレイトは
/// 識別子の割り当てやメタデータの保存・転送方法を規定しません。
pub trait DomainEventDeliveryMetadata: DomainEvent {
    /// **English:** Borrows this event's delivery metadata, or returns `None` if unavailable.
    /// The returned reference cannot outlive the event borrow.
    ///
    /// **日本語:** このイベントの配信メタデータを借用し、存在しなければ `None` を返します。
    /// 返される参照の有効期間はイベントの借用を超えません。
    fn delivery_metadata(&self) -> Option<&DeliveryMetadata>;
}
