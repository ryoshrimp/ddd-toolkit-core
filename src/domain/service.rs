/// A stateless piece of domain logic that does not naturally belong to any
/// single [`Entity`](crate::domain::Entity) or
/// [`ValueObject`](crate::domain::ValueObject) - typically a rule that spans
/// two or more aggregates.
///
/// This is a marker trait: domain services share no method signature. A
/// funds transfer, a pricing rule and a uniqueness check have nothing in
/// common except that they are stateless and coordinate across aggregates.
/// The `Send + Sync` bound is the same minimal one as
/// [`Entity`](crate::domain::Entity), so a service can be held by a use
/// case and shared across tasks.
///
/// # Keep logic in the entities
///
/// Reach for a domain service only when the behaviour genuinely does not
/// fit one entity or value object. Moving every rule into services drains
/// the entities of behaviour and leaves an anaemic model: bags of getters
/// and setters with the actual domain knowledge scattered across
/// "manager" types. If a rule reads or changes the state of exactly one
/// aggregate, it belongs on that aggregate.
///
/// A domain service is also not an application service: it applies a rule
/// to objects it is handed. Loading those objects, saving them afterwards
/// and publishing events is the calling use case's job.
///
/// # Examples
///
/// ```
/// use ddd_toolkit_core::domain::{DomainService, Entity, EntityId, ValueObject};
/// use std::fmt::Display;
///
/// #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
/// struct AccountId(u32);
///
/// impl ValueObject for AccountId {}
///
/// impl Display for AccountId {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         write!(f, "account-{}", self.0)
///     }
/// }
///
/// impl EntityId for AccountId {}
///
/// #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
/// struct Money(u64);
///
/// impl ValueObject for Money {}
///
/// struct Account {
///     id: AccountId,
///     balance: Money,
/// }
///
/// impl Entity for Account {
///     type Id = AccountId;
///
///     fn id(&self) -> &Self::Id {
///         &self.id
///     }
/// }
///
/// #[derive(Debug, PartialEq)]
/// enum TransferError {
///     InsufficientFunds,
/// }
///
/// impl Display for TransferError {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         write!(f, "insufficient funds")
///     }
/// }
///
/// impl std::error::Error for TransferError {}
///
/// // Stateless: the rule spans two `Account` aggregates, so it lives here
/// // rather than on either account.
/// struct TransferFunds;
///
/// impl DomainService for TransferFunds {}
///
/// impl TransferFunds {
///     fn transfer(
///         &self,
///         from: &mut Account,
///         to: &mut Account,
///         amount: Money,
///     ) -> Result<(), TransferError> {
///         if from.balance < amount {
///             return Err(TransferError::InsufficientFunds);
///         }
///         from.balance = Money(from.balance.0 - amount.0);
///         to.balance = Money(to.balance.0 + amount.0);
///         Ok(())
///     }
/// }
///
/// // The calling use case would load both accounts, call the service, and
/// // then save both; the service only applies the rule.
/// let mut alice = Account { id: AccountId(1), balance: Money(100) };
/// let mut bob = Account { id: AccountId(2), balance: Money(0) };
///
/// TransferFunds.transfer(&mut alice, &mut bob, Money(30))?;
///
/// assert_eq!(alice.balance, Money(70));
/// assert_eq!(bob.balance, Money(30));
///
/// assert_eq!(
///     TransferFunds.transfer(&mut alice, &mut bob, Money(1_000)),
///     Err(TransferError::InsufficientFunds)
/// );
/// # Ok::<(), TransferError>(())
/// ```
pub trait DomainService: Send + Sync {}

#[cfg(test)]
mod test {
    use std::sync::Mutex;

    use super::*;

    struct FooService;

    impl DomainService for FooService {}

    // The trait does not forbid state - the docs discourage it. This must
    // still compile.
    struct StatefulFooService {
        calls: Mutex<usize>,
    }

    impl DomainService for StatefulFooService {}

    #[test]
    fn unit_struct_impl_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}

        assert_send_sync::<FooService>();
    }

    // Compile-time check: the `Send + Sync` supertraits are implied by the
    // `DomainService` bound alone, without naming them at the use site.
    #[test]
    fn domain_service_bound_implies_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        fn check<S: DomainService>() {
            assert_send_sync::<S>();
        }

        check::<FooService>();
    }

    #[test]
    fn domain_service_is_usable_through_generic_bound() {
        fn hold<S: DomainService>(service: &S) -> &S {
            service
        }

        let service = FooService;

        let held = hold(&service);

        assert!(std::ptr::eq(held, &service));
    }

    #[test]
    fn struct_with_mutex_field_implements_domain_service() {
        let service = StatefulFooService {
            calls: Mutex::new(0),
        };

        *service.calls.lock().expect("lock should not be poisoned") += 1;

        assert_eq!(
            *service.calls.lock().expect("lock should not be poisoned"),
            1
        );
    }

    #[test]
    fn box_dyn_domain_service_is_usable() {
        let service: Box<dyn DomainService> = Box::new(FooService);

        // Only the coercion matters; `DomainService` has no methods to call.
        let _ = &service;
    }
}
