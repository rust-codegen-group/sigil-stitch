#[test]
fn sigil_quote_compile_failures() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/sigil_quote/*.rs");
}
