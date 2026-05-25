use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::go::Go;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

use crate::shared::LanguageTestSuite;

pub struct GoSuite;

impl LanguageTestSuite for GoSuite {
    fn control_flow_block() -> CodeBlock {
        sigil_quote!(Go {
            if x > 0 {
                return $S("positive");
            } else {
                return $S("negative");
            }
        })
        .unwrap()
    }

    fn control_flow_golden_path() -> &'static str {
        "go/macro_control_flow.go"
    }

    fn basic_block() -> CodeBlock {
        sigil_quote!(Go {
            x := 42;
            name := $S("Alice");
            fmt.Println(name, x);
        })
        .unwrap()
    }

    fn basic_golden_path() -> &'static str {
        "go/macro_basic.go"
    }

    fn render(block: CodeBlock) -> String {
        FileSpec::builder_with("test.go", Go::new())
            .add_code(block)
            .build()
            .unwrap()
            .render(80)
            .unwrap()
    }

    fn file_spec_name() -> &'static str {
        "test.go"
    }
}
