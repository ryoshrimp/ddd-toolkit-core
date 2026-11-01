/// **English:** Represents a domain value by wrapping an inner value.
///
/// Implementors provide both a borrowed view through [`ValueObject::as_inner`] and an ownership-
/// consuming conversion through [`ValueObject::into_inner`]. Equality, hashing, validation, and
/// serialization policies remain specific to the implementing type.
///
/// **日本語:** 内包値をラップしてドメイン上の値を表現します。
///
/// 実装型は [`ValueObject::as_inner`] による借用ビューと、[`ValueObject::into_inner`] による
/// 所有権を消費する変換を提供します。等価性、ハッシュ、検証、シリアライズの方針は実装型ごとに定義します。
pub trait ValueObject {
    /// **English:** The value type wrapped by this value object.
    ///
    /// **日本語:** この値オブジェクトが内包する値の型です。
    type Inner;

    /// **English:** Borrows the wrapped value without consuming the value object.
    ///
    /// The returned reference is tied to `self` and reflects the implementation's current inner
    /// value. This method does not mutate the value object and does not panic by itself.
    ///
    /// **日本語:** 値オブジェクトを消費せずに内包値を借用します。
    ///
    /// 返される参照の有効期間は `self` の借用に結び付き、実装型が現在保持する内包値を反映します。
    /// このメソッド自体は値オブジェクトを変更せず、通常はパニックしません。
    fn as_inner(&self) -> &Self::Inner;

    /// **English:** Consumes the value object and returns its wrapped value.
    ///
    /// After this call, the value object cannot be used again. Implementations may perform
    /// validation or other work before returning, so callers should follow the implementation's
    /// documented behavior.
    ///
    /// **日本語:** 値オブジェクトを消費して内包値を返します。
    ///
    /// 呼び出し後は値オブジェクトを再利用できません。実装は返却前に検証などの処理を行う場合があるため、
    /// 呼び出し側は実装型が定める動作に従ってください。
    fn into_inner(self) -> Self::Inner;
}
