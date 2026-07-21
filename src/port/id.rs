use crate::domain::EntityId;

/// Generates new [`EntityId`]s, e.g. for a new aggregate.
///
/// [`crate::adapter::id::UuidV4Generator`] and
/// [`crate::adapter::id::UuidV7Generator`] generate real, unique ids;
/// [`crate::mock::id::FixedIdGenerator`] is a test double that always
/// returns the same configured id.
///
/// # Examples
///
/// ```
/// use ddd_toolkit_core::domain::{EntityId, ValueObject};
/// use ddd_toolkit_core::port::id::IdGenerator;
/// use std::fmt::Display;
/// use std::sync::atomic::{AtomicU32, Ordering};
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
/// struct SequentialGenerator(AtomicU32);
///
/// impl IdGenerator<OrderId> for SequentialGenerator {
///     fn generate(&self) -> OrderId {
///         OrderId(self.0.fetch_add(1, Ordering::Relaxed))
///     }
/// }
///
/// let generator = SequentialGenerator(AtomicU32::new(1));
///
/// assert_eq!(generator.generate(), OrderId(1));
/// assert_eq!(generator.generate(), OrderId(2));
/// ```
pub trait IdGenerator<Id: EntityId>: Send + Sync {
    /// Generates a new id.
    fn generate(&self) -> Id;
}

#[cfg(test)]
mod test {
    use std::{
        fmt::Display,
        sync::atomic::{AtomicU32, Ordering},
    };

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

    struct SequentialFooIdGenerator {
        counter: AtomicU32,
    }

    impl SequentialFooIdGenerator {
        fn new() -> Self {
            Self {
                counter: AtomicU32::new(0),
            }
        }
    }

    impl IdGenerator<FooId> for SequentialFooIdGenerator {
        fn generate(&self) -> FooId {
            let n = self.counter.fetch_add(1, Ordering::Relaxed) + 1;
            FooId(format!("id-{n}"))
        }
    }

    #[test]
    fn generate_returns_id_from_generator_impl() {
        let generator = SequentialFooIdGenerator::new();

        assert_eq!(generator.generate(), FooId("id-1".to_string()));
        assert_eq!(generator.generate(), FooId("id-2".to_string()));
    }

    #[test]
    fn generic_fn_accepts_id_generator_impl() {
        fn next_id<G: IdGenerator<FooId>>(generator: &G) -> FooId {
            generator.generate()
        }

        let generator = SequentialFooIdGenerator::new();

        assert_eq!(next_id(&generator), FooId("id-1".to_string()));
    }

    #[test]
    fn box_dyn_id_generator_is_usable() {
        let generator: Box<dyn IdGenerator<FooId>> = Box::new(SequentialFooIdGenerator::new());

        assert_eq!(generator.generate(), FooId("id-1".to_string()));
    }

    #[test]
    fn id_generator_bound_implies_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        fn check<G: IdGenerator<FooId>>() {
            assert_send_sync::<G>();
        }

        check::<SequentialFooIdGenerator>();
    }
}
