# Domain Service and Factory Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `domain::DomainService` (marker trait) and `domain::Factory` (construction trait) to the crate, with doctests, unit tests, and docs, as roadmap Phase A (v0.4).

**Architecture:** Two new files under `src/domain/`, one trait each, following the existing "one trait per file, inline `mod test`, doctest on the trait, glob re-export from `domain/mod.rs`" convention. No existing trait changes. No mocks or adapters.

**Tech Stack:** Rust 2024 edition, MSRV 1.85, no new dependencies. Tests run with `cargo test --all-features`; CI also runs `cargo fmt --check`, `cargo clippy --all-features --all-targets -- -D warnings`, and `cargo doc`.

**Spec:** `docs/superpowers/specs/2026-09-03-domain-service-factory-design.md` (roadmap: `docs/ROADMAP.md`, Phase A)

## Global Constraints

- `rust-version = "1.85"`, `edition = "2024"` (from `Cargo.toml`). Do not use features newer than 1.85.
- No new dependencies. `Cargo.toml` is not touched by this plan (the version bump to 0.4.0 is a separate release step, see "After the plan").
- Follow existing file conventions: one trait per file, `/// # Examples` doctest on every public trait, `#[cfg(test)] mod test` at the bottom of the same file, test fixtures named `Foo*` (`FooId`, `FooError`, ...), `pub use <file>::*;` in `src/domain/mod.rs`.
- Trait signatures are fixed by the spec and must be copied exactly:
  - `pub trait DomainService: Send + Sync {}`
  - `pub trait Factory: Send + Sync { type Input: Send; type Output; type Error: std::error::Error + Send + Sync + 'static; fn create(&self, input: Self::Input) -> Result<Self::Output, Self::Error>; }`
- `Factory::create` is synchronous. No `async`, no `trait_variant`.
- Commit message style in this repo: imperative sentence, no `feat:` prefix (e.g. `Add design spec for DomainService and Factory`). Every commit ends with the trailer block given in each commit step.
- Every commit must leave `cargo fmt --check`, `cargo clippy --all-features --all-targets -- -D warnings`, and `cargo test --all-features` green.

---

## File structure

| File | Responsibility |
|---|---|
| `src/domain/service.rs` (new) | `DomainService` marker trait, its doctest, its unit tests |
| `src/domain/factory.rs` (new) | `Factory` trait, its doctest, its unit tests |
| `src/domain/mod.rs` (modify) | declare the two modules and re-export them |
| `src/lib.rs` (modify) | crate-level doc: list the two new traits under `domain` |
| `README.md` (modify) | "What's here" bullet for `domain` |
| `CHANGELOG.md` (modify) | `[Unreleased]` → `### Added` entries |

Task 1 and Task 2 are independent of each other (they touch different lines of `mod.rs`). Task 3 depends on both.

---

### Task 1: `DomainService` marker trait

**Files:**
- Create: `src/domain/service.rs`
- Modify: `src/domain/mod.rs` (add `mod service;` and `pub use service::*;`)
- Test: inline `mod test` in `src/domain/service.rs`

**Interfaces:**
- Consumes: nothing from other tasks.
- Produces: `ddd_toolkit_core::domain::DomainService` (`pub trait DomainService: Send + Sync {}`), used by Task 3's docs.

- [ ] **Step 1: Write the failing tests**

Create `src/domain/service.rs` with only the test module (no trait yet) so the first run fails on "cannot find trait `DomainService`":

```rust
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

        assert_eq!(*service.calls.lock().expect("lock should not be poisoned"), 1);
    }

    #[test]
    fn box_dyn_domain_service_is_usable() {
        let service: Box<dyn DomainService> = Box::new(FooService);

        // Only the coercion matters; `DomainService` has no methods to call.
        let _ = &service;
    }
}
```

- [ ] **Step 2: Register the module and run the tests to verify they fail**

Edit `src/domain/mod.rs`. Add `mod service;` to the module list (alphabetical, between `mod secret_vo;` and `mod vo;`) and `pub use service::*;` to the re-export list (between `pub use secret_vo::*;` and `pub use vo::*;`).

Run: `cargo test --all-features domain::service`
Expected: compile error, `cannot find trait `DomainService` in this scope`.

- [ ] **Step 3: Write the trait with its doctest**

Insert the following above the `#[cfg(test)]` block in `src/domain/service.rs`:

```rust
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
```

- [ ] **Step 4: Run the tests and doctest to verify they pass**

Run: `cargo test --all-features domain::service`
Expected: 5 unit tests pass, `src/domain/service.rs` doctest passes (`test result: ok`).

- [ ] **Step 5: Run the full check set**

Run:
```bash
cargo fmt
cargo clippy --all-features --all-targets -- -D warnings
cargo doc --all-features --no-deps
cargo test --all-features
```
Expected: no clippy warnings, no rustdoc warnings (in particular no broken intra-doc links for `crate::domain::Entity` / `crate::domain::ValueObject`), all tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/domain/service.rs src/domain/mod.rs
git commit -m "Add DomainService marker trait

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01SaeT9esBWg2PCnxk5So54o"
```

---

### Task 2: `Factory` trait

**Files:**
- Create: `src/domain/factory.rs`
- Modify: `src/domain/mod.rs` (add `mod factory;` and `pub use factory::*;`)
- Test: inline `mod test` in `src/domain/factory.rs`

**Interfaces:**
- Consumes: `crate::domain::ValidationError` (existing, `ValidationError::new(type_name: &'static str, reason: impl Into<String>)`), `crate::mock::id::FixedIdGenerator<Id>(pub Id)` and `crate::port::id::IdGenerator<Id>::generate(&self) -> Id` (existing).
- Produces: `ddd_toolkit_core::domain::Factory` with associated types `Input`, `Output`, `Error` and `fn create(&self, input: Self::Input) -> Result<Self::Output, Self::Error>`, used by Task 3's docs.

- [ ] **Step 1: Write the failing tests**

Create `src/domain/factory.rs` with only the test module:

```rust
#[cfg(test)]
mod test {
    use std::fmt::Display;

    use crate::{
        domain::{Entity, EntityId, ValidationError, ValueObject},
        mock::id::FixedIdGenerator,
        port::id::IdGenerator,
    };

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

        let first = factory.create("a".to_string()).expect("first create should succeed");
        let second = factory.create("b".to_string()).expect("second create should succeed");

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

        let foo = build(&foo_factory(), "generic".to_string()).expect("generic create should succeed");

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
```

- [ ] **Step 2: Register the module and run the tests to verify they fail**

Edit `src/domain/mod.rs`. Add `mod factory;` to the module list (alphabetical, between `mod event;` and `mod secret_vo;`; if Task 1 already ran, the list also contains `mod service;`) and `pub use factory::*;` to the re-export list (between `pub use event::*;` and `pub use secret_vo::*;`).

Run: `cargo test --all-features domain::factory`
Expected: compile error, `cannot find trait `Factory` in this scope`.

- [ ] **Step 3: Write the trait with its doctest**

Insert the following above the `#[cfg(test)]` block in `src/domain/factory.rs`:

```rust
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
```


- [ ] **Step 4: Run the tests and doctest to verify they pass**

Run: `cargo test --all-features domain::factory`
Expected: 8 unit tests pass, `src/domain/factory.rs` doctest passes.

- [ ] **Step 5: Run the full check set**

Run:
```bash
cargo fmt
cargo clippy --all-features --all-targets -- -D warnings
cargo doc --all-features --no-deps
cargo test --all-features
```
Expected: no clippy warnings, no rustdoc warnings (check the intra-doc links to `crate::application::usecase::UseCase`, `crate::port::id::IdGenerator`, `crate::port::clock::Clock`, `crate::mock::id::FixedIdGenerator` resolve), all tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/domain/factory.rs src/domain/mod.rs
git commit -m "Add Factory trait

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01SaeT9esBWg2PCnxk5So54o"
```

---

### Task 3: Crate docs, README, CHANGELOG

**Files:**
- Modify: `src/lib.rs:5-8` (the `domain` bullet of the crate-level doc)
- Modify: `README.md:47-48` (the `domain` bullet under "What's here")
- Modify: `CHANGELOG.md:9` (the `## [Unreleased]` section)

**Interfaces:**
- Consumes: `domain::DomainService` (Task 1) and `domain::Factory` (Task 2) must exist so the intra-doc links in `src/lib.rs` resolve.
- Produces: nothing code-level.

- [ ] **Step 1: Update the crate-level doc in `src/lib.rs`**

Replace the `domain` bullet:

```rust
//! - [`domain`] - the domain-layer traits: [`domain::Entity`],
//!   [`domain::AggregateRoot`], [`domain::ValueObject`]/[`domain::Wrapped`],
//!   [`domain::EntityId`], [`domain::SecretVo`], [`domain::EnumVo`],
//!   [`domain::DomainEvent`].
```

with:

```rust
//! - [`domain`] - the domain-layer traits: [`domain::Entity`],
//!   [`domain::AggregateRoot`], [`domain::ValueObject`]/[`domain::Wrapped`],
//!   [`domain::EntityId`], [`domain::SecretVo`], [`domain::EnumVo`],
//!   [`domain::DomainEvent`], [`domain::DomainService`], [`domain::Factory`].
```

- [ ] **Step 2: Verify the doc links resolve**

Run: `RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps`
Expected: builds with no warnings. A typo in a link name fails this step with `unresolved link`.

- [ ] **Step 3: Update `README.md`**

Replace:

```markdown
- **`domain`** - `Entity`, `AggregateRoot`, `ValueObject`/`Wrapped`,
  `EntityId`, `SecretVo`, `EnumVo`, `DomainEvent`, `ValidationError`.
```

with:

```markdown
- **`domain`** - `Entity`, `AggregateRoot`, `ValueObject`/`Wrapped`,
  `EntityId`, `SecretVo`, `EnumVo`, `DomainEvent`, `DomainService`,
  `Factory`, `ValidationError`.
```

- [ ] **Step 4: Update `CHANGELOG.md`**

Directly under `## [Unreleased]` (currently an empty section) add:

```markdown
### Added

- `domain::DomainService` - marker trait (`Send + Sync`) for stateless
  logic that spans aggregates. Docs carry the anaemic-model warning: a
  rule that fits one entity or value object belongs there, not in a
  service.
- `domain::Factory` - trait for building an aggregate/entity/value object
  that is valid from the moment it exists (`Input`/`Output`/`Error`
  associated types, synchronous `create`). Pairs with `ValidationError`;
  no mock or adapter, construction is user-specific.
```

- [ ] **Step 5: Run the full check set one last time**

Run:
```bash
cargo fmt --check
cargo clippy --all-features --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps
cargo test --all-features
cargo test --no-default-features
```
Expected: all green. The `--no-default-features` run mirrors the CI feature matrix and proves the new files do not depend on any optional feature.

- [ ] **Step 6: Commit**

```bash
git add src/lib.rs README.md CHANGELOG.md
git commit -m "Document DomainService and Factory in crate docs, README and changelog

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01SaeT9esBWg2PCnxk5So54o"
```

---

## After the plan

- Open a PR against `main` (CI must pass; `main` is branch-protected).
- The version bump to `0.4.0` and the `## [0.4.0] - <date>` changelog heading are a separate release commit, following the existing `Release v0.3.0` pattern. They are not part of this plan.

## Self-review against the spec

- **Spec coverage:** `DomainService` trait + docs warning + `TransferFunds` doctest → Task 1. `Factory` trait with exact signature, `OrderFactory`/`FixedIdGenerator`/`ValidationError` doctest, port-as-field guidance → Task 2. `mod.rs` re-exports → Tasks 1 and 2. `lib.rs`, `README.md`, `CHANGELOG.md` → Task 3. Every test listed under the spec's "Testing" section maps to a named test function: `unit_struct_impl_is_send_and_sync`, `domain_service_is_usable_through_generic_bound`, `struct_with_mutex_field_implements_domain_service`, `create_returns_ok_on_valid_input`, `create_returns_implementors_error_on_invalid_input`, `factory_is_usable_through_generic_bound`, `error_can_be_boxed_as_dyn_error`, `factory_holding_fixed_id_generator_produces_the_fixed_id`. Non-goals (no async factory, no mocks, no existing-trait changes) are respected.
- **Placeholders:** none.
- **Type consistency:** `Factory::{Input, Output, Error, create}` names are identical in the trait, the doctest, the unit tests and the changelog. `DomainService` has no members.
