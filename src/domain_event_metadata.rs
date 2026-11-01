//! **English:** Defines stable identification and optional metadata for a [`crate::DomainEvent`].
//!
//! Implement [`DomainEventMetadata`] when consumers need an event name, a schema or contract
//! version, and optional event-specific metadata without taking ownership of the event.
//!
//! **日本語:** [`crate::DomainEvent`] の安定した識別情報と任意のメタデータを定義します。
//!
//! イベントの所有権を取り上げずに、イベント名、スキーマまたは契約のバージョン、
//! イベント固有の任意メタデータを利用者へ提供する場合に [`DomainEventMetadata`] を実装します。

use crate::DomainEvent;

/// **English:** Provides an event name, version, and optional metadata for a domain event.
///
/// The associated metadata may be unsized, such as a dynamically sized string or slice. The
/// default event version is `1`; implementations may override it to identify a later contract.
/// Implementations choose whether metadata is present for a given event and how the returned name maps to
/// external event contracts.
///
/// **日本語:** ドメインイベントのイベント名、バージョン、任意のメタデータを提供します。
///
/// 関連付けられたメタデータは、動的サイズの文字列やスライスなど、サイズ不定でも構いません。
/// イベントバージョンの既定値は `1` であり、実装側で上位の契約を示す値に変更できます。
/// 個々のイベントにメタデータを用意するかどうかや、返す名前を外部イベント契約へどう対応付けるかは実装が決めます。
pub trait DomainEventMetadata: DomainEvent {
    /// **English:** The metadata type associated with this event.
    ///
    /// The `?Sized` bound permits borrowed unsized metadata such as `str` or `[u8]`.
    ///
    /// **日本語:** このイベントに関連付けられたメタデータの型です。
    ///
    /// `?Sized` 境界により、`str` や `[u8]` のような借用されたサイズ不定のメタデータを指定できます。
    type Metadata: ?Sized;

    /// **English:** Returns the stable name used to identify this event type.
    ///
    /// The returned string has a `'static` lifetime and is not tied to an event instance.
    ///
    /// **日本語:** このイベント型を識別するための安定した名前を返します。
    ///
    /// 返される文字列のライフタイムは `'static` であり、イベントのインスタンスには依存しません。
    fn event_name() -> &'static str;

    /// **English:** Returns the contract version for this event type.
    ///
    /// The default implementation returns `1`. This method does not validate compatibility with
    /// consumers or perform migration.
    ///
    /// **日本語:** このイベント型の契約バージョンを返します。
    ///
    /// 既定の実装は `1` を返します。このメソッドは利用者との互換性検証や移行を行いません。
    fn event_version() -> u32 {
        1
    }

    /// **English:** Returns optional metadata borrowed from this event.
    ///
    /// `Some` contains a reference valid for the lifetime of the borrow; `None` indicates that
    /// this event has no metadata. The method does not transfer ownership.
    ///
    /// **日本語:** このイベントから借用した任意のメタデータを返します。
    ///
    /// `Some` には借用期間中有効な参照が入り、`None` はメタデータがないことを示します。
    /// このメソッドは所有権を移転しません。
    fn metadata(&self) -> Option<&Self::Metadata>;
}
