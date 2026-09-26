#![allow(clippy::type_complexity)]

use crate::{AggregateRoot, Entity};
use std::future::Future;
use std::pin::Pin;

/// **English:** Provides asynchronous persistence operations for one aggregate type. Implementors
/// supply storage behavior and its error type; returned futures borrow the repository and are
/// `Send`.
///
/// **日本語:** ひとつの集約型に対する非同期の永続化操作を提供します。実装側が保存処理とエラー型を
/// 定義します。返される future はリポジトリを借用し、`Send` です。
pub trait AsyncRepository {
    /// **English:** Aggregate type managed by this repository.
    ///
    /// **日本語:** このリポジトリが管理する集約型です。
    type Aggregate: AggregateRoot;
    /// **English:** Error returned by persistence operations.
    ///
    /// **日本語:** 永続化操作が返すエラー型です。
    type Error;

    /// **English:** Loads an aggregate by identifier, returning `Ok(None)` when no matching
    /// aggregate exists and an error when the lookup fails.
    ///
    /// **日本語:** 識別子で集約を読み込みます。一致する集約がなければ `Ok(None)`、検索に失敗すれば
    /// エラーを返します。
    fn find_by_id<'a>(
        &'a self,
        id: &'a <Self::Aggregate as Entity>::Id,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Self::Aggregate>, Self::Error>> + Send + 'a>>;

    /// **English:** Persists `aggregate`, returning an error if storage fails. Whether this inserts
    /// or updates is determined by the implementation.
    ///
    /// **日本語:** `aggregate` を永続化し、保存に失敗した場合はエラーを返します。新規登録か更新かは
    /// 実装によって決まります。
    fn save<'a>(
        &'a mut self,
        aggregate: &'a Self::Aggregate,
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'a>>;

    /// **English:** Removes the aggregate identified by `id`; returns an error if the operation
    /// fails. Behavior when the identifier is absent is implementation-defined.
    ///
    /// **日本語:** `id` で識別される集約を削除します。操作に失敗すればエラーを返し、識別子が存在しない
    /// 場合の動作は実装によって異なります。
    fn delete<'a>(
        &'a mut self,
        id: &'a <Self::Aggregate as Entity>::Id,
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'a>>;

    /// **English:** Checks whether an aggregate with `id` exists, returning its presence or an
    /// error if the check fails.
    ///
    /// **日本語:** `id` の集約が存在するかを調べ、その有無または確認に失敗した場合のエラーを返します。
    fn exists<'a>(
        &'a self,
        id: &'a <Self::Aggregate as Entity>::Id,
    ) -> Pin<Box<dyn Future<Output = Result<bool, Self::Error>> + Send + 'a>>;
}
