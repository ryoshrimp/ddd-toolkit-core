/// **English:** Identifies a domain object by an associated identifier type.
///
/// Implementors expose the identifier without transferring ownership. The trait does not require
/// identifiers to be comparable, serializable, or globally unique; those constraints belong to the
/// domain type or to the repository that uses it.
///
/// **日本語:** 関連付けられた識別子型によってドメインオブジェクトを識別します。
///
/// 実装型は所有権を移さずに識別子を公開します。識別子の比較可能性、シリアライズ可能性、
/// グローバルな一意性は要求しないため、必要な制約はドメイン型または利用するリポジトリで定義します。
pub trait Entity {
    /// **English:** The type used to identify this entity.
    ///
    /// **日本語:** このエンティティの識別に使う型です。
    type Id;

    /// **English:** Borrows this entity's identifier.
    ///
    /// The returned reference is tied to `self`, so the identifier remains owned by the entity and
    /// cannot outlive the borrow. This method does not mutate the entity and does not panic by itself.
    ///
    /// **日本語:** このエンティティの識別子を借用します。
    ///
    /// 返される参照の有効期間は `self` の借用に結び付くため、識別子の所有権はエンティティに残り、
    /// 借用より長く存続することはありません。このメソッド自体はエンティティを変更せず、通常はパニックしません。
    fn id(&self) -> &Self::Id;
}
