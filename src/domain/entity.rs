use crate::domain::EntityId;

/// A domain object whose equality is defined by its identity, not its
/// attributes - the same conceptual entity can change all of its other
/// fields over time and still be "the same" entity.
///
/// This trait deliberately does not require [`PartialEq`]/[`Eq`]: nothing
/// stops an implementor from also deriving `PartialEq`, but a derived
/// `PartialEq` compares *every* field structurally, which is almost never
/// the identity comparison a DDD Entity is supposed to have. Use
/// [`Entity::is_same_as`] wherever "is this the same entity" is the
/// question being asked; if you do implement `PartialEq`/`Eq` for an
/// `Entity`, implement it in terms of `is_same_as` rather than deriving it,
/// so `==` and `is_same_as` cannot silently disagree.
pub trait Entity: Send + Sync {
    type Id: EntityId;

    fn id(&self) -> &Self::Id;

    /// Identity comparison: true iff `self` and `other` are the same
    /// entity, regardless of whether their other fields currently match.
    fn is_same_as(&self, other: &Self) -> bool {
        self.id() == other.id()
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

    struct Foo {
        id: FooId,
        value: String,
    }

    impl Entity for Foo {
        type Id = FooId;
        fn id(&self) -> &Self::Id {
            &self.id
        }
    }

    #[test]
    fn id_returns_reference_to_entity_id() {
        let foo = Foo {
            id: FooId("foo-1".to_string()),
            value: "value".to_string(),
        };

        assert_eq!(foo.id(), &FooId("foo-1".to_string()));
    }

    #[test]
    fn is_same_as_returns_true_for_same_id_and_same_value() {
        let a = Foo {
            id: FooId("foo-1".to_string()),
            value: "value".to_string(),
        };
        let b = Foo {
            id: FooId("foo-1".to_string()),
            value: "value".to_string(),
        };

        assert!(a.is_same_as(&b));
    }

    #[test]
    fn is_same_as_returns_true_for_same_id_and_different_value() {
        let a = Foo {
            id: FooId("foo-1".to_string()),
            value: "value-a".to_string(),
        };
        let b = Foo {
            id: FooId("foo-1".to_string()),
            value: "value-b".to_string(),
        };

        assert_ne!(a.value, b.value);
        assert!(a.is_same_as(&b));
    }

    #[test]
    fn is_same_as_returns_false_for_different_id_and_same_value() {
        let a = Foo {
            id: FooId("foo-1".to_string()),
            value: "value".to_string(),
        };
        let b = Foo {
            id: FooId("foo-2".to_string()),
            value: "value".to_string(),
        };

        assert!(!a.is_same_as(&b));
    }

    #[test]
    fn is_same_as_returns_true_for_self_comparison() {
        let foo = Foo {
            id: FooId("foo-1".to_string()),
            value: "value".to_string(),
        };

        assert!(foo.is_same_as(&foo));
    }

    #[test]
    fn is_same_as_returns_true_for_empty_string_ids() {
        let a = Foo {
            id: FooId(String::new()),
            value: "value-a".to_string(),
        };
        let b = Foo {
            id: FooId(String::new()),
            value: "value-b".to_string(),
        };

        assert!(a.is_same_as(&b));
    }

    #[test]
    fn entity_impl_satisfies_send_and_sync() {
        fn assert_impl<T: Send + Sync>() {}
        assert_impl::<Foo>();
    }
}
