use super::helpers::*;

#[test]
fn test_haskell_basic_statement() {
    let block = sigil_quote!(Haskell {
        putStrLn "hello";
    })
    .unwrap();

    let output = render_hs(&block);
    assert!(output.contains("putStrLn \"hello\""), "got: {output}");
    assert!(!output.contains(";"), "got: {output}");
}

#[test]
fn test_haskell_control_flow() {
    let block = sigil_quote!(Haskell {
        if x > 0 {
            return True;
        }
    })
    .unwrap();

    let output = render_hs(&block);
    assert!(output.contains("if x > 0 ="), "got: {output}");
    assert!(output.contains("return True"), "got: {output}");
}

#[test]
fn test_ocaml_let_binding() {
    let block = sigil_quote!(OCaml {
        let x = 42;
    })
    .unwrap();

    let output = render_ml(&block);
    assert!(output.contains("let x = 42"), "got: {output}");
}
