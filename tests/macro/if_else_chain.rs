use super::helpers::*;

#[test]
fn test_if_else_if_else_chain() {
    let block = sigil_quote!(TypeScript {
        if(x > 0) {
            return 1;
        } else if(x < 0) {
            return -1;
        } else {
            return 0;
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("if (x > 0) {"), "got: {output}");
    assert!(output.contains("} else if (x < 0) {"), "got: {output}");
    assert!(output.contains("} else {"), "got: {output}");
    assert!(output.contains("return 0;"), "got: {output}");
}
