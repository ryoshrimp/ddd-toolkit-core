use ddd_toolkit_core::Secret;

fn main() {
    let secret = Secret::new(String::from("secret"));
    let _: &str = &secret;
}
