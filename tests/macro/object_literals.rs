use super::helpers::*;

#[test]
fn test_object_literal_simple() {
    let block = sigil_quote!(TypeScript {
        const config = { timeout: 30, retries: 3 };
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("config = {"), "got: {output}");
    assert!(output.contains("timeout:"), "got: {output}");
    assert!(output.contains("retries:"), "got: {output}");
}

#[test]
fn test_nested_object_literal() {
    let block = sigil_quote!(TypeScript {
        const x = { a: 1, b: { c: 2 } };
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("{"), "got: {output}");
    assert!(output.contains("a:"), "got: {output}");
}
