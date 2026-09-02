# Roadmap

Tactical DDD building blocks still missing from this crate, in the order
they will be added. Each phase ships as its own minor release and gets its
own design spec under `docs/superpowers/specs/` before implementation.

The gap analysis is based on the DDD reference notes (entity, value object,
aggregate, repository, domain event, domain service, factory, CQRS / event
sourcing) compared against `src/` as of v0.3.0.

## Current state (v0.3.0)

| Concept | Status |
|---|---|
| Entity / Value Object / EntityId | done (`Entity`, `ValueObject`, `Wrapped`, `EnumVo`, `SecretVo`) |
| Aggregate root | done, but only event recording/draining |
| Repository | done (`Load` / `Save` / `Delete`, `InMemoryStore`) |
| Domain event | marker trait only; no metadata contract |
| Domain service | missing |
| Factory | missing |
| CQRS | missing (`UseCase` only) |
| Event sourcing | missing |
| Optimistic concurrency | `PortErrorKind::Conflict` exists; no version on the aggregate |

Specs:

- Phase A: `superpowers/specs/2026-09-03-domain-service-factory-design.md`
- Phase B: `superpowers/specs/2026-09-03-event-envelope-versioning-design.md`
- Phase C: `superpowers/specs/2026-09-03-cqrs-design.md`
- Phase D: `superpowers/specs/2026-09-03-event-sourcing-design.md`

## Phase A - Domain service and factory (v0.4)

Fills the last two "classic" tactical patterns. Fewest dependencies,
follows the existing `domain` module pattern directly.

- `domain::DomainService` - marker trait for stateless, `Send + Sync`
  services that coordinate across aggregates. Documents the "don't drain
  logic out of entities" warning from the notes.
- `domain::Factory` - trait for building an aggregate/entity that is valid
  from the moment it exists (`Output`, `Error` associated types; input via
  a method argument). Pairs with `ValidationError`.
- Mocks/adapters: none needed.

## Phase B - Event metadata and optimistic concurrency (v0.5)

Makes the `DomainEvent` and `AggregateRoot` contracts match the notes:
events are published after commit, carry when/where they happened, and
`Save` can detect stale writes.

- `DomainEvent`: add `occurred_at` and `aggregate_id` accessors (event
  envelope or required methods - decided in the spec). `chrono` feature
  interplay to be resolved there.
- `AggregateRoot::version()` (or a `Versioned` trait) so `Save`
  implementations can return `Conflict`; `InMemoryStore` gains a version
  check as the reference implementation.
- Likely a breaking change to the `AggregateRoot` / `DomainEvent`
  surface; the spec must call out the migration.

## Phase C - CQRS (v0.6)

Separates the command and query sides that `UseCase` currently conflates.

- `application::Command` / `application::Query` traits (or `UseCase`
  markers), plus how they relate to the existing `UseCase`.
- `port::ReadModel` / `port::Projection` - consumes `DomainEvent`s to
  update a read-side view; in-memory mock.
- Depends on Phase B for event metadata a projection can key on.

## Phase D - Event sourcing (v0.7)

The largest phase; depends on B (versions, event metadata) and C
(projections consume the stream).

- `domain::EventSourced` - `apply(&mut self, event)` and
  `rehydrate(events) -> Self`.
- `port::EventStore` - append with expected version, load a stream; stream
  version is the optimistic-lock unit.
- `mock::InMemoryEventStore` reference implementation.
- An event-sourced `Load` / `Save` adapter that bridges `EventStore` to the
  existing repository ports.

## Out of scope

Strategic design (bounded contexts, context maps, subdomains, ubiquitous
language) is documentation/organisation, not code; it stays out of this
crate.
