use super::helpers::*;

#[test]
fn test_open_haskell_where() {
    let block = sigil_quote!(Haskell {
        class Functor f $open(" where") {
            fmap :: (a -> b) -> f a -> f b;
        }
    })
    .unwrap();

    let output = render_hs(&block);
    assert!(output.contains("class Functor f where"), "got: {output}");
    assert!(output.contains("fmap"), "got: {output}");
    assert!(!output.contains("class Functor f ="), "got: {output}");
}

#[test]
fn test_open_ocaml_module() {
    let block = sigil_quote!(OCaml {
        module Foo $open(" = struct") {
            let x = 42;
        }
    })
    .unwrap();

    let output = render_ml(&block);
    assert!(output.contains("module Foo = struct"), "got: {output}");
    assert!(output.contains("let x = 42"), "got: {output}");
}

#[test]
fn test_open_empty_suppresses_block_opener() {
    let block = sigil_quote!(TypeScript {
        something $open("") {
            body;
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("something"), "got: {output}");
    assert!(output.contains("body;"), "got: {output}");
    assert!(!output.contains("something {"), "got: {output}");
}
