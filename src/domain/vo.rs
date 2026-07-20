use std::fmt::Debug;

/// A domain object with no identity, whose equality is purely structural:
/// two value objects are the same if their data is the same.
pub trait ValueObject: Clone + PartialEq + Debug + Send + Sync {}

/// A type that wraps a single inner value.
pub trait WrappedInner {
    /// The wrapped type.
    type Inner;
}

/// A [`ValueObject`] newtype around a single [`WrappedInner::Inner`] value,
/// only constructible through validation (`TryFrom<Self::Inner>`).
///
/// Note this invariant only holds if the wrapped field is private - see
/// `#[derive(ValueObject)]` in `ddd-toolkit-macro`, which enforces that at
/// derive time.
pub trait Wrapped:
    ValueObject
    + WrappedInner
    + TryFrom<<Self as WrappedInner>::Inner, Error: std::error::Error + Send + Sync + 'static>
{
    /// Borrows the wrapped value.
    fn as_inner(&self) -> &Self::Inner;
    /// Consumes `self`, returning the wrapped value.
    fn into_inner(self) -> Self::Inner;
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::domain::ValidationError;

    #[derive(Clone, PartialEq, Debug)]
    struct FooValue(String);

    impl ValueObject for FooValue {}

    impl WrappedInner for FooValue {
        type Inner = String;
    }

    impl Wrapped for FooValue {
        fn as_inner(&self) -> &Self::Inner {
            &self.0
        }

        fn into_inner(self) -> Self::Inner {
            self.0
        }
    }

    impl TryFrom<String> for FooValue {
        type Error = ValidationError;

        fn try_from(value: String) -> Result<Self, Self::Error> {
            if value.trim().is_empty() {
                return Err(ValidationError::new("FooValue", "empty"));
            }
            Ok(Self(value))
        }
    }

    #[test]
    fn try_from_non_empty_string_succeeds() {
        assert!(FooValue::try_from("value".to_string()).is_ok())
    }

    #[test]
    fn as_inner_returns_reference_to_original_value() {
        let foo = FooValue("value".to_string());
        assert_eq!(foo.as_inner(), "value")
    }

    #[test]
    fn into_inner_returns_owned_original_value() {
        let foo = FooValue("value".to_string());
        assert_eq!(foo.into_inner(), "value".to_string())
    }

    #[test]
    fn round_trip_preserves_value() {
        let foo = FooValue::try_from("value".to_string()).unwrap();
        assert_eq!(foo.into_inner(), "value".to_string())
    }

    #[test]
    fn clone_produces_equal_value() {
        let foo = FooValue("value".to_string());
        let bar = foo.clone();

        assert_eq!(foo, bar);
        assert_eq!(foo.into_inner(), bar.into_inner());
    }

    #[test]
    fn eq_returns_true_for_same_inner_value() {
        let foo = FooValue("foo".to_string());
        let bar = FooValue("foo".to_string());

        assert_eq!(foo, bar);
        assert_eq!(foo.into_inner(), bar.into_inner());
    }

    #[test]
    fn eq_returns_false_for_different_inner_value() {
        let foo = FooValue("foo".to_string());
        let bar = FooValue("bar".to_string());

        assert_ne!(foo, bar);
        assert_ne!(foo.into_inner(), bar.into_inner());
    }

    #[test]
    fn try_from_empty_string_returns_err() {
        assert!(FooValue::try_from("".to_string()).is_err());
        assert!(FooValue::try_from("  ".to_string()).is_err());
    }

    #[test]
    fn try_from_empty_string_error_has_expected_type_name() {
        let err = FooValue::try_from("".to_string()).unwrap_err();
        assert_eq!(err.type_name, "FooValue")
    }

    #[test]
    fn try_from_empty_string_error_has_expected_reason() {
        let err = FooValue::try_from("".to_string()).unwrap_err();
        assert_eq!(err.reason, "empty")
    }

    #[test]
    fn try_from_empty_string_error_display_message() {
        let err = FooValue::try_from("".to_string()).unwrap_err();
        assert_eq!(format!("{err}"), "invalid FooValue: empty")
    }
}
