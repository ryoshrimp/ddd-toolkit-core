#[test]
fn secret_does_not_deref() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/secret_no_deref.rs");
}
