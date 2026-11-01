#[test]
fn amended_sync_callback_lifetime_allows_completion() {
    let tests = trybuild::TestCases::new();
    tests.pass("tests/ui/pass_cqrs_sync_callback_lifetime.rs");
}
