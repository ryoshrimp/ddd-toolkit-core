/// **English:** A zero-sized marker for metadata associated with a secret.
///
/// It stores no secret value or descriptive fields. The marker alone does not redact,
/// zeroize, or restrict access to any data; use [`crate::Secret`] to wrap a value.
///
/// **日本語:** 秘密情報に関連するメタデータのための、サイズがゼロのマーカーです。
///
/// 秘密の値や説明用のフィールドは保持しません。このマーカー自体はデータの秘匿化、ゼロ化、
/// アクセス制限を行いません。値を包むには [`crate::Secret`] を使います。
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct SecretMetadata;
