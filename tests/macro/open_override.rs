use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::code_renderer::CodeRenderer;
use sigil_stitch::import::ImportGroup;

use super::helpers::*;

#[test]
fn test_haskell_class_where_via_block_open_for() {
    let block = sigil_quote!(Haskell {
        class Functor f {
            fmap :: (a -> b) -> f a -> f b;
        }
    })
    .unwrap();

    let output = render_hs(&block);
    assert!(
        output.contains("class Functor f where"),
        "Haskell block_open_for should emit 'where' for class, got: {output}"
    );
    assert!(output.contains("fmap"), "got: {output}");
    assert!(
        !output.contains("class Functor f ="),
        "should NOT use default block_open, got: {output}"
    );
}

#[test]
fn test_ocaml_module_struct_via_block_open_for() {
    let block = sigil_quote!(OCaml {
        module Foo {
            let x = 42;
        }
    })
    .unwrap();

    let output = render_ml(&block);
    assert!(
        output.contains("module Foo = struct"),
        "OCaml block_open_for should emit '= struct' for module, got: {output}"
    );
    assert!(output.contains("let x = 42"), "got: {output}");
}

#[test]
fn test_ocaml_match_suppresses_block_opener() {
    let block = sigil_quote!(OCaml {
        match x with {
            | Red -> "red";
        }
    })
    .unwrap();

    let output = render_ml(&block);
    assert!(
        output.contains("match x with"),
        "OCaml block_open_for should return empty for match, got: {output}"
    );
    assert!(
        !output.contains("match x with ="),
        "should NOT emit default block_open after match, got: {output}"
    );
}

#[test]
fn test_empty_opener_via_builder_block_open_for() {
    let ocaml = sigil_stitch::lang::ocaml::OCaml::new();
    let imports = ImportGroup::new();
    let mut b = CodeBlock::builder();
    b.begin_control_flow("match color with", ());
    b.add("| Red -> \"red\"", ());
    b.add_line();
    b.end_control_flow();
    let block = b.build().unwrap();
    let renderer = CodeRenderer::new(&ocaml, &imports, 80);
    let output = renderer.render(&block).unwrap();
    assert!(
        output.contains("match color with\n"),
        "block_open_for returning Some(\"\") should suppress opener, got: {output}"
    );
    assert!(
        !output.contains("match color with ="),
        "should NOT contain default block_open, got: {output}"
    );
}
