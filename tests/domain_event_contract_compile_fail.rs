#[test]
fn event_bound_contracts_reject_non_events() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/domain_event_*_event_bound.rs");
}
