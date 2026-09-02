# CQRS: command / query markers and projections (Roadmap Phase C)

Separates the write side from the read side that `application::UseCase`
currently conflates, and gives read models a way to be fed from domain
events. Ships as v0.6.0. Depends on Phase B (`EventEnvelope`).

## Goals

- Let a use case declare, in its type, whether it mutates state
  (`Command`) or only reads (`Query`), without changing how `UseCase`
  itself works.
- Provide a `Projection` port: something that consumes event envelopes to
  maintain a read-side view.
- Keep the whole phase additive: nothing from Phase B changes.

## Non-goals

- A message bus, subscription registry or handler routing. How envelopes
  get from a `Save` to a `Projection` is application wiring (call it
  directly, or put an `EventDispatcher` in between).
- A separate read-side repository trait. A `Projection` owns its storage;
  how it is queried is up to the `Query` use case that reads it.
- Physically separate read/write data stores. This crate provides the
  seams only.

## Design

### `application::Command` and `application::Query`

```rust
/// A `UseCase` that changes state. Its `Output` is typically the id of the
/// aggregate touched, or `()`.
pub trait Command: UseCase {}

/// A `UseCase` that reads state and has no side effects on the domain.
pub trait Query: UseCase {}
```

- Marker traits over `UseCase`; a use case implements `UseCase` and then
  one of the markers. Existing `UseCase` implementors are unaffected.
- The markers exist so wiring code can require them (`C: Command`) and so
  a reviewer can tell intent from the signature. Nothing is enforced at
  the type level beyond the marker; the docs say so.
- Not both on one type: the docs state that a use case implementing both
  is a design smell, but the crate cannot forbid it.

### `port::projection::Projection`

```rust
#[trait_variant::make(Send)]
pub trait Projection<A: AggregateRoot>: Send + Sync {
    /// Applies one envelope to the read model. Must be idempotent for a
    /// given (aggregate_id, version): replaying an envelope already seen
    /// is a no-op, not an error.
    async fn project(&self, envelope: &EventEnvelope<A>) -> Result<(), PortError>;
}
```

- Generic over the aggregate so `A::Event` and `A::Id` are known;
  matches the `Load<A>` / `Save<A>` shape.
- Takes one envelope, not a batch: a projection that wants batching wraps
  the loop itself, and one-at-a-time makes the idempotency contract
  simple to state and test.
- Idempotency on `(aggregate_id, version)` is the contract because an
  `EventDispatcher` may redeliver after a partial failure
  (`DispatchError::undelivered`).
- Errors are `PortError`; `Unavailable` means "retry this envelope",
  anything else is left to the caller.

### `mock::projection::InMemoryProjection<A, V>`

```rust
pub struct InMemoryProjection<A: AggregateRoot, V> { ... }

impl<A, V> InMemoryProjection<A, V> {
    pub fn new(apply: impl Fn(Option<V>, &A::Event) -> V + Send + Sync + 'static) -> Self;
    pub fn get(&self, id: &A::Id) -> Option<V>;   // requires V: Clone
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
}
```

- Keeps `HashMap<A::Id, (Version, V)>`. `project` skips an envelope whose
  `version` is `<=` the stored one for that id (the idempotency
  reference behaviour), otherwise folds `apply` over the current view.
- `apply` receives `Option<V>` so the first event for an id builds the
  view from scratch.
- This is a test double and a template, like `InMemoryStore`.

## Files

| File | Change |
|---|---|
| `src/application/command.rs` | new: `Command` |
| `src/application/query.rs` | new: `Query` |
| `src/application/mod.rs` | wire + re-export |
| `src/port/projection.rs` | new: `Projection` |
| `src/port/mod.rs` | `pub mod projection;` |
| `src/mock/projection.rs` | new: `InMemoryProjection` |
| `src/mock/mod.rs` | wire |
| `src/lib.rs`, `README.md` | mention CQRS pieces |
| `CHANGELOG.md` | Added entries |

## Testing

- `Command` / `Query`: a `UseCase` can implement either marker; a generic
  `fn run<C: Command>` accepts it; a type with only `UseCase` is rejected
  by a `C: Command` bound (compile-fail test via `#[cfg(doctest)]`
  `compile_fail` doc block, matching how the crate already documents
  contracts).
- `Projection`: usable through a generic bound; future is `Send`;
  `PortError` propagates.
- `InMemoryProjection`: first envelope builds the view; later versions
  fold onto it; replaying the same `(id, version)` leaves the view
  unchanged; older version after newer is ignored; different ids are kept
  apart; `get` on an unknown id is `None`.
- End-to-end doctest: save to `InMemoryStore`, feed
  `store.published()` into an `InMemoryProjection`, read the view back
  through a `Query`.
