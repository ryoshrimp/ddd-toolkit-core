use crate::{DomainEvent, Entity};

/// **English:** Represents an [`Entity`] that records domain events produced by state changes.
///
/// Implementations own the event queue. [`AggregateRoot::record_event`] adds an event, while
/// [`AggregateRoot::pull_events`] returns the events selected by the implementation and may clear
/// them from the queue. The trait does not prescribe ordering, deduplication, or persistence.
///
/// **日本語:** 状態変更によって発生したドメインイベントを記録する [`Entity`] を表します。
///
/// 実装型がイベントキューを所有します。[`AggregateRoot::record_event`] はイベントを追加し、
/// [`AggregateRoot::pull_events`] は実装が選んだイベントを返し、キューから削除する場合があります。
/// 順序、重複排除、永続化の方法はこのトレイトでは規定しません。
pub trait AggregateRoot: Entity {
    /// **English:** The domain-event type emitted by this aggregate.
    ///
    /// **日本語:** この集約が発行するドメインイベントの型です。
    type Event: DomainEvent;

    /// **English:** Records `event` for later retrieval by [`AggregateRoot::pull_events`].
    ///
    /// The implementation decides whether recording changes aggregate state, preserves insertion
    /// order, or rejects a value. The method may panic only if the implementation documents such
    /// a condition.
    ///
    /// **日本語:** 後で [`AggregateRoot::pull_events`] から取得できるよう `event` を記録します。
    ///
    /// 集約の状態を変更するか、挿入順を保つか、値を拒否するかは実装が決めます。実装が条件を
    /// 文書化している場合を除き、このメソッドがパニックすることは想定されません。
    fn record_event(&mut self, event: Self::Event);

    /// **English:** Takes the currently recorded events from this aggregate.
    ///
    /// Implementations define whether the returned vector is empty when no events are pending and
    /// whether taking events clears the internal queue. The returned events remain owned by the caller.
    ///
    /// **日本語:** この集約に現在記録されているイベントを取り出します。
    ///
    /// 保留イベントがない場合に空のベクターを返すか、取り出し時に内部キューを空にするかは
    /// 実装が定義します。返されたイベントの所有権は呼び出し側に移ります。
    fn pull_events(&mut self) -> Vec<Self::Event>;
}
