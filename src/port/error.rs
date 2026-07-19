use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PortErrorKind {
    Conflict,
    Unavailable,
    Other,
}

#[derive(Debug)]
pub struct PortError {
    kind: PortErrorKind,
    source: Box<dyn std::error::Error + Send + Sync>,
}

impl PortError {
    pub fn new(
        kind: PortErrorKind,
        source: impl Into<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self {
            kind,
            source: source.into(),
        }
    }

    pub fn conflict(source: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        Self::new(PortErrorKind::Conflict, source)
    }

    pub fn unavailable(source: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        Self::new(PortErrorKind::Unavailable, source)
    }

    pub fn other(source: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        Self::new(PortErrorKind::Other, source)
    }

    pub fn kind(&self) -> PortErrorKind {
        self.kind
    }
}

impl Display for PortError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.source)
    }
}

impl std::error::Error for PortError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.source.as_ref())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn new_returns_error_with_given_kind() {
        let error = PortError::new(PortErrorKind::Conflict, "boom");

        assert_eq!(error.kind(), PortErrorKind::Conflict);
    }

    #[test]
    fn conflict_returns_error_with_conflict_kind() {
        let error = PortError::conflict("boom");

        assert_eq!(error.kind(), PortErrorKind::Conflict);
    }

    #[test]
    fn unavailable_returns_error_with_unavailable_kind() {
        let error = PortError::unavailable("boom");

        assert_eq!(error.kind(), PortErrorKind::Unavailable);
    }

    #[test]
    fn other_returns_error_with_other_kind() {
        let error = PortError::other("boom");

        assert_eq!(error.kind(), PortErrorKind::Other);
    }

    #[test]
    fn source_returns_inner_error() {
        let error: &dyn std::error::Error = &PortError::conflict("boom");

        let source = error.source().expect("source should be Some");
        assert_eq!(source.to_string(), "boom");
    }

    #[derive(Debug)]
    struct MiddleError {
        inner: Box<dyn std::error::Error + Send + Sync>,
    }

    impl Display for MiddleError {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(f, "middle")
        }
    }

    impl std::error::Error for MiddleError {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            Some(self.inner.as_ref())
        }
    }

    #[test]
    fn source_preserves_nested_error_chain() {
        let middle = MiddleError {
            inner: "root".into(),
        };
        let error: &dyn std::error::Error = &PortError::other(middle);

        let middle = error.source().expect("source should be Some");
        let root = middle.source().expect("nested source should be Some");
        assert_eq!(middle.to_string(), "middle");
        assert_eq!(root.to_string(), "root");
    }

    #[test]
    fn display_formats_kind_and_source_message() {
        let error = PortError::conflict("boom");

        assert_eq!(error.to_string(), "Conflict: boom");
    }

    #[test]
    fn new_accepts_str_string_and_custom_error() {
        let _ = PortError::new(PortErrorKind::Other, "str source");
        let _ = PortError::new(PortErrorKind::Other, String::from("string source"));
        let _ = PortError::new(
            PortErrorKind::Other,
            MiddleError {
                inner: "root".into(),
            },
        );
    }

    #[test]
    fn kind_copy_produces_equal_value() {
        let kind = PortErrorKind::Unavailable;
        let copied = kind;

        assert_eq!(copied, kind);
    }

    #[test]
    fn kind_eq_returns_true_for_same_variant() {
        assert_eq!(PortErrorKind::Conflict, PortErrorKind::Conflict);
    }

    #[test]
    fn kind_eq_returns_false_for_different_variant() {
        assert_ne!(PortErrorKind::Conflict, PortErrorKind::Unavailable);
    }

    #[test]
    fn display_with_empty_source_message_ends_with_colon_space() {
        let error = PortError::other("");

        assert_eq!(error.to_string(), "Other: ");
    }
}
