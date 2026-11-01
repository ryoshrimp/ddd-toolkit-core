#[cfg(feature = "serde")]
#[test]
fn secret_does_not_implement_serde() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/secret_serde.rs");
}
