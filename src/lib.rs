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
mod domain_event;
mod entity;
mod value_object;

/// **English:** Re-exports the [`aggregate_root::AggregateRoot`] trait for the crate's public API.
///
/// **日本語:** [`aggregate_root::AggregateRoot`] トレイトをクレートの公開 API として再エクスポートします。
pub use aggregate_root::AggregateRoot;

/// **English:** Re-exports the [`domain_event::DomainEvent`] marker trait for the crate's public API.
///
/// **日本語:** [`domain_event::DomainEvent`] マーカートレイトをクレートの公開 API として再エクスポートします。
pub use domain_event::DomainEvent;

/// **English:** Re-exports the [`entity::Entity`] trait for the crate's public API.
///
/// **日本語:** [`entity::Entity`] トレイトをクレートの公開 API として再エクスポートします。
pub use entity::Entity;

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
