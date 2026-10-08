#[test]
fn sigil_quote_compile_failures() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/sigil_quote/*.rs");
}

#[test]
fn parametric_public_api_diagnostics() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/parametric/legacy_inputs.rs");
    cases.pass("tests/ui/parametric/modern_inputs.rs");
}
