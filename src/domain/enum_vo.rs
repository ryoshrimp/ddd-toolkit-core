use crate::domain::ValueObject;
use core::str::FromStr;
use std::fmt::Display;

pub trait EnumVo:
    ValueObject + FromStr<Err: std::error::Error + Send + Sync + 'static> + Display + Copy + 'static
{
    fn variants() -> &'static [Self];
}

#[cfg(test)]
mod test {
    use crate::domain::ValidationError;

    use super::*;
    use std::fmt::Display;

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum Baz {
        Foo,
        Bar,
    }

    impl Display for Baz {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Baz::Foo => write!(f, "foo"),
                Baz::Bar => write!(f, "bar"),
            }
        }
    }

    impl FromStr for Baz {
        type Err = ValidationError;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            if s.eq("foo") {
                Ok(Self::Foo)
            } else if s.eq("bar") {
                Ok(Self::Bar)
            } else {
                Err(ValidationError::new("Baz", "unknown variant"))
            }
        }
    }

    impl ValueObject for Baz {}

    impl EnumVo for Baz {
        fn variants() -> &'static [Self] {
            &[Self::Bar, Self::Foo]
        }
    }

    #[test]
    fn from_str_foo_returns_foo_variant() {
        assert_eq!("foo".parse(), Ok(Baz::Foo))
    }

    #[test]
    fn from_str_bar_returns_bar_variant() {
        assert_eq!("bar".parse(), Ok(Baz::Bar))
    }

    #[test]
    fn display_formats_foo_variant() {
        assert_eq!(format!("{}", Baz::Foo), "foo")
    }

    #[test]
    fn display_formats_bar_variant() {
        assert_eq!(format!("{}", Baz::Bar), "bar")
    }

    #[test]
    fn variants_returns_all_variants() {
        let variants = Baz::variants();

        assert_eq!(variants.len(), 2);
        assert!(variants.contains(&Baz::Foo));
        assert!(variants.contains(&Baz::Bar))
    }

    #[test]
    fn round_trip_from_str_of_display_preserves_variant() {
        for v in Baz::variants() {
            assert_eq!(v.to_string().parse(), Ok(*v))
        }
    }

    #[test]
    fn copy_produces_equal_value() {
        let foo = Baz::Foo;
        let copied = foo;

        assert_eq!(foo, copied)
    }

    #[test]
    fn eq_returns_true_for_same_variant() {
        assert_eq!(Baz::Foo, Baz::Foo)
    }

    #[test]
    fn eq_returns_false_for_different_variant() {
        assert_ne!(Baz::Foo, Baz::Bar)
    }

    #[test]
    fn from_str_unknown_string_returns_err() {
        assert!("qux".parse::<Baz>().is_err())
    }

    #[test]
    fn from_str_unknown_string_error_has_expected_type_name() {
        let err = "qux".parse::<Baz>().unwrap_err();
        assert_eq!(err.type_name, "Baz")
    }

    #[test]
    fn from_str_unknown_string_error_has_expected_reason() {
        let err = "qux".parse::<Baz>().unwrap_err();
        assert_eq!(err.reason, "unknown variant")
    }

    #[test]
    fn from_str_unknown_string_error_display_message() {
        let err = "qux".parse::<Baz>().unwrap_err();
        assert_eq!(format!("{err}"), "invalid Baz: unknown variant")
    }

    #[test]
    fn from_str_empty_string_returns_err() {
        assert!("".parse::<Baz>().is_err())
    }

    #[test]
    fn from_str_is_case_sensitive() {
        assert!("Foo".parse::<Baz>().is_err());
        assert!("BAR".parse::<Baz>().is_err())
    }

    #[test]
    fn from_str_rejects_surrounding_whitespace() {
        assert!(" foo".parse::<Baz>().is_err());
        assert!("bar ".parse::<Baz>().is_err())
    }
}
