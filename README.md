# ddd-toolkit-core

Traits and reference implementations for Domain-Driven Design building
blocks in Rust: entities, value objects, aggregates, repositories, and event
dispatch. No macro dependency - everything here can be implemented by hand.

If you want `#[derive(...)]` support for the value-object-shaped traits, add
[`ddd-toolkit-macro`](https://github.com/ryoshrimp/ddd-toolkit-macro)
alongside this crate, or depend on
[`ddd-toolkit`](https://github.com/ryoshrimp/ddd-toolkit) instead, which
bundles both behind a single dependency.

## What's here

- **`domain`** - `Entity`, `AggregateRoot`, `ValueObject`/`Wrapped`,
  `EntityId`, `SecretVo`, `EnumVo`, `DomainEvent`, `ValidationError`.
- **`port`** - `Load`/`Save`/`Delete` repository traits, `EventDispatcher`
  (with `DispatchError<E>` reporting undelivered events on partial
  failure), `Clock`, `IdGenerator`, `PortError`.
- **`adapter`** (behind the `chrono`/`uuid` features) - `SystemClock`,
  `UuidV4Generator`, `UuidV7Generator`.
- **`mock`** - `InMemoryStore` (a `Load`/`Save`/`Delete` implementation
  backed by a `HashMap`), `FixedClock`, `FixedIdGenerator`.
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

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
