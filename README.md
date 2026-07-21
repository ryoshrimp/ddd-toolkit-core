# ddd-toolkit-core

[![crates.io](https://img.shields.io/crates/v/ddd-toolkit-core.svg)](https://crates.io/crates/ddd-toolkit-core)
[![docs.rs](https://img.shields.io/docsrs/ddd-toolkit-core)](https://docs.rs/ddd-toolkit-core)
[![CI](https://github.com/ryoshrimp/ddd-toolkit-core/actions/workflows/ci.yml/badge.svg)](https://github.com/ryoshrimp/ddd-toolkit-core/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/ryoshrimp/ddd-toolkit-core/graph/badge.svg)](https://codecov.io/gh/ryoshrimp/ddd-toolkit-core)
[![MSRV](https://img.shields.io/crates/msrv/ddd-toolkit-core)](Cargo.toml)
[![license](https://img.shields.io/crates/l/ddd-toolkit-core.svg)](#license)

Traits and reference implementations for Domain-Driven Design building
blocks in Rust: entities, value objects, aggregates, repositories, and event
dispatch. No macro dependency - everything here can be implemented by hand.

If you want `#[derive(...)]` support for the value-object-shaped traits, add
[`ddd-toolkit-macro`](https://github.com/ryoshrimp/ddd-toolkit-macro)
alongside this crate, or depend on
[`ddd-toolkit`](https://github.com/ryoshrimp/ddd-toolkit) instead, which
bundles both behind a single dependency. Reach for this crate directly when
you want the traits with zero macro machinery and full control over every
impl; reach for `ddd-toolkit` when you'd rather generate the boilerplate for
`ValueObject`/`EntityId`/etc.

## Installation

```sh
cargo add ddd-toolkit-core
```

Or add it to `Cargo.toml` directly, enabling the adapter features you need:

```toml
[dependencies]
ddd-toolkit-core = { version = "0.2", features = ["chrono", "uuid"] }
```

- `chrono` enables `adapter::clock::SystemClock` and `mock::clock::FixedClock`.
- `uuid` enables `adapter::id::UuidV4Generator`/`UuidV7Generator`.
- `serde` derives `Serialize` (not `Deserialize` - see `ValidationError`'s
  docs for why) for `domain::ValidationError`.

None of these features is required: the `domain`, `port`, `application`
modules and the rest of `mock` build with no default features. Requires
Rust 1.85 or newer (edition 2024).

## What's here

- **`domain`** - `Entity`, `AggregateRoot`, `ValueObject`/`Wrapped`,
  `EntityId`, `SecretVo`, `EnumVo`, `DomainEvent`, `ValidationError`.
- **`port`** - `Load`/`Save`/`Delete` repository traits, `EventDispatcher`
  (with `DispatchError<E>` reporting undelivered events on partial
  failure), `Clock`, `IdGenerator`, `PortError`.
- **`adapter`** - real port implementations, gated per feature so you only
  pull in what you use: `SystemClock` behind `chrono`,
  `UuidV4Generator`/`UuidV7Generator` behind `uuid`.
- **`mock`** - test doubles: `InMemoryStore` (a `Load`/`Save`/`Delete`
  implementation backed by a `HashMap`) and `FixedIdGenerator` build
  unconditionally; `FixedClock` is behind the `chrono` feature.
- **`application`** - `UseCase`.

## Example

```rust
use ddd_toolkit_core::domain::{Entity, EntityId, ValueObject};
use std::fmt::Display;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct OrderId(String);

impl ValueObject for OrderId {}

impl Display for OrderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl EntityId for OrderId {}

struct Order {
    id: OrderId,
}

impl Entity for Order {
    type Id = OrderId;

    fn id(&self) -> &Self::Id {
        &self.id
    }
}

fn main() {
    let a = Order { id: OrderId("order-1".to_string()) };
    let b = Order { id: OrderId("order-1".to_string()) };
    assert!(a.is_same_as(&b)); // identity, not structural equality
}
```

`Entity` is the smallest building block; a real aggregate also implements
`AggregateRoot` to record domain events, and is persisted through the
`Load`/`Save` ports - here via `mock::repository::InMemoryStore`, a drop-in
test double:

```rust
use ddd_toolkit_core::domain::{AggregateRoot, DomainEvent, Entity, EntityId, ValueObject};
use ddd_toolkit_core::mock::repository::InMemoryStore;
use ddd_toolkit_core::port::repository::{Load, Save};
use std::fmt::Display;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct OrderId(String);

impl ValueObject for OrderId {}

impl Display for OrderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl EntityId for OrderId {}

#[derive(Debug, Clone, PartialEq)]
struct OrderPlaced;

impl DomainEvent for OrderPlaced {}

#[derive(Debug, Clone)]
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

    fn record(&mut self, event: Self::Event) {
        self.events.push(event);
    }

    fn take_events(&mut self) -> Vec<Self::Event> {
        std::mem::take(&mut self.events)
    }
}

// This crate has no async-runtime dependency of its own; `tokio` here is
// just an example caller driving the async `Load`/`Save` ports.
#[tokio::main]
async fn main() -> Result<(), ddd_toolkit_core::port::PortError> {
    let store = InMemoryStore::new();
    let mut order = Order { id: OrderId("order-1".to_string()), events: vec![OrderPlaced] };

    store.save(&mut order).await?; // drains the aggregate's recorded events as a side effect
    assert!(order.take_events().is_empty());

    let loaded = store
        .load(&OrderId("order-1".to_string()))
        .await?
        .expect("just-saved order should be found");
    assert_eq!(loaded.id, OrderId("order-1".to_string()));
    Ok(())
}
```

More usage, including `EventDispatcher` and the `chrono`/`uuid` adapters, is
covered in the [API docs](https://docs.rs/ddd-toolkit-core).

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for release history.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
