use super::helpers::*;

#[test]
fn test_simple_statement() {
    let block = sigil_quote!(TypeScript {
        const x = 42;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("const x = 42;"), "got: {output}");
}

#[test]
fn test_multiple_statements() {
    let block = sigil_quote!(TypeScript {
        const a = 1;
        const b = 2;
        const c = a + b;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("const a = 1;"), "got: {output}");
    assert!(output.contains("const b = 2;"), "got: {output}");
    assert!(output.contains("const c = a + b;"), "got: {output}");
}

#[test]
fn test_empty_body() {
    let block = sigil_quote!(TypeScript {}).unwrap();
    assert!(block.is_empty());
}
