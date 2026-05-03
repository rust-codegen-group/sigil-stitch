use super::helpers::*;

#[test]
fn test_percent_in_source() {
    let block = sigil_quote!(TypeScript {
        const x = 100 % 10;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("100 % 10"), "got: {output}");
}

#[test]
fn test_dollar_escape_basic() {
    let block = sigil_quote!(TypeScript {
        const price = $$100;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("$"), "got: {output}");
    assert!(output.contains("100"), "got: {output}");
}

#[test]
fn test_dollar_escape_with_interpolation() {
    let val = "50";
    let block = sigil_quote!(TypeScript {
        const total = $$100 + $L(val);
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("$"), "got: {output}");
    assert!(output.contains("100"), "got: {output}");
    assert!(output.contains("50"), "got: {output}");
}
