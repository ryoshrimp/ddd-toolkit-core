/// **English:** Fallible predicate over a borrowed candidate, with consuming boolean combinators.
/// `T` may be unsized, but specification values must be sized. Combinations evaluate left to right,
/// short-circuit on a decisive boolean, and propagate the first error without evaluating later rules.
/// Implementations may use interior state; purity and thread safety are not required.
///
/// **日本語:** 借用した候補に対する失敗し得る述語と、自身を消費する論理結合操作です。
/// `T` はサイズ不定でもよいですが、仕様の値自体はサイズ確定が必要です。結合は左から評価し、
/// 真偽が確定すると短絡し、エラーでは後続の規則を評価せず最初のエラーを伝播します。
/// 実装は内部状態を利用でき、純粋性やスレッド安全性は要求しません。
///
/// **English:** Combine rules over an unsized string candidate and invert the result.
///
/// **日本語:** サイズ不定の文字列候補に対する規則を結合し、結果を反転します。
///
/// ```rust
/// use ddd_toolkit_core::Specification;
///
/// struct NonEmpty;
/// impl Specification<str> for NonEmpty {
///     type Error = std::convert::Infallible;
///     fn is_satisfied_by(&self, candidate: &str) -> Result<bool, Self::Error> {
///         Ok(!candidate.is_empty())
///     }
/// }
/// let rule = NonEmpty.and(NonEmpty).not();
/// assert_eq!(rule.is_satisfied_by("order"), Ok(false));
/// assert_eq!(rule.is_satisfied_by(""), Ok(true));
/// ```
pub trait Specification<T: ?Sized>: Sized {
    /// **English:** Evaluation failure, distinct from a successfully evaluated `false` result.
    ///
    /// **日本語:** 正常に評価された `false` とは区別される、評価の失敗です。
    type Error;

    /// **English:** Returns whether `candidate` satisfies the rule or why evaluation failed.
    /// The candidate remains borrowed; side effects and panic conditions depend on the rule.
    ///
    /// **日本語:** `candidate` が規則を満たすか、評価が失敗した理由を返します。
    /// 候補は借用のままで、副作用とパニックの条件は規則に依存します。
    fn is_satisfied_by(&self, candidate: &T) -> Result<bool, Self::Error>;

    /// **English:** Consumes both rules to form [`And`], evaluating `other` only if this rule returns `Ok(true)`.
    /// Both must share an error type. Construction evaluates neither rule.
    ///
    /// **日本語:** 両規則を消費して [`And`] を作り、この規則が `Ok(true)` の場合のみ `other` を評価します。
    /// エラー型は共通でなければなりません。構築時にはどちらも評価しません。
    fn and<S>(self, other: S) -> And<Self, S>
    where
        S: Specification<T, Error = Self::Error>,
    {
        And {
            left: self,
            right: other,
        }
    }

    /// **English:** Consumes both rules to form [`Or`], evaluating `other` only if this rule returns `Ok(false)`.
    /// An error from this rule is returned without consulting `other`.
    ///
    /// **日本語:** 両規則を消費して [`Or`] を作り、この規則が `Ok(false)` の場合のみ `other` を評価します。
    /// この規則のエラーは `other` を評価せずに返します。
    fn or<S>(self, other: S) -> Or<Self, S>
    where
        S: Specification<T, Error = Self::Error>,
    {
        Or {
            left: self,
            right: other,
        }
    }

    /// **English:** Consumes this rule to form [`Not`], which flips successful booleans and preserves errors.
    ///
    /// **日本語:** この規則を消費して [`Not`] を作り、成功時の真偽を反転し、エラーは保持します。
    fn not(self) -> Not<Self> {
        Not { inner: self }
    }
}

/// **English:** Conjunction that evaluates the right rule only after the left returns `Ok(true)`.
/// A left `Ok(false)` or error is returned immediately; a right result is forwarded unchanged.
///
/// **日本語:** 左が `Ok(true)` を返した場合のみ右を評価する論理積です。
/// 左の `Ok(false)` やエラーは直ちに返し、右を評価した場合はその結果を変更せず返します。
pub struct And<L, R> {
    left: L,
    right: R,
}

impl<L, R> And<L, R> {
    /// **English:** Stores `left` and `right` without evaluation; evaluation requires compatible specification impls.
    ///
    /// **日本語:** 評価せずに `left` と `right` を保持します。評価には互換な仕様の実装が必要です。
    pub fn new(left: L, right: R) -> Self {
        Self { left, right }
    }
}

impl<T: ?Sized, L, R> Specification<T> for And<L, R>
where
    L: Specification<T>,
    R: Specification<T, Error = L::Error>,
{
    type Error = L::Error;

    fn is_satisfied_by(&self, candidate: &T) -> Result<bool, Self::Error> {
        if !self.left.is_satisfied_by(candidate)? {
            return Ok(false);
        }
        self.right.is_satisfied_by(candidate)
    }
}

/// **English:** Disjunction that evaluates the right rule only after the left returns `Ok(false)`.
/// A left `Ok(true)` or error short-circuits; errors are not treated as a negative match.
///
/// **日本語:** 左が `Ok(false)` を返した場合のみ右を評価する論理和です。
/// 左の `Ok(true)` やエラーで短絡し、エラーを不一致として扱いません。
pub struct Or<L, R> {
    left: L,
    right: R,
}

impl<L, R> Or<L, R> {
    /// **English:** Owns the two operands in left-to-right evaluation order, without calling either rule.
    ///
    /// **日本語:** どちらも呼び出さず、左から右の評価順に二つの被演算子を所有します。
    pub fn new(left: L, right: R) -> Self {
        Self { left, right }
    }
}

impl<T: ?Sized, L, R> Specification<T> for Or<L, R>
where
    L: Specification<T>,
    R: Specification<T, Error = L::Error>,
{
    type Error = L::Error;

    fn is_satisfied_by(&self, candidate: &T) -> Result<bool, Self::Error> {
        if self.left.is_satisfied_by(candidate)? {
            return Ok(true);
        }
        self.right.is_satisfied_by(candidate)
    }
}

/// **English:** Negation that evaluates its inner rule once, flips `Ok(bool)`, and propagates `Err` unchanged.
///
/// **日本語:** 内側の規則を一度評価し、`Ok(bool)` を反転し、`Err` は変更せず伝播する否定です。
pub struct Not<S> {
    inner: S,
}

impl<S> Not<S> {
    /// **English:** Wraps `inner` by value without evaluating or validating it.
    ///
    /// **日本語:** 評価や検証をせず、`inner` を値として包みます。
    pub fn new(inner: S) -> Self {
        Self { inner }
    }
}

impl<T: ?Sized, S> Specification<T> for Not<S>
where
    S: Specification<T>,
{
    type Error = S::Error;

    fn is_satisfied_by(&self, candidate: &T) -> Result<bool, Self::Error> {
        self.inner.is_satisfied_by(candidate).map(|value| !value)
    }
}
