/// **English:** Marks a type as an event that describes a meaningful domain occurrence.
///
/// This is a marker trait with no required methods or bounds. Event payloads, serialization,
/// equality, and dispatch behavior are defined by the implementing type and its integrations.
///
/// **日本語:** 意味のあるドメイン上の出来事を表す型であることを示します。
///
/// 必須メソッドや境界を持たないマーカートレイトです。イベントのペイロード、シリアライズ、
/// 等価性、ディスパッチの動作は実装型と連携先で定義します。
pub trait DomainEvent {}
