use crate::{AggregateRoot, Entity};

/// **English:** Defines persistence operations for one [`AggregateRoot`] type.
///
/// Implementations choose the storage backend and error type. The trait does not prescribe
/// transaction boundaries, consistency guarantees, identifier ownership, or whether operations
/// are synchronous. A repository may return [`None`] from [`Repository::find_by_id`] when no
/// aggregate matches an identifier; storage failures are reported through [`Repository::Error`].
///
/// **日本語:** 1 つの [`AggregateRoot`] 型に対する永続化操作を定義します。
///
/// ストレージバックエンドとエラー型は実装が選択します。トランザクション境界、整合性保証、
/// 識別子の所有権、操作が同期的かどうかはこのトレイトでは規定しません。識別子に一致する
/// 集約がない場合、リポジトリは [`Repository::find_by_id`] から [`None`] を返せます。ストレージ
/// の失敗は [`Repository::Error`] を通じて報告します。
pub trait Repository {
    /// **English:** The aggregate type managed by this repository.
    ///
    /// **日本語:** このリポジトリが管理する集約の型です。
    type Aggregate: AggregateRoot;

    /// **English:** The error type returned when a repository operation fails.
    ///
    /// **日本語:** リポジトリ操作が失敗したときに返すエラー型です。
    type Error;

    /// **English:** Loads the aggregate identified by `id`.
    ///
    /// Returns `Ok(None)` when no aggregate matches `id`, or `Err` when the implementation cannot
    /// complete the lookup. The returned aggregate is owned by the caller.
    ///
    /// **日本語:** `id` で識別される集約を読み込みます。
    ///
    /// `id` に一致する集約がない場合は `Ok(None)`、実装が検索を完了できない場合は `Err` を
    /// 返します。返された集約の所有権は呼び出し側に移ります。
    fn find_by_id(
        &self,
        id: &<Self::Aggregate as Entity>::Id,
    ) -> Result<Option<Self::Aggregate>, Self::Error>;

    /// **English:** Persists the current state of `aggregate`.
    ///
    /// The implementation defines whether this inserts, updates, or otherwise stores the aggregate.
    /// It returns `Err` if persistence fails.
    ///
    /// **日本語:** `aggregate` の現在の状態を永続化します。
    ///
    /// 挿入、更新、その他の保存のいずれになるかは実装が定義します。永続化に失敗した場合は
    /// `Err` を返します。
    fn save(&mut self, aggregate: &Self::Aggregate) -> Result<(), Self::Error>;

    /// **English:** Removes the aggregate identified by `id`.
    ///
    /// The implementation defines the behavior when no matching aggregate exists. It returns `Err`
    /// if deletion cannot be completed.
    ///
    /// **日本語:** `id` で識別される集約を削除します。
    ///
    /// 一致する集約が存在しない場合の動作は実装が定義します。削除を完了できない場合は `Err`
    /// を返します。
    fn delete(&mut self, id: &<Self::Aggregate as Entity>::Id) -> Result<(), Self::Error>;

    /// **English:** Checks whether an aggregate identified by `id` exists.
    ///
    /// Returns `Ok(true)` when the aggregate exists and `Ok(false)` otherwise. It returns `Err` if
    /// the implementation cannot determine the result.
    ///
    /// **日本語:** `id` で識別される集約が存在するか確認します。
    ///
    /// 集約が存在する場合は `Ok(true)`、それ以外は `Ok(false)` を返します。結果を判定できない
    /// 場合は `Err` を返します。
    fn exists(&self, id: &<Self::Aggregate as Entity>::Id) -> Result<bool, Self::Error>;
}
