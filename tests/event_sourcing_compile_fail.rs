#[test]
fn event_sourcing_contracts_reject_invalid_type_relationships() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/event_sourcing_*.rs");
}
