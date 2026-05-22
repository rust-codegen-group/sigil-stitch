use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::haskell::Haskell;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

use crate::shared::LanguageTestSuite;

pub struct HaskellSuite;

impl LanguageTestSuite for HaskellSuite {
    fn control_flow_block() -> CodeBlock {
        sigil_quote!(Haskell {
            if x > 0 {
                return True;
            } else {
                return False;
            }
        })
        .unwrap()
    }

    fn control_flow_golden_path() -> &'static str {
        "haskell/macro_control_flow.hs"
    }

    fn basic_block() -> CodeBlock {
        sigil_quote!(Haskell {
            let x = 42;
            putStrLn $S("hello");
        })
        .unwrap()
    }

    fn basic_golden_path() -> &'static str {
        "haskell/macro_basic.hs"
    }

    fn render(block: CodeBlock) -> String {
        FileSpec::builder_with("test.hs", Haskell::new())
            .add_code(block)
            .build()
            .unwrap()
            .render(80)
            .unwrap()
    }

    fn file_spec_name() -> &'static str {
        "test.hs"
    }
}
