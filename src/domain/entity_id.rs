use crate::domain::ValueObject;
use std::{fmt::Display, hash::Hash};

pub trait EntityId: ValueObject + Eq + Hash + Ord + Display {}

#[cfg(test)]
mod test {
    use super::*;
    use std::cmp::Ordering;
    use std::collections::{HashMap, HashSet};

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
    fn eq_returns_true_for_same_inner_value() {
        let foo = FooId("foo".to_string());
        let bar = FooId("foo".to_string());

        assert_eq!(foo, bar);
    }

    #[test]
    fn eq_returns_false_for_different_inner_value() {
        let foo = FooId("foo".to_string());
        let bar = FooId("bar".to_string());

        assert_ne!(foo, bar);
    }

    #[test]
    fn clone_produces_equal_value() {
        let foo = FooId("foo".to_string());
        let bar = foo.clone();

        assert_eq!(foo, bar);
    }

    #[test]
    fn display_formats_inner_value() {
        let foo = FooId("foo".to_string());

        assert_eq!(format!("{foo}"), "foo");
    }

    #[test]
    fn ord_orders_by_inner_value() {
        let a = FooId("a".to_string());
        let b = FooId("b".to_string());

        assert!(a < b);
    }

    #[test]
    fn hash_set_deduplicates_equal_ids() {
        let foo = FooId("foo".to_string());
        let bar = FooId("foo".to_string());

        let set: HashSet<FooId> = HashSet::from([foo, bar]);
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn hash_map_lookup_succeeds_with_equal_key() {
        let key = FooId("foo".to_string());
        let map = HashMap::from([(key, 42)]);

        let lookup = FooId("foo".to_string());
        assert_eq!(map.get(&lookup), Some(&42));
    }

    #[test]
    fn sort_orders_ids_lexicographically() {
        let mut ids = vec![
            FooId("c".to_string()),
            FooId("a".to_string()),
            FooId("b".to_string()),
        ];
        ids.sort();

        assert_eq!(
            ids,
            vec![
                FooId("a".to_string()),
                FooId("b".to_string()),
                FooId("c".to_string()),
            ]
        );
    }

    #[test]
    fn foo_id_satisfies_entity_id_bounds() {
        fn assert_impl<T: EntityId>() {}
        assert_impl::<FooId>();
    }

    #[test]
    fn display_of_empty_inner_value_is_empty_string() {
        let foo = FooId(String::new());

        assert_eq!(format!("{foo}"), "");
    }

    #[test]
    fn cmp_returns_equal_for_same_inner_value() {
        let foo = FooId("foo".to_string());
        let bar = FooId("foo".to_string());

        assert_eq!(foo.cmp(&bar), Ordering::Equal);
    }
}
