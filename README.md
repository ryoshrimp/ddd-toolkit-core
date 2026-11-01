# ddd-toolkit-core

English | [日本語](README.ja.md)

[![crates.io](https://img.shields.io/crates/v/ddd-toolkit-core.svg)](https://crates.io/crates/ddd-toolkit-core)
[![docs.rs](https://img.shields.io/docsrs/ddd-toolkit-core)](https://docs.rs/ddd-toolkit-core)
[![CI](https://github.com/ryoshrimp/ddd-toolkit-core/actions/workflows/ci.yml/badge.svg)](https://github.com/ryoshrimp/ddd-toolkit-core/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/ryoshrimp/ddd-toolkit-core/graph/badge.svg)](https://codecov.io/gh/ryoshrimp/ddd-toolkit-core)
[![MSRV](https://img.shields.io/crates/msrv/ddd-toolkit-core)](Cargo.toml)
[![license](https://img.shields.io/crates/l/ddd-toolkit-core.svg)](#license)

Core traits and helpers for Domain-Driven Design in Rust: entities, value
objects, aggregates, persistence, CQRS, and event-driven workflows.

The crate defines typed boundaries rather than a persistence framework. You
implement domain types and adapters, choose error types, and supply storage,
transactions, delivery policies, and an async runtime when needed. There is no
macro dependency. All public building blocks are re-exported directly from
`ddd_toolkit_core`.

## Installation

```sh
cargo add ddd-toolkit-core
```

Requires Rust **1.85 or newer** (edition 2024). There are no default features.
The only optional feature, `serde`, exposes serde internally for generated
implementations; it does not automatically serialize domain types or `Secret`.
The crate uses `zeroize` for secret-value cleanup and has no async-runtime
dependency.

## Building blocks

| Area | Public API |
| --- | --- |
| Domain model | `Entity`, `ValueObject`, `AggregateRoot`, `DomainEvent` |
| Persistence | `Repository`, `AsyncRepository` |
| Event publishing | `DomainEventPublisher`, `AsyncDomainEventPublisher`, `dispatch_events`, `dispatch_events_async` |
| Event description | `DomainEventMetadata`, `DomainEventConversion`, `DeliveryMetadata`, `DomainEventDeliveryMetadata` |
| CQRS | `CommandHandler`, `AggregateCreator`, `QueryHandler`, lifecycle contexts/callbacks, and their `Async` counterparts |
| Event sourcing | `EventStore`, `AsyncEventStore`, revision/history types, `AggregateReplay`, `replay_aggregate` |
| Snapshots | `AggregateSnapshot`, `SnapshotStore`, `AsyncSnapshotStore`, `replay_with_optional_snapshot` |
| Event consumption | `EventConsumer`, `Projection`, and their `Async` counterparts |
| Reliable projections | `ProjectionCheckpoint`, `ProjectionCheckpointStore`, `IdempotentProjection`, `ProjectionApplication`, and asynchronous ports |
| Transactional outbox | `TransactionalOutbox`, `AsyncTransactionalOutbox`, `OutboxClaim`, `OutboxMessageId`, and lifecycle contexts |
| Domain rules | `Specification`, `And`, `Or`, `Not`, `DomainService`, `Policy`, `DomainFactory` |
| Workflow and transactions | `ProcessManager`, `Transition`, `UnitOfWork` |
| Schema evolution | `EventIdentity`, `VersionedEvent`, `EventUpcaster`, `UpcasterChain` |
| Secrets | `Secret`, `SecretMetadata` |

Async ports return boxed `Send` futures. Their implementations and callers must
satisfy the relevant lifetime and thread-safety bounds; the crate does not
select an executor.

## Example: aggregate, repository, and event publishing

Identifiers are application-defined: `Entity::Id` has no required marker trait.
An aggregate records events using `record_event` and transfers its pending
events using `pull_events`. Persistence and publishing are separate operations.

This example supplies its own in-memory repository and publisher; neither is a
built-in adapter.

```rust
use ddd_toolkit_core::{
    AggregateRoot, DomainEvent, DomainEventPublisher, Entity, Repository,
    dispatch_events,
};
use std::{collections::HashMap, convert::Infallible};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct OrderId(String);

#[derive(Debug, Clone)]
struct OrderPlaced;

impl DomainEvent for OrderPlaced {}

#[derive(Clone)]
struct Order {
    id: OrderId,
    events: Vec<OrderPlaced>,
}

impl Entity for Order {
    type Id = OrderId;

    fn id(&self) -> &Self::Id {
        &self.id
    }
}

impl AggregateRoot for Order {
    type Event = OrderPlaced;

    fn record_event(&mut self, event: Self::Event) {
        self.events.push(event);
    }

    fn pull_events(&mut self) -> Vec<Self::Event> {
        std::mem::take(&mut self.events)
    }
}

#[derive(Default)]
struct OrderRepository(HashMap<OrderId, Order>);

impl Repository for OrderRepository {
    type Aggregate = Order;
    type Error = Infallible;

    fn find_by_id(&self, id: &OrderId) -> Result<Option<Order>, Self::Error> {
        Ok(self.0.get(id).cloned())
    }

    fn save(&mut self, aggregate: &Order) -> Result<(), Self::Error> {
        // Persist domain state, not the pending delivery queue.
        self.0.insert(aggregate.id.clone(), Order {
            id: aggregate.id.clone(),
            events: Vec::new(),
        });
        Ok(())
    }

    fn delete(&mut self, id: &OrderId) -> Result<(), Self::Error> {
        self.0.remove(id);
        Ok(())
    }

    fn exists(&self, id: &OrderId) -> Result<bool, Self::Error> {
        Ok(self.0.contains_key(id))
    }
}

#[derive(Default)]
struct Publisher(Vec<OrderPlaced>);

impl DomainEventPublisher for Publisher {
    type Event = OrderPlaced;
    type Error = Infallible;

    fn publish(&mut self, event: &Self::Event) -> Result<(), Self::Error> {
        self.0.push(event.clone());
        Ok(())
    }
}

fn main() -> Result<(), Infallible> {
    let mut repository = OrderRepository::default();
    let mut publisher = Publisher::default();
    let mut order = Order {
        id: OrderId("order-1".into()),
        events: Vec::new(),
    };
    order.record_event(OrderPlaced);

    repository.save(&order)?; // Does not drain the aggregate's events.
    dispatch_events(&mut order, &mut publisher)?;

    assert!(order.pull_events().is_empty());
    assert_eq!(publisher.0.len(), 1);
    let loaded = repository.find_by_id(order.id())?.expect("saved order");
    assert_eq!(loaded.id(), order.id());
    Ok(())
}
```

`dispatch_events` publishes in the order returned by `pull_events`. On a
publishing error, it restores **all** pulled events, including those already
accepted by the publisher, and returns the publisher's error unchanged. Retrying
can therefore produce duplicates. `dispatch_events_async` provides the async
counterpart and restores pulled events if its in-progress future is dropped.
Neither helper makes persistence and publication atomic.

## Value objects and secrets

`ValueObject` exposes a wrapped value; validation and equality semantics belong
to the domain type's implementation.

```rust
use ddd_toolkit_core::ValueObject;

#[derive(Debug, Clone, PartialEq, Eq)]
struct CustomerName(String);

impl ValueObject for CustomerName {
    type Inner = String;

    fn as_inner(&self) -> &Self::Inner {
        &self.0
    }

    fn into_inner(self) -> Self::Inner {
        self.0
    }
}

let name = CustomerName("Alice".into());
assert_eq!(name.as_inner(), "Alice");
assert_eq!(name.into_inner(), "Alice");
```

`Secret<T>` requires `T: zeroize::Zeroize`, redacts `Debug` and `Display`, and
zeroizes the wrapped value on drop. Access to plaintext is explicit:

```rust
use ddd_toolkit_core::Secret;

let token = Secret::new(String::from("example-token"));
assert_eq!(format!("{token}"), "[REDACTED]");
assert!(token.expose_secret(|value| value.starts_with("example-")));
```

`as_inner` also exposes plaintext. Calling `into_inner` transfers responsibility
for zeroization to the caller. `SecretMetadata` is only a zero-sized marker; it
does not protect a value. `Secret` does not implement serde serialization or
implicit dereferencing.

## Workflow and reliability boundaries

- **CQRS:** implement command handlers and aggregate creators around typed
  repositories, publishers, and lifecycle callbacks. Lifecycle contexts track
  save/dispatch attempts and detect missing or unsuccessful operations; they
  are not transaction coordinators. Query handlers use shared access.
- **Event sourcing:** event stores load complete validated `StreamHistory`
  values and append using `ExpectedRevision`. Replay hooks reconstruct domain
  state; snapshot stores persist snapshot state separately. Storage adapters
  enforce concurrency and persistence guarantees.
- **Outbox:** adapters implement atomic save-and-enqueue, claiming, and
  acknowledgement. Lifecycle contexts track a single enqueue attempt, consume
  pending events, and do not restore them on failure. The async context consumes
  the attempt and pulls events before polling; dropping the future does not
  restore them. The context itself does not implement a transaction.
- **Projections:** checkpoint stores and idempotent projections expose progress
  and applied/duplicate outcomes. Implementations provide duplicate detection
  and coordinate read-model changes with checkpoint persistence.
- **Other workflows:** specifications compose fallible predicates with
  short-circuiting logic. Process managers return state/effect descriptions for
  the caller to persist and execute. Unit-of-work implementations define commit
  and rollback guarantees. Upcaster chains apply stages in insertion order,
  without automatically selecting schema versions.

Dropping an async port's future does not generally guarantee rollback or the
absence of side effects. Define cancellation, retries, and delivery guarantees
in your adapters and application.

Run `cargo doc --all-features --open` for API contracts. See the
[integration tests](tests) for concrete implementations, async examples, and
compile-time boundary checks.

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for release history.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
