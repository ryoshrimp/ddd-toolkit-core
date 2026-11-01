use std::cell::Cell;

fn requires_id_sync<I: Sync>() {}

fn main() {
    requires_id_sync::<Cell<u64>>();
}
