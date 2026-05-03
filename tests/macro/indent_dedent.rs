use super::helpers::*;

#[test]
fn test_manual_indent_dedent() {
    let block = sigil_quote!(TypeScript {
        namespace Foo {$>
        const x = 1;
        const y = 2;
        $<}
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("namespace Foo {"), "got: {output}");
    assert!(
        output.contains("    const x = 1;"),
        "expected indented x, got: {output}"
    );
    assert!(
        output.contains("    const y = 2;"),
        "expected indented y, got: {output}"
    );
    assert!(output.contains("}"), "got: {output}");
}
