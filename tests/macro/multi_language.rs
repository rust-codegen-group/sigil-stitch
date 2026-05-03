use super::helpers::*;

#[test]
fn test_rust_language() {
    let block = sigil_quote!(RustLang {
        let x = 42;
    })
    .unwrap();

    let output = render_rs(&block);
    assert!(output.contains("let x = 42;"), "got: {output}");
}

#[test]
fn test_python_control_flow() {
    let mut b = CodeBlock::builder();
    b.begin_control_flow("if x > 0", ());
    b.add_statement("return True", ());
    b.end_control_flow();
    let manual = b.build().unwrap();

    let macro_block = sigil_quote!(Python {
        if x > 0 {
            return True;
        }
    })
    .unwrap();

    let manual_output = render_py(&manual);
    let macro_output = render_py(&macro_block);
    assert!(
        manual_output.contains("if x > 0:"),
        "manual: {manual_output}"
    );
    assert!(macro_output.contains("if x > 0:"), "macro: {macro_output}");
}

#[test]
fn test_go_language() {
    let block = sigil_quote!(GoLang {
        x := 42;
    })
    .unwrap();

    let output = render_go(&block);
    assert!(output.contains("x := 42"), "got: {output}");
}
