use crate::DomainEvent;

/// **English:** Publishes events that implement [`DomainEvent`].
///
/// A publisher may keep mutable state, such as a connection or an in-memory queue. Each call
/// receives a shared reference to one event and returns the publisher-defined error type when
/// publishing fails. Implementations decide whether events are delivered synchronously and
/// whether a successful return means acceptance, delivery, or queuing.
///
/// **日本語:** [`DomainEvent`] を実装するイベントを発行します。
///
/// パブリッシャーは、接続やメモリ内キューなどの可変状態を保持できます。各呼び出しは
/// 1 つのイベントへの共有参照を受け取り、発行に失敗した場合はパブリッシャー固有の
/// エラー型を返します。イベントを同期的に配送するか、成功が受理・配送・キュー投入の
/// どれを意味するかは実装が決定します。
pub trait DomainEventPublisher {
    /// **English:** The event type accepted by [`Self::publish`].
    ///
    /// **日本語:** [`Self::publish`] が受け付けるイベント型です。
    type Event: DomainEvent;

    /// **English:** The error returned when [`Self::publish`] cannot publish an event.
    ///
    /// **日本語:** [`Self::publish`] がイベントを発行できない場合に返すエラー型です。
    type Error;

    /// **English:** Attempts to publish `event` using this publisher.
    ///
    /// The event is borrowed for the duration of the call, so the implementation must not retain
    /// the reference after returning. A successful result has publisher-specific delivery
    /// semantics; an error indicates that the implementation could not complete its publish
    /// operation.
    ///
    /// **日本語:** このパブリッシャーを使って `event` の発行を試みます。
    ///
    /// イベントは呼び出し中だけ借用されるため、実装は戻り値を返した後にその参照を保持
    /// してはいけません。成功時の配送の意味はパブリッシャーごとに異なり、エラーは実装が
    /// 発行処理を完了できなかったことを示します。
    fn publish(&mut self, event: &Self::Event) -> Result<(), Self::Error>;
}
