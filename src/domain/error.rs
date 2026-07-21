use std::{error::Error, fmt::Display};

/// A validation failure for a domain value, naming the type that rejected
/// the value and why.
///
/// `type_name` is a plain string, not tied to `Self` by the compiler: it is
/// whatever the caller of [`ValidationError::new`] passes in. Derive-macro
/// generated code always passes the exact type name it was expanded for, so
/// it is guaranteed accurate there. A hand-written `impl` (as in this
/// crate's own test fixtures) must keep the `type_name` argument in sync
/// with the type by hand - copy-pasting an impl from one type to another is
/// the most likely way for it to drift.
///
/// # Examples
///
/// ```
/// use ddd_toolkit_core::domain::ValidationError;
///
/// let error = ValidationError::new("Email", "missing '@'");
///
/// assert_eq!(error.type_name, "Email");
/// assert_eq!(error.reason, "missing '@'");
/// assert_eq!(error.to_string(), "invalid Email: missing '@'");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    /// The name of the type that rejected the value.
    pub type_name: &'static str,
    /// A human-readable explanation of why the value was rejected.
    pub reason: String,
}

impl ValidationError {
    /// Creates a new `ValidationError` for the type named `type_name`.
    pub fn new(type_name: &'static str, reason: impl Into<String>) -> Self {
        Self {
            type_name,
            reason: reason.into(),
        }
    }
}

impl Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid {}: {}", self.type_name, self.reason)
    }
}

impl Error for ValidationError {}

#[cfg(test)]
mod test {
    use crate::domain::ValidationError;

    #[test]
    fn new_sets_type_name_field() {
        let ve = ValidationError::new("foo", "bar");
        assert_eq!(ve.type_name, "foo")
    }

    #[test]
    fn new_sets_reason_field() {
        let ve = ValidationError::new("foo", "bar");
        assert_eq!(ve.reason, "bar")
    }

    #[test]
    fn new_accepts_str_reason() {
        ValidationError::new("foo", "bar");
    }

    #[test]
    fn new_accepts_string_reason() {
        ValidationError::new("foo", "bar".to_string());
    }

    #[test]
    fn display_formats_invalid_type_and_reason() {
        let ve = ValidationError::new("foo", "bar");
        assert_eq!(format!("{ve}"), "invalid foo: bar")
    }

    #[test]
    fn clone_produces_equal_value() {
        let ve = ValidationError::new("foo", "bar");
        let ve_clone = ve.clone();

        assert_eq!(ve, ve_clone);
        assert_eq!(ve.type_name, ve_clone.type_name);
        assert_eq!(ve.reason, ve_clone.reason)
    }

    #[test]
    fn eq_returns_true_for_same_fields() {
        let ve = ValidationError::new("foo", "bar");
        let ve_2 = ValidationError::new("foo", "bar");

        assert_eq!(ve, ve_2);
        assert_eq!(ve.type_name, ve_2.type_name);
        assert_eq!(ve.reason, ve_2.reason)
    }

    #[test]
    fn eq_returns_false_for_different_type_name() {
        let ve = ValidationError::new("foo", "bar");
        let ve_2 = ValidationError::new("bar", "bar");

        assert_ne!(ve, ve_2);
        assert_ne!(ve.type_name, ve_2.type_name);
        assert_eq!(ve.reason, ve_2.reason)
    }

    #[test]
    fn eq_returns_false_for_different_reason() {
        let ve = ValidationError::new("foo", "bar");
        let ve_2 = ValidationError::new("foo", "foo");

        assert_ne!(ve, ve_2);
        assert_eq!(ve.type_name, ve_2.type_name);
        assert_ne!(ve.reason, ve_2.reason)
    }

    #[test]
    fn implements_std_error_trait() {
        let ve = ValidationError::new("foo", "bar");
        let err: &dyn std::error::Error = &ve;
        assert!(err.source().is_none())
    }

    #[test]
    fn new_accepts_empty_reason() {
        let ve = ValidationError::new("foo", "");
        assert!(ve.reason.is_empty())
    }

    #[test]
    fn display_formats_empty_reason() {
        let ve: ValidationError = ValidationError::new("foo", "");
        assert_eq!(format!("{}", ve), "invalid foo: ")
    }
}
