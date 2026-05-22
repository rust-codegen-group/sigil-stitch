use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::ocaml::OCaml;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

use crate::shared::LanguageTestSuite;

pub struct OCamlSuite;

impl LanguageTestSuite for OCamlSuite {
    fn control_flow_block() -> CodeBlock {
        sigil_quote!(OCaml {
            if x > 0 {
                return true;
            } else {
                return false;
            }
        })
        .unwrap()
    }

    fn control_flow_golden_path() -> &'static str {
        "ocaml/macro_control_flow.ml"
    }

    fn basic_block() -> CodeBlock {
        sigil_quote!(OCaml {
            let x = 42;
            let name = $S("Alice");
        })
        .unwrap()
    }

    fn basic_golden_path() -> &'static str {
        "ocaml/macro_basic.ml"
    }

    fn render(block: CodeBlock) -> String {
        FileSpec::builder_with("test.ml", OCaml::new())
            .add_code(block)
            .build()
            .unwrap()
            .render(80)
            .unwrap()
    }

    fn file_spec_name() -> &'static str {
        "test.ml"
    }
}
