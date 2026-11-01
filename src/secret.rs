use core::fmt;
use core::mem::ManuallyDrop;
use zeroize::Zeroize;

/// **English:** Owns a value whose [`Zeroize::zeroize`] method is called before it is dropped.
///
/// `Debug` and `Display` write the literal `[REDACTED]` rather than formatting `T`. Access is explicit through
/// [`Self::as_inner`], [`Self::expose_secret`], or [`Self::into_inner`]; this type implements
/// neither `Deref` nor serde serialization or deserialization, even with the `serde` feature.
/// Cloning copies the inner value into another independently owned `Secret`. Equality delegates
/// to the inner type and does not promise constant-time comparison.
///
/// Zeroization follows `T`'s implementation and requires this wrapper's destructor to run;
/// it does not erase copies made elsewhere. [`Self::into_inner`] bypasses that destructor.
/// If `T::zeroize` panics, the inner value's destructor is not called by this wrapper.
/// `Send` and `Sync` depend on `T`; no synchronization is added.
///
/// **日本語:** 所有する値を破棄する前に、その値の [`Zeroize::zeroize`] を呼び出すラッパーです。
///
/// `Debug` と `Display` は `T` を書式出力せず、リテラル `[REDACTED]` を出力します。値へのアクセスには
/// [`Self::as_inner`]、[`Self::expose_secret`]、[`Self::into_inner`] を明示的に使います。
/// `Deref` は実装せず、`serde` feature が有効でも serde のシリアライズ・デシリアライズは
/// 実装しません。クローンは内包値を複製して別の `Secret` に所有させます。等値比較は内包型に
/// 委譲し、定数時間での比較を保証しません。
///
/// ゼロ化の範囲は `T` の実装に従い、このラッパーのデストラクターが実行される必要があります。
/// 別の場所に作られたコピーは消去しません。[`Self::into_inner`] はデストラクターを迂回します。
/// `T::zeroize` がパニックした場合、このラッパーは内包値のデストラクターを呼び出しません。
/// `Send` と `Sync` は `T` に依存し、同期機構は追加しません。
///
/// **English:** Formatting hides the value; the closure below explicitly reads its length.
///
/// **日本語:** 書式出力は値を隠します。次のクロージャーでは長さを明示的に読み取ります。
///
/// ```rust
/// use ddd_toolkit_core::Secret;
///
/// let secret = Secret::new(String::from("token"));
/// assert_eq!(format!("{secret:?}"), "[REDACTED]");
/// assert_eq!(format!("{secret}"), "[REDACTED]");
/// assert_eq!(secret.expose_secret(|value| value.len()), 5);
/// ```
pub struct Secret<T: Zeroize> {
    value: ManuallyDrop<T>,
}

impl<T: Zeroize> Secret<T> {
    /// **English:** Takes ownership of `value` without copying or zeroizing it at construction.
    ///
    /// **日本語:** 構築時にコピーやゼロ化を行わず、`value` の所有権を受け取ります。
    pub fn new(value: T) -> Self {
        Self {
            value: ManuallyDrop::new(value),
        }
    }

    /// **English:** Borrows the unredacted value for no longer than the borrow of `self`.
    ///
    /// Callers can read or copy the value; those copies are not zeroized by this wrapper.
    ///
    /// **日本語:** `self` の借用期間を超えない参照として、秘匿化していない値を借用します。
    ///
    /// 呼び出し側は値を読み取ったりコピーしたりできますが、そのコピーはこのラッパーではゼロ化しません。
    pub fn as_inner(&self) -> &T {
        // ManuallyDrop<T> is repr(transparent) over T.
        unsafe { &*(&self.value as *const ManuallyDrop<T> as *const T) }
    }

    /// **English:** Consumes the wrapper and returns its value without zeroizing it.
    ///
    /// The wrapper's destructor is not run. The caller becomes responsible for the returned
    /// value's lifetime and any required zeroization.
    ///
    /// **日本語:** ラッパーを消費し、ゼロ化せずに内包値を返します。
    ///
    /// ラッパーのデストラクターは実行しません。返された値の生存期間と、必要なゼロ化の管理は
    /// 呼び出し側の責任になります。
    ///
    /// ```rust
    /// use ddd_toolkit_core::Secret;
    /// use zeroize::Zeroize;
    ///
    /// let mut value = Secret::new(vec![1_u8, 2, 3]).into_inner();
    /// assert_eq!(value, [1, 2, 3]);
    /// value.zeroize();
    /// ```
    pub fn into_inner(self) -> T {
        let mut this = ManuallyDrop::new(self);
        // Prevent Secret::drop from running after taking the value.
        unsafe { ManuallyDrop::take(&mut this.value) }
    }

    /// **English:** Calls `f` once with a shared reference to the value and returns its result.
    ///
    /// This does not consume or zeroize the value. The closure can copy it or return a borrowed
    /// reference; the closure is not an isolation boundary. Any panic from `f` propagates.
    ///
    /// **日本語:** 値への共有参照を渡して `f` を1回呼び出し、その結果を返します。
    ///
    /// 値の消費やゼロ化は行いません。クロージャーは値をコピーしたり借用した参照を返したりでき、
    /// 情報の隔離境界にはなりません。`f` がパニックした場合はそのまま伝播します。
    pub fn expose_secret<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        f(self.as_inner())
    }
}

impl<T: Zeroize> Drop for Secret<T> {
    fn drop(&mut self) {
        unsafe {
            (&mut *(&mut self.value as *mut ManuallyDrop<T> as *mut T)).zeroize();
            ManuallyDrop::drop(&mut self.value);
        }
    }
}

impl<T: Zeroize + Clone> Clone for Secret<T> {
    fn clone(&self) -> Self {
        Self::new(self.as_inner().clone())
    }
}

impl<T: Zeroize + PartialEq> PartialEq for Secret<T> {
    fn eq(&self, other: &Self) -> bool {
        self.as_inner() == other.as_inner()
    }
}

impl<T: Zeroize + Eq> Eq for Secret<T> {}

impl<T: Zeroize> fmt::Debug for Secret<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("[REDACTED]")
    }
}

impl<T: Zeroize> fmt::Display for Secret<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("[REDACTED]")
    }
}
