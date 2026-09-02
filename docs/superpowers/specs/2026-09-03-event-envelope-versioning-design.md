# Event envelope and optimistic concurrency (Roadmap Phase B)

Makes the `DomainEvent` and `AggregateRoot` contracts match the reference
notes: an event knows which aggregate it came from and when it happened,
and a `Save` implementation can detect a stale write. Ships as v0.5.0.
Depends on nothing; Phases C and D build on it.

## Goals

- Attach `aggregate_id`, `version` and `occurred_at` to recorded events
  without forcing every user event type to carry those fields.
- Give aggregates a version so `Save` can honour the `Conflict` contract
  already documented on `port::repository::Save`.
- Make `mock::InMemoryStore` the reference implementation of both.

## Non-goals

- Event schema versioning / upcasting (an open question in the notes; out
  of scope until Phase D needs it).
- Changing `EventDispatcher`. It stays generic over `E: DomainEvent`; the
  envelope implements `DomainEvent` so it can be dispatched as-is.
- A `chrono`-typed timestamp on the envelope. `SystemTime` is used so the
  envelope exists with no features enabled; `chrono::DateTime<Utc>:
  From<SystemTime>` already covers conversion.

## Design

### `domain::Version`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Version(u64);

impl Version {
    pub const INITIAL: Version = Version(0);
    pub fn next(self) -> Version;
    pub fn as_u64(self) -> u64;
}
impl ValueObject for Version {}
impl Display for Version {}
```

`INITIAL` is the version of an aggregate that has never been saved. Each
successful save advances it by one. `Version` never wraps; `next` panics
on `u64::MAX` (unreachable in practice, documented).

### `AggregateRoot` changes (breaking)

```rust
pub trait AggregateRoot: Entity {
    type Event: DomainEvent;
    fn record(&mut self, event: Self::Event);
    fn take_events(&mut self) -> Vec<Self::Event>;

    /// The version this instance was loaded at (`Version::INITIAL` if new).
    fn version(&self) -> Version;
    /// Called by `Save` after a successful write.
    fn advance_version(&mut self);
}
```

Both new methods are required (no defaults) so an implementor cannot
forget to store the version. The CHANGELOG entry shows the two-line
migration (add a `version: Version` field, implement both methods).

### `domain::EventEnvelope<A>`

```rust
#[derive(Debug, Clone)]
pub struct EventEnvelope<A: AggregateRoot> {
    pub aggregate_id: A::Id,
    /// The aggregate version this event was recorded *at* (before the
    /// save that persisted it advanced it).
    pub version: Version,
    pub occurred_at: std::time::SystemTime,
    pub event: A::Event,
}

impl<A: AggregateRoot> DomainEvent for EventEnvelope<A> {}

impl<A: AggregateRoot> EventEnvelope<A> {
    /// Drains `aggregate`'s recorded events and stamps each one with the
    /// aggregate's id, current version and `occurred_at`.
    pub fn wrap(aggregate: &mut A, occurred_at: SystemTime) -> Vec<Self>;
}
```

- `Clone` is derived and requires `A::Id: Clone` (already true via
  `EntityId`) and `A::Event: Clone`; the derive is conditional, so
  non-`Clone` events still work, they just cannot be cloned in an
  envelope.
- `wrap` takes `occurred_at` explicitly rather than calling
  `SystemTime::now()` so a `Save` implementation can inject a `Clock`
  (converted) or a fixed time in tests.
- All events drained in one `wrap` call share the same `version` and
  `occurred_at`: they belong to the same unit of work.

### `Save` contract update

`Save::save` doc is extended: implementations must

1. compare `aggregate.version()` against the stored version; on mismatch
   return `PortError::conflict(...)` without writing;
2. on success, persist state with `aggregate.version().next()`, call
   `aggregate.advance_version()`, and drain events (typically via
   `EventEnvelope::wrap`).

`Load` doc: the returned aggregate's `version()` must equal the stored
version.

### `mock::InMemoryStore` changes

- Stores `(A, Version)` per id; `save` performs the version check above
  and returns `Conflict` on mismatch (removing the "intentionally does not"
  caveat from the `Save` docs).
- Gains `pub fn published(&self) -> Vec<EventEnvelope<A>>` (requires
  `A::Event: Clone`) returning every envelope drained by successful saves,
  in order, so tests can assert on what would have been dispatched.
- `save` uses `SystemTime::now()` for `occurred_at`; a `with_clock`
  constructor is not added (tests assert on ids/versions, not time).

## Files

| File | Change |
|---|---|
| `src/domain/version.rs` | new: `Version` |
| `src/domain/envelope.rs` | new: `EventEnvelope`, `wrap` |
| `src/domain/aggregate.rs` | add `version` / `advance_version`; update doctest and tests |
| `src/domain/mod.rs` | wire new modules |
| `src/port/repository.rs` | extend `Save` / `Load` docs and test fixtures |
| `src/mock/repository.rs` | version check, `published()` |
| `src/lib.rs`, `README.md` | mention envelope + versioning |
| `CHANGELOG.md` | Changed (breaking) + Added, with migration note |

## Testing

- `Version`: `INITIAL` is 0; `next` increments; ordering; `Display`.
- `EventEnvelope::wrap`: empty aggregate → empty vec; stamps id, version
  and time; drains the aggregate; multiple events share one stamp;
  `EventEnvelope<A>` satisfies `DomainEvent` and is dispatchable through a
  `RecordingDispatcher<EventEnvelope<A>>`.
- `AggregateRoot`: fresh aggregate is `INITIAL`; `advance_version` moves
  it forward.
- `InMemoryStore`: save-load round trip returns advanced version; saving
  a stale copy returns `PortErrorKind::Conflict` and leaves stored state
  untouched; `published()` returns envelopes in save order; a failed
  (conflicting) save publishes nothing and does not drain the aggregate.
