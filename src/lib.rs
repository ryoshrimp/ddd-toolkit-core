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
//!
//! **English:** For event-sourced aggregates, [`EventStore`] loads complete [`StreamHistory`] values
//! and appends events using [`ExpectedRevision`]. [`AggregateReplay`] supplies reconstruction hooks
//! for [`replay_aggregate`] and [`replay_with_optional_snapshot`]. [`SnapshotStore`] persists
//! [`AggregateSnapshot`] values separately; adapters define storage and concurrency behavior.
//!
//! **日本語:** イベントソーシングを使う集約では、[`EventStore`] が完全な [`StreamHistory`] を
//! 読み込み、[`ExpectedRevision`] を条件にイベントを追加します。[`AggregateReplay`] は
//! [`replay_aggregate`] と [`replay_with_optional_snapshot`] の復元フックを定めます。
//! [`SnapshotStore`] は [`AggregateSnapshot`] を別途永続化し、保存や並行処理の挙動は
//! アダプターが定めます。
//!
//! **English:** [`Secret`] wraps a value implementing `zeroize::Zeroize`, redacts its formatted
//! output, and zeroizes it on drop. Access to the unredacted value is explicit; extracting it
//! transfers responsibility for zeroization to the caller. [`SecretMetadata`] is a zero-sized
//! marker and does not itself protect data.
//!
//! **日本語:** [`Secret`] は `zeroize::Zeroize` を実装する値を包み、書式出力を秘匿化し、
//! 破棄時にゼロ化します。秘匿化していない値には明示的にアクセスし、値を取り出した後の
//! ゼロ化は呼び出し側が管理します。[`SecretMetadata`] はサイズがゼロのマーカーであり、
//! それ自体はデータを保護しません。
//!
//! **English:** [`AsyncEventStore`] and [`AsyncSnapshotStore`] expose boxed `Send` futures for
//! event and snapshot persistence. [`EventConsumer`] and [`Projection`] separate event handling
//! from read-model updates, with asynchronous counterparts [`AsyncEventConsumer`] and
//! [`AsyncProjection`]. [`DomainEventDeliveryMetadata`] optionally exposes [`DeliveryMetadata`].
//! [`TransactionalOutbox`] separates save-and-enqueue, claim, and acknowledgement operations;
//! adapters supply transactions and delivery policies, not this crate.
//!
//! **日本語:** [`AsyncEventStore`] と [`AsyncSnapshotStore`] はイベントとスナップショットの
//! 永続化を、ボックス化された `Send` Future で公開します。[`EventConsumer`] と [`Projection`]
//! はイベント処理と読み取りモデル更新を分け、非同期版の [`AsyncEventConsumer`] と
//! [`AsyncProjection`] も提供します。[`DomainEventDeliveryMetadata`] は任意で
//! [`DeliveryMetadata`] を公開します。[`TransactionalOutbox`] は保存・登録、取得、確認応答を
//! 分離します。トランザクションと配信方針はこのクレートではなくアダプターが提供します。
//!
//! **English:** [`TransactionalOutboxLifecycleContext`] tracks a single save-and-enqueue attempt.
//! It pulls pending events without restoring them on failure, rejects repeated attempts, and
//! reports missing or unsuccessful enqueueing at completion. It does not implement a transaction.
//!
//! **日本語:** [`TransactionalOutboxLifecycleContext`] は保存・登録の一回の試行を追跡します。
//! 保留イベントを取り出して失敗時も復元せず、繰り返しの試行を拒否し、完了確認時に
//! 未試行や未成功を報告します。トランザクション自体は実装しません。

mod aggregate_root;
mod async_dispatch;
mod async_domain_event_publisher;
mod async_event_consumption;
mod async_event_store;
mod async_repository;
mod async_snapshot;
mod cqrs;
mod dispatch;
mod domain_event;
mod domain_event_conversion;
mod domain_event_metadata;
mod domain_event_publisher;
mod entity;
mod event_consumption;
mod event_delivery_metadata;
mod event_store;
mod outbox_lifecycle;
mod replay;
mod repository;
mod secret;
mod secret_metadata;
mod snapshot;
mod transactional_outbox;
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

/// **English:** Exposes asynchronous event handling and read-model update ports.
///
/// **日本語:** 非同期イベント処理と読み取りモデル更新のポートを公開します。
pub use async_event_consumption::{AsyncEventConsumer, AsyncProjection};

/// **English:** Exposes [`async_event_store::AsyncEventStore`] for future-based event persistence.
///
/// **日本語:** Future によるイベント永続化の [`async_event_store::AsyncEventStore`] を公開します。
pub use async_event_store::AsyncEventStore;

/// **English:** Re-exports [`async_repository::AsyncRepository`], the trait for asynchronous
/// persistence operations on aggregates.
///
/// **日本語:** 集約に対する非同期永続化操作のトレイト [`async_repository::AsyncRepository`] を
/// 再エクスポートします。
pub use async_repository::AsyncRepository;

/// **English:** Exposes [`async_snapshot::AsyncSnapshotStore`] for asynchronous snapshot storage.
///
/// **日本語:** 非同期スナップショット保存の [`async_snapshot::AsyncSnapshotStore`] を公開します。
pub use async_snapshot::AsyncSnapshotStore;

/// **English:** Re-exports synchronous and asynchronous CQRS handlers, aggregate creators, and lifecycle APIs.
///
/// **日本語:** 同期・非同期の CQRS ハンドラー、集約作成器、ライフサイクル API を再エクスポートします。
pub use cqrs::{
    AggregateCreator, AsyncAggregateCreator, AsyncCommandHandler, AsyncLifecycleCallback,
    AsyncLifecycleContext, AsyncQueryHandler, CommandHandler, LifecycleCallback,
    LifecycleCompletionError, LifecycleContext, LifecycleOperationError, QueryHandler,
};

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

/// **English:** Exposes synchronous event consumption and projection update contracts.
///
/// **日本語:** 同期イベント処理とプロジェクション更新の契約を公開します。
pub use event_consumption::{EventConsumer, Projection};

/// **English:** Exposes delivery identifier containers and opt-in metadata access for events.
///
/// **日本語:** 配信識別子のコンテナーとイベントの任意メタデータアクセスを公開します。
pub use event_delivery_metadata::{DeliveryMetadata, DomainEventDeliveryMetadata};

/// **English:** Re-exports event-stream storage contracts, revision checks, and validated histories.
///
/// **日本語:** イベントストリームの保存契約、リビジョン照合、検証済み履歴を再エクスポートします。
pub use event_store::{
    AppendError, AppendResult, CurrentRevision, EventStore, ExpectedRevision, LoadResult, NoStream,
    RevisionConflict, StreamHistory, StreamHistoryError, StreamRevision,
};

/// **English:** Exposes the single-attempt outbox lifecycle context, its errors, and their short aliases.
///
/// **日本語:** 一回のみ試行するアウトボックスのライフサイクルコンテキスト、エラー、短い別名を公開します。
pub use outbox_lifecycle::{
    OutboxLifecycleCompletionError, OutboxLifecycleContext, OutboxLifecycleOperationError,
    TransactionalOutboxCompletionError, TransactionalOutboxLifecycleContext,
    TransactionalOutboxOperationError,
};

/// **English:** Re-exports aggregate reconstruction hooks, replay helpers, results, and failure types.
///
/// **日本語:** 集約復元フック、再生ヘルパー、結果、失敗の型を再エクスポートします。
pub use replay::{
    AggregateReplay, RehydratedAggregate, ReplayError, SnapshotReplayError, replay_aggregate,
    replay_with_optional_snapshot,
};

/// **English:** Re-exports the [`repository::Repository`] trait for the crate's public API.
///
/// **日本語:** [`repository::Repository`] トレイトをクレートの公開 API として再エクスポートします。
pub use repository::Repository;

/// **English:** Exposes [`secret::Secret`] for redacted formatting and zeroization on drop.
///
/// **日本語:** 書式出力の秘匿化と破棄時のゼロ化を行う [`secret::Secret`] を公開します。
pub use secret::Secret;

/// **English:** Exposes [`secret_metadata::SecretMetadata`], a marker with no stored secret value.
///
/// **日本語:** 秘密の値を保持しないマーカー [`secret_metadata::SecretMetadata`] を公開します。
pub use secret_metadata::SecretMetadata;

/// **English:** Re-exports snapshot state containers, classification, storage contracts, and errors.
///
/// **日本語:** スナップショット状態コンテナー、分類、保存契約、エラーを再エクスポートします。
pub use snapshot::{
    AggregateSnapshot, SnapshotLoadError, SnapshotSaveError, SnapshotStore, SnapshotUse,
};

/// **English:** Exposes the outbox persistence port, owned claims, and acknowledgement identifiers.
///
/// **日本語:** アウトボックス永続化ポート、所有された取得結果、確認応答用の識別子を公開します。
pub use transactional_outbox::{OutboxClaim, OutboxMessageId, TransactionalOutbox};

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
