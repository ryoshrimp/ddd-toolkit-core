/// **English:** Domain operation with owned input and implementation-selected output and error types.
/// It supplies no transaction or retry logic. The shared receiver permits interior state;
/// neither purity nor concurrent use is required by the trait.
///
/// **日本語:** 所有する入力と、実装が選ぶ出力型・エラー型を持つドメイン操作です。
/// トランザクションや再試行処理は提供しません。共有参照でも内部状態を持てるため、
/// このトレイトは純粋性や並行利用を要求しません。
pub trait DomainService<Input> {
    /// **English:** Result value produced when the domain operation succeeds.
    ///
    /// **日本語:** ドメイン操作が成功した際の結果値です。
    type Output;

    /// **English:** Failure reported by the service, with no required error trait bounds.
    ///
    /// **日本語:** サービスが報告する失敗です。エラートレイトの境界は課しません。
    type Error;

    /// **English:** Consumes `input` to perform the operation, returning its output or error.
    /// Implementations define side effects on success and failure and document any panic conditions.
    ///
    /// **日本語:** `input` を消費して操作を実行し、出力またはエラーを返します。
    /// 成功・失敗時の副作用とパニックの条件は各実装が定め、文書化します。
    fn execute(&self, input: Input) -> Result<Self::Output, Self::Error>;
}

/// **English:** Fallible domain rule checked against a borrowed, possibly unsized context.
/// Rejection and inability to evaluate are both represented by the implementation's error type.
/// This trait does not enforce the rule automatically at other domain boundaries.
///
/// **日本語:** サイズ不定でもよいコンテキストを借用して確認する、失敗し得るドメイン規則です。
/// 拒否と評価不能は、どちらも実装のエラー型で表現します。
/// 他のドメイン境界で規則を自動的に強制するものではありません。
pub trait Policy<Context: ?Sized> {
    /// **English:** Policy-specific rejection or evaluation failure.
    ///
    /// **日本語:** ポリシー固有の拒否または評価の失敗です。
    type Error;

    /// **English:** Returns `Ok(())` when `context` passes the rule, otherwise a policy error.
    /// No ownership of the context is transferred; effects and panics depend on the implementation.
    ///
    /// **日本語:** `context` が規則を満たす場合は `Ok(())`、それ以外はポリシーエラーを返します。
    /// コンテキストの所有権は移らず、副作用とパニックは実装に依存します。
    fn check(&self, context: &Context) -> Result<(), Self::Error>;
}
