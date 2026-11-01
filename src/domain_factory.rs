/// **English:** Constructs a domain product from owned input, reporting a typed construction error.
/// Validation, identifier allocation, and side effects are chosen by the implementation;
/// this trait does not require the product to be an aggregate or provide persistence.
/// A shared receiver does not require `Send`, `Sync`, or side-effect-free construction.
///
/// **日本語:** 所有する入力からドメインの生成物を構築し、型付きの構築エラーを返します。
/// 検証、識別子の割り当て、副作用は実装が定めます。生成物を集約に制限せず、永続化も提供しません。
/// 共有参照のレシーバーは、`Send`、`Sync`、副作用のない構築を要求するものではありません。
pub trait DomainFactory<Input> {
    /// **English:** Value returned after successful construction; no domain trait bound is imposed.
    ///
    /// **日本語:** 構築成功時に返す値です。ドメイントレイトの境界は課しません。
    type Product;

    /// **English:** Implementation-defined reason that construction could not return a product.
    ///
    /// **日本語:** 生成物を返せなかった理由を表す実装定義のエラーです。
    type Error;

    /// **English:** Consumes `input` and returns a product or a construction error.
    /// The implementation specifies validation rules, failure side effects, and any panics.
    ///
    /// **日本語:** `input` を消費し、生成物または構築エラーを返します。
    /// 検証規則、失敗時の副作用、パニックの条件は実装が定めます。
    fn create(&self, input: Input) -> Result<Self::Product, Self::Error>;
}
