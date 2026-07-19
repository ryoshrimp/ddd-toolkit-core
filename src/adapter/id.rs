use crate::domain::EntityId;
use crate::port::id::IdGenerator;

#[derive(Debug, Clone)]
pub struct UuidV4Generator<Id>(core::marker::PhantomData<fn() -> Id>);

impl<Id> UuidV4Generator<Id> {
    pub fn new() -> Self {
        Self(core::marker::PhantomData)
    }
}

impl<Id> Default for UuidV4Generator<Id> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Id> IdGenerator<Id> for UuidV4Generator<Id>
where
    Id: EntityId + TryFrom<uuid::Uuid, Error = core::convert::Infallible>,
{
    fn generate(&self) -> Id {
        match Id::try_from(uuid::Uuid::new_v4()) {
            Ok(id) => id,
            Err(never) => match never {},
        }
    }
}

#[derive(Debug, Clone)]
pub struct UuidV7Generator<Id>(core::marker::PhantomData<fn() -> Id>);

impl<Id> UuidV7Generator<Id> {
    pub fn new() -> Self {
        Self(core::marker::PhantomData)
    }
}

impl<Id> Default for UuidV7Generator<Id> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Id> IdGenerator<Id> for UuidV7Generator<Id>
where
    Id: EntityId + TryFrom<uuid::Uuid, Error = core::convert::Infallible>,
{
    fn generate(&self) -> Id {
        match Id::try_from(uuid::Uuid::now_v7()) {
            Ok(id) => id,
            Err(never) => match never {},
        }
    }
}

#[cfg(test)]
mod test {
    use std::fmt::Display;

    use uuid::{Uuid, Variant};

    use crate::domain::ValueObject;

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
    struct UuidFooId(Uuid);

    impl ValueObject for UuidFooId {}

    impl Display for UuidFooId {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl EntityId for UuidFooId {}

    impl From<Uuid> for UuidFooId {
        fn from(value: Uuid) -> Self {
            Self(value)
        }
    }

    #[test]
    fn uuid_v4_generate_returns_version_4_rfc4122_uuid() {
        let generator = UuidV4Generator::<UuidFooId>::new();

        let id = generator.generate();

        assert_eq!(id.0.get_version_num(), 4);
        assert_eq!(id.0.get_variant(), Variant::RFC4122);
    }

    #[test]
    fn uuid_v4_generate_twice_returns_distinct_ids() {
        let generator = UuidV4Generator::<UuidFooId>::new();

        assert_ne!(generator.generate(), generator.generate());
    }

    #[test]
    fn uuid_v4_new_and_default_construct_usable_generator() {
        let from_new = UuidV4Generator::<UuidFooId>::new();
        let from_default = UuidV4Generator::<UuidFooId>::default();

        assert_eq!(from_new.generate().0.get_version_num(), 4);
        assert_eq!(from_default.generate().0.get_version_num(), 4);
    }

    #[test]
    fn uuid_v4_generator_works_through_id_generator_bound() {
        fn next_id<G: IdGenerator<UuidFooId>>(generator: &G) -> UuidFooId {
            generator.generate()
        }

        let generator = UuidV4Generator::new();

        assert_eq!(next_id(&generator).0.get_version_num(), 4);
    }

    #[test]
    fn uuid_v7_generate_returns_version_7_rfc4122_uuid() {
        let generator = UuidV7Generator::<UuidFooId>::new();

        let id = generator.generate();

        assert_eq!(id.0.get_version_num(), 7);
        assert_eq!(id.0.get_variant(), Variant::RFC4122);
    }

    #[test]
    fn uuid_v7_generate_twice_returns_distinct_ids() {
        let generator = UuidV7Generator::<UuidFooId>::new();

        assert_ne!(generator.generate(), generator.generate());
    }

    #[test]
    fn uuid_v7_timestamps_are_non_decreasing() {
        let generator = UuidV7Generator::<UuidFooId>::new();

        let timestamps: Vec<_> = (0..10)
            .map(|_| {
                generator
                    .generate()
                    .0
                    .get_timestamp()
                    .expect("v7 uuid should embed a timestamp")
                    .to_unix()
            })
            .collect();

        for pair in timestamps.windows(2) {
            assert!(pair[0] <= pair[1], "timestamps went backwards: {pair:?}");
        }
    }

    #[test]
    fn uuid_generators_are_send_sync_and_clone() {
        fn assert_impl<T: Send + Sync + Clone>() {}

        assert_impl::<UuidV4Generator<UuidFooId>>();
        assert_impl::<UuidV7Generator<UuidFooId>>();
    }
}
