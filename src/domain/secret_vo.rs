use crate::domain::ValueObject;

/// A value object that wraps a secret (API key, password, token, ...) which
/// must never appear in logs or error messages by accident.
///
/// This trait cannot force a redacting `Debug` impl: `ValueObject` requires
/// `Debug` to exist, but nothing in the type system can require *what* it
/// prints. A hand-written `impl SecretVo` (as in this crate's own test
/// fixtures) is only as safe as its own `Debug`/`Display` impls - it is the
/// implementor's responsibility to redact. `#[derive(SecretVo)]` in
/// `ddd-toolkit-macro` closes this gap for derived types: it always
/// generates a `Debug` impl that prints `TypeName(***)` and never the
/// wrapped value, so prefer the derive over a manual impl wherever possible.
pub trait SecretVo: ValueObject {
    /// The wrapped secret's raw type.
    type Inner;
    /// The error produced when a value fails validation.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Validates and wraps `inner`.
    fn try_new(inner: Self::Inner) -> Result<Self, Self::Error>;

    /// Returns the wrapped secret. Named to make call sites (`key.expose_secret()`
    /// rather than `key.0`) an explicit, greppable signal that a secret is
    /// about to be handled directly.
    fn expose_secret(&self) -> &Self::Inner;
}

#[cfg(test)]
mod test {
    use std::fmt::{Debug, Display};

    use crate::domain::ValidationError;

    use super::*;

    #[derive(Clone, PartialEq)]
    struct FooSecret(String);

    impl Display for FooSecret {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "[REDACTED]")
        }
    }

    impl Debug for FooSecret {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "FooSecret([REDACTED])")
        }
    }

    impl ValueObject for FooSecret {}
    impl SecretVo for FooSecret {
        type Inner = String;
        type Error = ValidationError;

        fn try_new(inner: Self::Inner) -> Result<Self, Self::Error> {
            if inner.trim().is_empty() {
                return Err(ValidationError::new("FooSecret", "empty"));
            }
            Ok(Self(inner))
        }

        fn expose_secret(&self) -> &Self::Inner {
            &self.0
        }
    }

    #[test]
    fn try_new_non_empty_string_succeeds() {
        assert!(FooSecret::try_new("foo".to_string()).is_ok())
    }

    #[test]
    fn expose_secret_returns_reference_to_original_value() {
        let fs = FooSecret::try_new("foo".to_string()).unwrap();
        assert_eq!(fs.expose_secret(), "foo");
    }

    #[test]
    fn clone_produces_equal_value() {
        let fs = FooSecret::try_new("foo".to_string()).unwrap();
        let fs_clone = fs.clone();
        assert_eq!(fs, fs_clone);
        assert_eq!(fs.expose_secret(), fs_clone.expose_secret());
    }

    #[test]
    fn eq_returns_true_for_same_inner_value() {
        let fs = FooSecret::try_new("foo".to_string()).unwrap();
        let fs_2 = FooSecret::try_new("foo".to_string()).unwrap();
        assert_eq!(fs, fs_2);
        assert_eq!(fs.expose_secret(), fs_2.expose_secret());
    }

    #[test]
    fn eq_returns_false_for_different_inner_value() {
        let fs = FooSecret::try_new("foo".to_string()).unwrap();
        let fs_2 = FooSecret::try_new("bar".to_string()).unwrap();
        assert_ne!(fs, fs_2);
        assert_ne!(fs.expose_secret(), fs_2.expose_secret());
    }

    #[test]
    fn display_redacts_secret_value() {
        let fs = FooSecret::try_new("foo".to_string()).unwrap();
        assert_eq!(format!("{fs}"), "[REDACTED]");
    }

    #[test]
    fn debug_redacts_secret_value() {
        let fs = FooSecret::try_new("foo".to_string()).unwrap();
        assert_eq!(format!("{:?}", fs), "FooSecret([REDACTED])");
    }

    #[test]
    fn try_new_empty_string_returns_err() {
        assert!(FooSecret::try_new("".to_string()).is_err());
        assert!(FooSecret::try_new("  ".to_string()).is_err())
    }

    #[test]
    fn try_new_empty_string_error_has_expected_type_name() {
        let err = FooSecret::try_new("".to_string()).unwrap_err();
        assert_eq!(err.type_name, "FooSecret")
    }

    #[test]
    fn try_new_empty_string_error_has_expected_reason() {
        let err = FooSecret::try_new("".to_string()).unwrap_err();
        assert_eq!(err.reason, "empty")
    }

    #[test]
    fn try_new_empty_string_error_display_message() {
        let err = FooSecret::try_new("".to_string()).unwrap_err();
        assert_eq!(format!("{err}"), "invalid FooSecret: empty")
    }

    #[test]
    fn error_type_coerces_to_boxed_error() {
        let err = FooSecret::try_new("".to_string()).unwrap_err();
        let _: Box<dyn std::error::Error + Send + Sync> = Box::new(err);
    }
}
