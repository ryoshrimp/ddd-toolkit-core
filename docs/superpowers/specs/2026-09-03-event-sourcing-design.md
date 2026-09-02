# Event sourcing (Roadmap Phase D)

Adds the pieces needed to persist an aggregate as its event history
instead of its latest state: an `EventSourced` aggregate contract, an
`EventStore` port, an in-memory store, and a repository adapter that makes
an event store look like the existing `Load` / `Save` ports. Ships as
v0.7.0. Depends on Phase B (`Version`, `EventEnvelope`) and reuses the
`Projection` port from Phase C for read models.

## Goals

- Let an aggregate be rebuilt from its events (`rehydrate`) and evolve by
  applying them (`apply`), with the state-mutation logic in one place.
- Persist and load event streams with stream-version optimistic locking.
- Plug an event-sourced aggregate into code written against `Load<A>` /
  `Save<A>` without that code changing.

## Non-goals

- Snapshots. A stream is always replayed from the start; snapshotting can
  be added as a separate `SnapshotStore` port later without changing the
  traits here.
- Event schema evolution / upcasting. Events are stored as `A::Event`
  values; serialization and versioning of the wire format belong to a
  real adapter, not this crate.
- Global (cross-aggregate) ordering or subscriptions. `EventStore` is
  per-stream; fan-out is `EventDispatcher` / `Projection` wiring.

## Design

### `domain::EventSourced`

```rust
pub trait EventSourced: AggregateRoot {
    /// Mutates state to reflect `event`. Must not record events, must not
    /// fail: an event in the stream already happened.
    fn apply(&mut self, event: &Self::Event);

    /// Builds the aggregate from its first event; `None` if `event` cannot
    /// start a stream (e.g. it is not a "created" event).
    fn from_first_event(id: Self::Id, event: &Self::Event) -> Option<Self>
    where
        Self: Sized;

    /// Replays a full stream in order. Provided: calls `from_first_event`
    /// on the first envelope, `apply` on the rest, and advances the
    /// version once per envelope so `version()` ends equal to the
    /// stream length.
    fn rehydrate(id: Self::Id, stream: &[EventEnvelope<Self>]) -> Option<Self>
    where
        Self: Sized;
}
```

- `apply` takes `&Self::Event` so the same event can be both applied to
  the aggregate and kept for dispatch.
- Command methods on the aggregate are expected to construct the event,
  call `self.apply(&event)` then `self.record(event)`; `apply` is the
  single source of truth for state transitions. Documented, not
  enforced.
- `rehydrate` returns `None` for an empty stream or a stream whose first
  event cannot create the aggregate; the repository adapter maps `None`
  to "not found".

### `port::event_store::EventStore`

```rust
#[trait_variant::make(Send)]
pub trait EventStore<A: EventSourced>: Send + Sync {
    /// Returns the stream for `id` in order; empty if none.
    async fn load_stream(&self, id: &A::Id) -> Result<Vec<EventEnvelope<A>>, PortError>;

    /// Appends `envelopes` to `id`'s stream iff the stream's current
    /// version equals `expected`; otherwise returns
    /// `PortErrorKind::Conflict` and appends nothing. Empty `envelopes`
    /// is a successful no-op that still performs the version check.
    async fn append(
        &self,
        id: &A::Id,
        expected: Version,
        envelopes: Vec<EventEnvelope<A>>,
    ) -> Result<(), PortError>;
}
```

- "Stream version" is the number of envelopes in the stream
  (`Version::INITIAL` when empty). This matches what `rehydrate`
  produces, so `aggregate.version()` is the `expected` argument.
- Versioning therefore differs from state-based `Save` (Phase B), where
  one save is one version step: for an event-sourced aggregate one
  *event* is one step. The `version` stamped on each envelope by
  `EventEnvelope::wrap` is still the aggregate version at the start of
  the batch (all envelopes of one save share it); the stream position is
  the envelope's index. Both facts are documented on `EventStore`.
- `append` is atomic per call; partial writes are not allowed.

### `mock::event_store::InMemoryEventStore<A>`

- `HashMap<A::Id, Vec<EventEnvelope<A>>>` behind a `Mutex`.
- Implements `EventStore<A>` with the conflict check above.
- `pub fn stream_len(&self, id) -> usize` for tests.

### `adapter::repository::EventSourcedRepository<A, S>`

```rust
pub struct EventSourcedRepository<A: EventSourced, S: EventStore<A>> { store: S, .. }

impl<A, S> Load<A> for EventSourcedRepository<A, S>   // load_stream + rehydrate
impl<A, S> Save<A> for EventSourcedRepository<A, S>   // wrap + append + advance
```

- `Load::load` returns `Ok(None)` when `rehydrate` returns `None`.
- `Save::save`: `EventEnvelope::wrap(aggregate, now)` → `append(id,
  aggregate.version(), envelopes)` → on `Ok`, `aggregate.advance_version()`
  once per appended envelope, so `version()` again equals the stream
  length. On `Conflict`, the drained events are pushed back via `record`
  so the caller can reload and retry without losing them.
- `now` comes from a `Clock` if the `chrono` feature is on, otherwise
  `SystemTime::now()`; the constructor takes an `Fn() -> SystemTime` to
  avoid a feature-gated API.
- `Delete` is not implemented: deleting a stream is an anti-pattern in
  event sourcing; model deletion as an event.
- Lives in `adapter` because it is a real implementation, not a test
  double, even though it has no external dependency.

## Files

| File | Change |
|---|---|
| `src/domain/event_sourced.rs` | new: `EventSourced` with provided `rehydrate` |
| `src/port/event_store.rs` | new: `EventStore` |
| `src/mock/event_store.rs` | new: `InMemoryEventStore` |
| `src/adapter/repository.rs` | new: `EventSourcedRepository` |
| corresponding `mod.rs` files | wire + re-export |
| `src/lib.rs`, `README.md` | mention event sourcing |
| `CHANGELOG.md` | Added entries |

## Testing

- `EventSourced::rehydrate`: empty stream → `None`; non-creating first
  event → `None`; full stream reproduces the state a live aggregate
  reaches by the same commands; resulting `version()` equals the stream
  length.
- `InMemoryEventStore`: append then load round trip; append with wrong
  `expected` → `Conflict` and stream unchanged; empty append with correct
  version → `Ok`, with wrong version → `Conflict`; streams for different
  ids are independent.
- `EventSourcedRepository`: save new aggregate, load returns equal state
  and advanced version; second save with stale copy → `Conflict` and the
  stale copy still holds its events; a `Load` / `Save` generic function
  from `port::repository` tests accepts it unchanged.
- Doctest: define a small event-sourced `Account` (opened / deposited),
  run two commands, save, reload, assert balance and version.
