use ddd_toolkit_core::{
    And, DomainFactory, DomainService, EventIdentity, EventUpcaster, Not, Or, Policy,
    ProcessManager, Specification, Transition, UnitOfWork, UpcasterChain, VersionedEvent,
};
use std::cell::Cell;

struct Spec<F>(F);

impl<T: ?Sized, F> Specification<T> for Spec<F>
where
    F: Fn(&T) -> Result<bool, &'static str>,
{
    type Error = &'static str;

    fn is_satisfied_by(&self, candidate: &T) -> Result<bool, Self::Error> {
        (self.0)(candidate)
    }
}

#[test]
fn specifications_short_circuit_and_compose() {
    let calls = Cell::new(0);
    let first = Spec(|_: &i32| {
        calls.set(calls.get() + 1);
        Ok(false)
    });
    let second = Spec(|_: &i32| {
        calls.set(calls.get() + 1);
        Ok(true)
    });

    assert!(!first.and(second).is_satisfied_by(&1).unwrap());
    assert_eq!(calls.get(), 1);

    let calls = Cell::new(0);
    let first = Spec(|_: &i32| {
        calls.set(calls.get() + 1);
        Ok(true)
    });
    let second = Spec(|_: &i32| {
        calls.set(calls.get() + 1);
        Ok(false)
    });
    assert!(!first.or(second).not().is_satisfied_by(&1).unwrap());
    assert_eq!(calls.get(), 1);
}

#[test]
fn specification_constructors_evaluate_right_operands_and_preserve_errors() {
    let results = [Ok(false), Ok(true), Err("right failed")];
    for result in results {
        assert_eq!(
            And::new(Spec(|_: &i32| Ok(true)), Spec(|_: &i32| result)).is_satisfied_by(&1),
            result
        );
        assert_eq!(
            Or::new(Spec(|_: &i32| Ok(false)), Spec(|_: &i32| result)).is_satisfied_by(&1),
            result
        );
        assert_eq!(
            Not::new(Spec(|_: &i32| result)).is_satisfied_by(&1),
            result.map(|value| !value)
        );
    }

    let right = Spec(|_: &i32| -> Result<bool, &'static str> {
        panic!("right rule must not run after a left error")
    });
    assert_eq!(
        And::new(Spec(|_: &i32| Err("left failed")), right).is_satisfied_by(&1),
        Err("left failed")
    );
    let right = Spec(|_: &i32| -> Result<bool, &'static str> {
        panic!("right rule must not run after a left error")
    });
    assert_eq!(
        Or::new(Spec(|_: &i32| Err("left failed")), right).is_satisfied_by(&1),
        Err("left failed")
    );
}

struct AddOne;
impl DomainService<i32> for AddOne {
    type Output = i32;
    type Error = &'static str;
    fn execute(&self, input: i32) -> Result<Self::Output, Self::Error> {
        Ok(input + 1)
    }
}

struct Positive;
impl Policy<i32> for Positive {
    type Error = &'static str;
    fn check(&self, context: &i32) -> Result<(), Self::Error> {
        (*context > 0).then_some(()).ok_or("not positive")
    }
}

struct MakeString;
impl DomainFactory<i32> for MakeString {
    type Product = String;
    type Error = &'static str;
    fn create(&self, input: i32) -> Result<Self::Product, Self::Error> {
        Ok(input.to_string())
    }
}

#[test]
fn service_policy_and_factory_preserve_typed_boundaries() {
    assert_eq!(AddOne.execute(1), Ok(2));
    assert_eq!(Positive.check(&1), Ok(()));
    assert_eq!(Positive.check(&0), Err("not positive"));
    assert_eq!(MakeString.create(7), Ok(String::from("7")));
}

struct Manager;
impl ProcessManager for Manager {
    type State = i32;
    type Event = i32;
    type Effect = i32;
    type Error = &'static str;

    fn handle(
        &self,
        state: &Self::State,
        event: &Self::Event,
    ) -> Result<Transition<Self::State, Self::Effect>, Self::Error> {
        Ok(Transition::new(state + event, vec![*event]))
    }
}

#[test]
fn process_manager_is_deterministic_and_reentrant() {
    let next = Manager.handle(&1, &2).unwrap();
    assert_eq!(next.state(), &3);
    assert_eq!(next.effects(), &[2]);
    let (state, effects) = next.into_parts();
    assert_eq!((state, effects), (3, vec![2]));
}

struct RecordingUow {
    changes: Vec<i32>,
    committed: bool,
    rolled_back: bool,
}
impl UnitOfWork for RecordingUow {
    type Change = i32;
    type Error = &'static str;
    fn enlist(&mut self, change: i32) -> Result<(), Self::Error> {
        self.changes.push(change);
        Ok(())
    }
    fn commit(&mut self) -> Result<(), Self::Error> {
        self.committed = true;
        Ok(())
    }
    fn rollback(&mut self) -> Result<(), Self::Error> {
        self.rolled_back = true;
        Ok(())
    }
}

#[test]
fn unit_of_work_exposes_explicit_lifecycle() {
    let mut uow = RecordingUow {
        changes: Vec::new(),
        committed: false,
        rolled_back: false,
    };
    uow.enlist(1).unwrap();
    uow.enlist(2).unwrap();
    uow.commit().unwrap();
    assert_eq!(uow.changes, vec![1, 2]);
    assert!(uow.committed);
    uow.rollback().unwrap();
    assert!(uow.rolled_back);
}

#[derive(Debug, PartialEq)]
struct Versioned {
    value: i32,
}
impl VersionedEvent for Versioned {
    fn identity() -> EventIdentity {
        EventIdentity {
            name: "Versioned",
            version: 2,
        }
    }
}
struct Add(i32);
impl EventUpcaster<Versioned> for Add {
    type Error = &'static str;
    fn upcast(&self, mut event: Versioned) -> Result<Versioned, Self::Error> {
        event.value += self.0;
        Ok(event)
    }
}

#[test]
fn version_identity_and_upcasters_are_ordered() {
    assert_eq!(
        Versioned::identity(),
        EventIdentity {
            name: "Versioned",
            version: 2
        }
    );
    let mut chain = UpcasterChain::<Versioned, Add>::default();
    assert!(chain.is_empty());
    assert_eq!(chain.len(), 0);
    assert_eq!(
        chain.apply(Versioned { value: 7 }),
        Ok(Versioned { value: 7 })
    );
    chain.push(Add(1));
    assert!(!chain.is_empty());
    assert_eq!(chain.len(), 1);
    chain.push(Add(2));
    assert_eq!(
        chain.apply(Versioned { value: 0 }).unwrap(),
        Versioned { value: 3 }
    );
}
