# Domain service and factory (Roadmap Phase A)

Adds the two remaining classic tactical DDD building blocks to the `domain`
module: a `DomainService` marker trait and a `Factory` trait. Ships as
v0.4.0. See `docs/ROADMAP.md` for how this fits the later phases.

## Goals

- Give stateless cross-aggregate logic a named home (`DomainService`) so
  it is distinguishable from application-layer orchestration.
- Give "build an aggregate/entity that is valid from the moment it exists"
  a trait (`Factory`) so callers depend on the abstraction, not a concrete
  constructor.
- Follow the existing conventions exactly: one trait per file, inline
  `mod test`, doctest on the trait, re-export from `domain/mod.rs`.

## Non-goals

- An async factory. Factories are pure domain construction; if a real
  backend must be consulted, that belongs in a use case that then calls
  the factory. An `AsyncFactory` can be added later without breaking this.
- Mock or adapter implementations of `Factory`. Construction logic is
  user-specific; the crate only provides the trait.
- Any change to existing traits.

## Design

### `domain::DomainService`

```rust
pub trait DomainService: Send + Sync {}
```

- Marker only. Domain services have no common method signature - a
  transfer, a pricing rule and a uniqueness check share nothing but
  "stateless, spans aggregates".
- Same minimal bound as `Entity`, so a service can be held by a use case
  and shared across tasks.
- Docs must carry the anaemic-model warning from the notes: logic that
  fits one entity or value object goes there, not in a service.
- Doc example: `TransferFunds` holding no state, with a `transfer(&self,
  from: &mut Account, to: &mut Account, amount: Money) -> Result<(),
  TransferError>` method. Loading/saving the two accounts is the calling
  use case's job; the service only applies the rule.

### `domain::Factory`

```rust
pub trait Factory: Send + Sync {
    /// What the factory needs to build one instance.
    type Input: Send;
    /// What it builds.
    type Output;
    /// Why construction can be refused.
    type Error: std::error::Error + Send + Sync + 'static;

    fn create(&self, input: Self::Input) -> Result<Self::Output, Self::Error>;
}
```

- Associated types mirror `application::usecase::UseCase` so the two read
  the same way; `create` is synchronous (see non-goals).
- `Output` is deliberately unbounded: factories build aggregates, entities
  and complex value objects alike.
- `Error` is not fixed to `ValidationError`; that type is the expected
  common choice and the doc example uses it.
- A factory that needs ids or timestamps owns the port as a field
  (`IdGenerator` / `Clock` are already synchronous) and calls it inside
  `create`.
- Doc example: `OrderFactory { ids: FixedIdGenerator<OrderId> }` taking
  `Vec<LineItem>`; empty input returns
  `ValidationError::new("Order", "must contain at least one line item")`,
  otherwise an `Order` with a generated id.

## Files

| File | Change |
|---|---|
| `src/domain/service.rs` | new: `DomainService` + tests |
| `src/domain/factory.rs` | new: `Factory` + tests |
| `src/domain/mod.rs` | `mod service; mod factory;` + `pub use` |
| `src/lib.rs` | add both to the crate-level module list |
| `README.md` | add both to "What's here" |
| `CHANGELOG.md` | `[Unreleased]` → Added entries |

## Testing

Inline `mod test` in each file, matching existing style:

- `DomainService`: a unit-struct impl satisfies `Send + Sync`; usable
  through a generic `S: DomainService` bound; a struct with a `Mutex`
  field still compiles (the trait does not forbid state, docs discourage
  it).
- `Factory`: `create` returns `Ok` on valid input; returns the
  implementor's `Error` on invalid input; usable through a generic
  `F: Factory` bound; `F::Error` upcasts to
  `Box<dyn std::error::Error + Send + Sync>`; a factory holding a
  `FixedIdGenerator` produces the fixed id.

Doctests on both traits are the worked examples above. CI runs
`cargo test --all-features` and `cargo doc`; both must pass.
