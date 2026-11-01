#[test]
fn cqrs_contracts_reject_invalid_bounds_and_private_state_access() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/cqrs_*.rs");
}
