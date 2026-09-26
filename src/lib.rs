//! **English:** Core traits for modeling Domain-Driven Design entities and value objects.
//!
//! **日本語:** ドメイン駆動設計のエンティティと値オブジェクトを表現するためのコアトレイトを提供します。
//!
//! **English:** [`Entity`] exposes a stable identifier by reference, while [`ValueObject`] exposes
//! a wrapped value by reference or by value. Implement these traits on domain types to keep their
//! domain-facing APIs independent from persistence details.
//!
//! **日本語:** [`Entity`] は安定した識別子を参照で公開し、[`ValueObject`] は内包値を参照または
//! 値で公開します。永続化の詳細から独立したドメイン型の API を保つために、これらのトレイトを実装します。

mod aggregate_root;
mod async_dispatch;
mod async_domain_event_publisher;
mod async_repository;
mod dispatch;
mod domain_event;
mod domain_event_conversion;
mod domain_event_metadata;
mod domain_event_publisher;
mod entity;
mod repository;
mod value_object;

/// **English:** Re-exports the [`aggregate_root::AggregateRoot`] trait for the crate's public API.
///
/// **日本語:** [`aggregate_root::AggregateRoot`] トレイトをクレートの公開 API として再エクスポートします。
pub use aggregate_root::AggregateRoot;

/// **English:** Re-exports [`async_dispatch::dispatch_events_async`] to publish aggregate events
/// asynchronously with restoration of pulled events if publishing fails.
///
/// **日本語:** 発行に失敗した場合に取り出したイベントを復元しながら、集約イベントを非同期発行する
/// [`async_dispatch::dispatch_events_async`] を再エクスポートします。
pub use async_dispatch::dispatch_events_async;

/// **English:** Re-exports [`async_domain_event_publisher::AsyncDomainEventPublisher`], the trait
/// implemented by asynchronous domain-event publishers.
///
/// **日本語:** 非同期ドメインイベントパブリッシャーが実装するトレイト
/// [`async_domain_event_publisher::AsyncDomainEventPublisher`] を再エクスポートします。
pub use async_domain_event_publisher::AsyncDomainEventPublisher;

/// **English:** Re-exports [`async_repository::AsyncRepository`], the trait for asynchronous
/// persistence operations on aggregates.
///
/// **日本語:** 集約に対する非同期永続化操作のトレイト [`async_repository::AsyncRepository`] を
/// 再エクスポートします。
pub use async_repository::AsyncRepository;

/// **English:** Publishes all currently pending events from an aggregate.
///
/// Events are published in the order returned by [`AggregateRoot::pull_events`]. If publishing any
/// event fails, the events returned from the aggregate are recorded back into it before the error
/// is returned. Events already accepted by the publisher are not rolled back, so retrying may
/// publish them again. On success, the pulled events are discarded from the aggregate.
///
/// **日本語:** 集約に現在保留されているすべてのイベントを発行します。
///
/// イベントは [`AggregateRoot::pull_events`] が返した順序で発行されます。いずれかのイベントの
/// 発行に失敗すると、集約から取り出したイベントを集約へ記録し直してからエラーを返します。
/// パブリッシャーがすでに受理したイベントはロールバックされないため、再試行時に重複して
/// 発行される可能性があります。成功した場合、取り出したイベントは集約から破棄されます。
///
/// [`dispatch_events`] は [`DomainEventPublisher::publish`] のエラーをそのまま返します。
/// [`AggregateRoot::pull_events`] または [`AggregateRoot::record_event`] のパニック条件は、
/// それぞれの実装に従います。
pub use dispatch::dispatch_events;

/// **English:** Re-exports the [`domain_event::DomainEvent`] marker trait for the crate's public API.
///
/// **日本語:** [`domain_event::DomainEvent`] マーカートレイトをクレートの公開 API として再エクスポートします。
pub use domain_event::DomainEvent;

/// **English:** Re-exports [`domain_event_conversion::DomainEventConversion`] for converting a domain event into another representation.
///
/// **日本語:** ドメインイベントを別の表現へ変換する [`domain_event_conversion::DomainEventConversion`] トレイトを再エクスポートします。
pub use domain_event_conversion::DomainEventConversion;

/// **English:** Re-exports [`domain_event_metadata::DomainEventMetadata`] for describing an event name, version, and optional metadata.
///
/// **日本語:** イベント名、バージョン、任意のメタデータを記述する [`domain_event_metadata::DomainEventMetadata`] トレイトを再エクスポートします。
pub use domain_event_metadata::DomainEventMetadata;

/// **English:** Re-exports the [`domain_event_publisher::DomainEventPublisher`] trait for publishing domain events.
///
/// **日本語:** ドメインイベントを発行する [`domain_event_publisher::DomainEventPublisher`] トレイトを再エクスポートします。
pub use domain_event_publisher::DomainEventPublisher;

/// **English:** Re-exports the [`entity::Entity`] trait for the crate's public API.
///
/// **日本語:** [`entity::Entity`] トレイトをクレートの公開 API として再エクスポートします。
pub use entity::Entity;

/// **English:** Re-exports the [`repository::Repository`] trait for the crate's public API.
///
/// **日本語:** [`repository::Repository`] トレイトをクレートの公開 API として再エクスポートします。
pub use repository::Repository;

/// **English:** Re-exports the [`value_object::ValueObject`] trait for the crate's public API.
///
/// **日本語:** [`value_object::ValueObject`] トレイトをクレートの公開 API として再エクスポートします。
pub use value_object::ValueObject;

#[cfg(feature = "serde")]
#[doc(hidden)]
/// **English:** Internal access to the optional `serde` dependency for generated implementations.
///
/// **日本語:** 生成された実装から任意依存の `serde` にアクセスするための内部モジュールです。
pub mod __private {
    /// **English:** Re-exports the optional `serde` crate for macro-generated code.
    ///
    /// **日本語:** マクロが生成したコードで使うために、任意依存の `serde` クレートを再エクスポートします。
    pub use serde;
}
