//! **English:** CQRS handlers and lifecycle coordination for aggregate commands and queries.
//!
//! **日本語:** 集約に対するコマンド・クエリの CQRS ハンドラーとライフサイクル調整機能を提供します。
//!
//! **English:** Lifecycle contexts require a successful save and event dispatch before completion.
//! A failed attempt is consumed and cannot be retried through the same context.
//!
//! **日本語:** ライフサイクルコンテキストは完了前に保存とイベント配信の成功を要求します。
//! 失敗した操作は消費済みとなり、同じコンテキストから再試行できません。

mod async_query_handler;
mod query_handler;

/// **English:** Re-exports asynchronous command and query handlers, aggregate creators,
/// lifecycle callbacks, and the context coordinating persistence and event dispatch.
///
/// **日本語:** 非同期のコマンド・クエリハンドラー、集約作成器、ライフサイクルコールバック、
/// 永続化とイベント配信を調整するコンテキストを再エクスポートします。
pub use async_query_handler::*;

/// **English:** Re-exports synchronous command and query handlers, aggregate creators,
/// lifecycle callbacks, and the context coordinating aggregate saves and event dispatch.
///
/// **日本語:** 同期のコマンド・クエリハンドラー、集約作成器、ライフサイクルコールバック、
/// 集約保存とイベント配信を調整するコンテキストを再エクスポートします。
pub use query_handler::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// **English:** Records whether a lifecycle operation is pending, successful, or consumed by failure.
///
/// **日本語:** ライフサイクル操作が未実行、成功、または失敗により消費済みかを表します。
pub enum OperationState {
    /// **English:** The operation has not been attempted.
    ///
    /// **日本語:** 操作はまだ試行されていません。
    Pending,
    /// **English:** The operation completed successfully.
    ///
    /// **日本語:** 操作は正常に完了しました。
    Succeeded,
    /// **English:** The operation failed or its future was dropped after starting; retry is disallowed.
    ///
    /// **日本語:** 操作が失敗したか開始後に future が破棄され、再試行できません。
    FailedConsumed,
}

#[derive(Debug, PartialEq, Eq)]
/// **English:** An error returned when a lifecycle save or dispatch cannot be attempted or its adapter fails.
///
/// **日本語:** ライフサイクルの保存・配信を実行できない場合、またはアダプターが失敗した場合のエラーです。
pub enum LifecycleOperationError<E> {
    /// **English:** The repository or publisher returned this adapter error.
    ///
    /// **日本語:** リポジトリまたはパブリッシャーが返したアダプターエラーです。
    Adapter(E),
    /// **English:** Save was already attempted on this context.
    ///
    /// **日本語:** このコンテキストでは保存がすでに試行されています。
    AlreadySaved,
    /// **English:** Dispatch was already attempted on this context.
    ///
    /// **日本語:** このコンテキストでは配信がすでに試行されています。
    AlreadyDispatched,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// **English:** Describes why a lifecycle context cannot be completed.
///
/// **日本語:** ライフサイクルコンテキストを完了できない理由を示します。
pub enum LifecycleCompletionError {
    /// **English:** No save was attempted before completion.
    ///
    /// **日本語:** 完了前に保存が試行されていません。
    MissingSave,
    /// **English:** No event dispatch was attempted before completion.
    ///
    /// **日本語:** 完了前にイベント配信が試行されていません。
    MissingDispatch,
    /// **English:** The save attempt failed and was consumed.
    ///
    /// **日本語:** 保存の試行が失敗し、消費済みです。
    SaveFailedConsumed,
    /// **English:** The dispatch attempt failed and was consumed.
    ///
    /// **日本語:** 配信の試行が失敗し、消費済みです。
    DispatchFailedConsumed,
}
