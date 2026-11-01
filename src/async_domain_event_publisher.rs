use crate::DomainEvent;
use std::future::Future;
use std::pin::Pin;

/// **English:** Publishes domain events asynchronously. Implementations define the event type and
/// error type; `publish` borrows both the publisher and event until its returned future completes.
///
/// **日本語:** ドメインイベントを非同期に発行します。実装側がイベント型とエラー型を定義します。
/// `publish` が返す future の完了まで、パブリッシャーとイベントの借用が保持されます。
pub trait AsyncDomainEventPublisher {
    /// **English:** Event type accepted by this publisher.
    ///
    /// **日本語:** このパブリッシャーが受け付けるイベント型です。
    type Event: DomainEvent;
    /// **English:** Error returned when publishing fails.
    ///
    /// **日本語:** 発行に失敗したときに返されるエラー型です。
    type Error;

    /// **English:** Publishes `event` and resolves to `Ok(())` on success or the implementation's
    /// error on failure. The returned future is `Send` and borrows `self` and `event` for `'a`.
    ///
    /// **日本語:** `event` を発行し、成功時は `Ok(())`、失敗時は実装が定めるエラーで解決します。
    /// 返される future は `Send` であり、`'a` の間 `self` と `event` を借用します。
    fn publish<'a>(
        &'a mut self,
        event: &'a Self::Event,
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'a>>;
}
