use ddd_toolkit_core::{AggregateRoot, Entity};

struct NotAnAggregate;

fn requires_aggregate<A: AggregateRoot>() {}

fn main() {
    requires_aggregate::<NotAnAggregate>();
}

#[allow(dead_code)]
fn requires_typed_id<A: Entity>() {}
