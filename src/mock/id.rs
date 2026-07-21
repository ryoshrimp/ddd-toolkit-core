use crate::{domain::EntityId, port::id::IdGenerator};

/// An [`IdGenerator`] that always returns the same, fixed id.
///
/// # Examples
///
/// ```
/// use ddd_toolkit_core::domain::{EntityId, ValueObject};
/// use ddd_toolkit_core::mock::id::FixedIdGenerator;
/// use ddd_toolkit_core::port::id::IdGenerator;
/// use std::fmt::Display;
///
/// #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
/// struct OrderId(u32);
///
/// impl ValueObject for OrderId {}
///
/// impl Display for OrderId {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         write!(f, "order-{}", self.0)
///     }
/// }
///
/// impl EntityId for OrderId {}
///
/// // useful in a test that needs a predictable id to assert against
/// let generator = FixedIdGenerator(OrderId(1));
///
/// assert_eq!(generator.generate(), OrderId(1));
/// assert_eq!(generator.generate(), OrderId(1));
/// ```
#[derive(Debug, Clone)]
pub struct FixedIdGenerator<Id>(pub Id);

impl<Id: EntityId> IdGenerator<Id> for FixedIdGenerator<Id> {
    fn generate(&self) -> Id {
        self.0.clone()
    }
}

#[cfg(test)]
mod test {
    use std::fmt::Display;

    use crate::domain::ValueObject;

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
    struct FooId(String);

    impl ValueObject for FooId {}

    impl Display for FooId {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl EntityId for FooId {}

    #[test]
    fn generate_returns_the_fixed_id() {
        let generator = FixedIdGenerator(FooId("id-1".to_string()));

        assert_eq!(generator.generate(), FooId("id-1".to_string()));
    }

    #[test]
    fn generate_returns_equal_id_on_repeated_calls() {
        let generator = FixedIdGenerator(FooId("id-1".to_string()));

        assert_eq!(generator.generate(), FooId("id-1".to_string()));
        assert_eq!(generator.generate(), FooId("id-1".to_string()));
        assert_eq!(generator.generate(), FooId("id-1".to_string()));
    }

    #[test]
    fn inner_field_is_publicly_accessible() {
        let mut generator = FixedIdGenerator(FooId("id-1".to_string()));

        assert_eq!(generator.0, FooId("id-1".to_string()));

        generator.0 = FooId("id-2".to_string());

        assert_eq!(generator.generate(), FooId("id-2".to_string()));
    }

    #[test]
    fn clone_produces_generator_returning_equal_id() {
        let generator = FixedIdGenerator(FooId("id-1".to_string()));

        let cloned = generator.clone();

        assert_eq!(cloned.generate(), generator.generate());
    }

    #[test]
    fn debug_format_contains_type_name_and_inner_id() {
        let generator = FixedIdGenerator(FooId("id-1".to_string()));

        let debug = format!("{generator:?}");

        assert!(debug.contains("FixedIdGenerator"));
        assert!(debug.contains("id-1"));
    }

    #[test]
    fn box_dyn_id_generator_is_usable() {
        let generator: Box<dyn IdGenerator<FooId>> =
            Box::new(FixedIdGenerator(FooId("id-1".to_string())));

        assert_eq!(generator.generate(), FooId("id-1".to_string()));
    }

    #[test]
    fn generate_returns_id_with_empty_inner_value_as_is() {
        let generator = FixedIdGenerator(FooId(String::new()));

        assert_eq!(generator.generate(), FooId(String::new()));
    }
}
