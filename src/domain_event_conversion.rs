//! **English:** Defines conversion behavior for a [`crate::DomainEvent`] value.
//!
//! Implement [`DomainEventConversion`] when an event can be transformed into another representation
//! and the transformation may fail.
//!
//! **日本語:** [`crate::DomainEvent`] の値を変換する動作を定義します。
//!
//! イベントを別の表現へ変換でき、その変換が失敗する可能性がある場合に
//! [`DomainEventConversion`] を実装します。

use crate::DomainEvent;

/// **English:** Converts a domain event into an associated output type.
///
/// The conversion borrows the event, so implementations can inspect it without taking ownership.
/// A failed conversion returns the associated [`Self::Error`] value and does not produce output.
/// The trait does not prescribe serialization, allocation, or whether the conversion is deterministic.
///
/// **日本語:** ドメインイベントを関連付けられた出力型へ変換します。
///
/// 変換はイベントを借用して行うため、所有権を取り上げずにイベントを検査できます。
/// 変換に失敗した場合は関連付けられた [`Self::Error`] の値を返し、出力は生成しません。
/// シリアライズ、メモリアロケーション、変換結果の決定性はこのトレイトでは規定しません。
pub trait DomainEventConversion: DomainEvent {
    /// **English:** The value produced when conversion succeeds.
    ///
    /// **日本語:** 変換に成功したときに生成される値です。
    type Output;

    /// **English:** The error returned when conversion fails.
    ///
    /// **日本語:** 変換に失敗したときに返されるエラーです。
    type Error;

    /// **English:** Converts this event without consuming it.
    ///
    /// Returns [`Self::Output`] on success or [`Self::Error`] when the event cannot be converted.
    /// Panics only if the implementation itself panics.
    ///
    /// **日本語:** このイベントを消費せずに変換します。
    ///
    /// 成功時は [`Self::Output`] を返し、イベントを変換できない場合は [`Self::Error`] を返します。
    /// パニックするのは実装自体がパニックする場合だけです。
    fn convert(&self) -> Result<Self::Output, Self::Error>;
}
