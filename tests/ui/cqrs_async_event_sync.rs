use std::cell::Cell;

fn requires_sync<T: Sync>() {}

fn main() {
    requires_sync::<Cell<u64>>();
}
