use ddd_toolkit_core::Secret;

fn requires_serialize<T: serde::Serialize>() {}

fn main() {
    requires_serialize::<Secret<String>>();
}
