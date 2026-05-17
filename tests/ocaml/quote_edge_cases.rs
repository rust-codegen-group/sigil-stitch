use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::ocaml::OCaml;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

use super::golden;

fn render(block: &CodeBlock) -> String {
    FileSpec::builder_with("test.ml", OCaml::new())
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

#[test]
fn test_open_struct() {
    let block = sigil_quote!(OCaml {
        module Foo {
            let x = 42;
            let name = $S("Alice");
        }
    })
    .unwrap();
    golden::assert_golden("ocaml/macro_open_struct.ml", &render(&block));
}

#[test]
fn test_pipe_operator() {
    let block = sigil_quote!(OCaml {
        let result = input
            |> List.map f
            |> List.filter g
            |> List.fold_left (+) 0
    })
    .unwrap();
    golden::assert_golden("ocaml/quote_pipe.ml", &render(&block));
}

#[test]
fn test_pattern_match() {
    let block = sigil_quote!(OCaml {
        let describe x = match x with {
            | Some(v) -> Printf.printf $S("value: %d") v
            | None -> print_endline $S("none")
        }
    })
    .unwrap();
    golden::assert_golden("ocaml/quote_pattern_match.ml", &render(&block));
}

#[test]
fn test_let_in() {
    let block = sigil_quote!(OCaml {
        let compute x =
            let squared = x * x in
            let doubled = squared * 2 in
            doubled + 1
    })
    .unwrap();
    golden::assert_golden("ocaml/quote_let_in.ml", &render(&block));
}

#[test]
fn test_record_update() {
    let block = sigil_quote!(OCaml {
        let updated = { user with name = $S("Bob"); age = 30 }
    })
    .unwrap();
    golden::assert_golden("ocaml/quote_record_update.ml", &render(&block));
}
