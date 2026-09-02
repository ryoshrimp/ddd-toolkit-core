/// Builds an aggregate, entity or complex value object that is valid from
/// the moment it exists.
///
/// A factory encapsulates construction that is too involved for a plain
/// constructor: it checks the invariants, generates an id or timestamp, and
/// either hands back a fully formed object or refuses with `Error`. Callers
/// depend on the `Factory` abstraction rather than on a concrete
/// constructor, so the construction rules can change (or be swapped in a
/// test) without touching them.
///
/// The associated types mirror
/// [`UseCase`](crate::application::usecase::UseCase) so the two read the
/// same way, but `create` is synchronous: factories are pure domain
/// construction. If a real backend must be consulted first, that belongs
/// in a use case that then calls the factory.
///
/// `Output` is deliberately unbounded. `Error` is not fixed either;
/// [`ValidationError`](crate::domain::ValidationError) is the expected
/// common choice and is what the example uses.
///
/// A factory that needs ids or timestamps owns the port as a field
/// ([`IdGenerator`](crate::port::id::IdGenerator) and
/// [`Clock`](crate::port::clock::Clock) are both synchronous) and calls it
/// inside `create`, as the example does with
/// [`FixedIdGenerator`](crate::mock::id::FixedIdGenerator).
///
/// # Examples
///
/// ```
/// use ddd_toolkit_core::domain::{Entity, EntityId, Factory, ValidationError, ValueObject};
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
/// #[derive(Debug, Clone, PartialEq, Eq)]
/// struct LineItem {
///     sku: String,
///     quantity: u32,
/// }
///
/// impl ValueObject for LineItem {}
///
/// #[derive(Debug, PartialEq)]
/// struct Order {
///     id: OrderId,
///     items: Vec<LineItem>,
/// }
///
/// impl Entity for Order {
///     type Id = OrderId;
///
///     fn id(&self) -> &Self::Id {
///         &self.id
///     }
/// }
///
/// // The factory owns the id port it needs. In production this would be a
/// // `UuidV4Generator`; a fixed generator makes the example deterministic.
/// struct OrderFactory {
///     ids: FixedIdGenerator<OrderId>,
/// }
///
/// impl Factory for OrderFactory {
///     type Input = Vec<LineItem>;
///     type Output = Order;
///     type Error = ValidationError;
///
///     fn create(&self, items: Self::Input) -> Result<Self::Output, Self::Error> {
///         if items.is_empty() {
///             return Err(ValidationError::new(
///                 "Order",
///                 "must contain at least one line item",
///             ));
///         }
///         Ok(Order { id: self.ids.generate(), items })
///     }
/// }
///
/// let factory = OrderFactory { ids: FixedIdGenerator(OrderId(1)) };
///
/// let order = factory.create(vec![LineItem { sku: "sku-1".to_string(), quantity: 2 }])?;
/// assert_eq!(order.id(), &OrderId(1));
/// assert_eq!(order.items.len(), 1);
///
/// // an `Order` with no items never exists, not even briefly
/// assert_eq!(
///     factory.create(vec![]),
///     Err(ValidationError::new("Order", "must contain at least one line item"))
/// );
/// # Ok::<(), ValidationError>(())
/// ```
pub trait Factory: Send + Sync {
    /// What the factory needs to build one instance.
    type Input: Send;
    /// What it builds.
    type Output;
    /// Why construction can be refused.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Builds one `Output` from `input`, or refuses with `Error` if the
    /// result would violate an invariant.
    fn create(&self, input: Self::Input) -> Result<Self::Output, Self::Error>;
}

#[cfg(test)]
mod test {
    use std::fmt::Display;

    use crate::{
        domain::{Entity, EntityId, Factory, ValidationError, ValueObject},
        mock::id::FixedIdGenerator,
        port::id::IdGenerator,
    };

    #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
    struct FooId(String);

    impl ValueObject for FooId {}

    impl Display for FooId {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl EntityId for FooId {}

    #[derive(Debug, PartialEq)]
    struct Foo {
        id: FooId,
        name: String,
    }

    impl Entity for Foo {
        type Id = FooId;

        fn id(&self) -> &Self::Id {
            &self.id
        }
    }

    // A factory that owns the id port it needs, as the docs recommend.
    struct FooFactory {
        ids: FixedIdGenerator<FooId>,
    }

    impl Factory for FooFactory {
        type Input = String;
        type Output = Foo;
        type Error = ValidationError;

        fn create(&self, input: Self::Input) -> Result<Self::Output, Self::Error> {
            if input.trim().is_empty() {
                return Err(ValidationError::new("Foo", "name must not be empty"));
            }
            Ok(Foo {
                id: self.ids.generate(),
                name: input,
            })
        }
    }

    // A factory with a custom error type, to prove `Error` is not fixed to
    // `ValidationError`.
    #[derive(Debug, PartialEq)]
    struct BarError;

    impl Display for BarError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "bar failed")
        }
    }

    impl std::error::Error for BarError {}

    struct FailingFactory;

    impl Factory for FailingFactory {
        type Input = ();
        type Output = ();
        type Error = BarError;

        fn create(&self, _input: Self::Input) -> Result<Self::Output, Self::Error> {
            Err(BarError)
        }
    }

    fn foo_factory() -> FooFactory {
        FooFactory {
            ids: FixedIdGenerator(FooId("id-1".to_string())),
        }
    }

    #[test]
    fn create_returns_ok_on_valid_input() {
        let factory = foo_factory();

        let foo = factory
            .create("foo".to_string())
            .expect("create should succeed");

        assert_eq!(foo.name, "foo");
    }

    #[test]
    fn create_returns_implementors_error_on_invalid_input() {
        let factory = foo_factory();

        let error = factory
            .create("   ".to_string())
            .expect_err("create should fail");

        assert_eq!(error, ValidationError::new("Foo", "name must not be empty"));
    }

    #[test]
    fn create_returns_custom_error_type() {
        let error = FailingFactory.create(()).expect_err("create should fail");

        assert_eq!(error, BarError);
        assert_eq!(error.to_string(), "bar failed");
    }

    #[test]
    fn factory_holding_fixed_id_generator_produces_the_fixed_id() {
        let factory = foo_factory();

        let foo = factory
            .create("foo".to_string())
            .expect("create should succeed");

        assert_eq!(foo.id(), &FooId("id-1".to_string()));
    }

    #[test]
    fn create_with_shared_reference_allows_multiple_calls() {
        let factory = foo_factory();

        let first = factory
            .create("a".to_string())
            .expect("first create should succeed");
        let second = factory
            .create("b".to_string())
            .expect("second create should succeed");

        assert_eq!(first.name, "a");
        assert_eq!(second.name, "b");
        assert!(first.is_same_as(&second)); // same fixed id
    }

    // Verifies the trait bounds are usable from generic code; the runtime
    // assertion is secondary to the fact that this compiles.
    #[test]
    fn factory_is_usable_through_generic_bound() {
        fn build<F: Factory>(factory: &F, input: F::Input) -> Result<F::Output, F::Error> {
            factory.create(input)
        }

        let foo =
            build(&foo_factory(), "generic".to_string()).expect("generic create should succeed");

        assert_eq!(foo.name, "generic");
    }

    // Verifies the `Error: std::error::Error + Send + Sync + 'static` bound is
    // strong enough for the usual boxed-error upcast.
    #[test]
    fn error_can_be_boxed_as_dyn_error() {
        fn boxed<F: Factory>(error: F::Error) -> Box<dyn std::error::Error + Send + Sync> {
            Box::new(error)
        }

        let error = foo_factory()
            .create(String::new())
            .expect_err("create should fail");

        let boxed = boxed::<FooFactory>(error);

        assert_eq!(boxed.to_string(), "invalid Foo: name must not be empty");
    }

    #[test]
    fn factory_bound_implies_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        fn check<F: Factory>() {
            assert_send_sync::<F>();
        }

        check::<FooFactory>();
    }
}
