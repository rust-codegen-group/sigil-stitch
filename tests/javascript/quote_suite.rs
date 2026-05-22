use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::javascript::JavaScript;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

use crate::shared::LanguageTestSuite;

pub struct JavaScriptSuite;

impl LanguageTestSuite for JavaScriptSuite {
    fn control_flow_block() -> CodeBlock {
        sigil_quote!(JavaScript {
            if(x > 0) {
                return $S("positive");
            } else if (x < 0) {
                return $S("negative");
            } else {
                return $S("zero");
            }
        })
        .unwrap()
    }

    fn control_flow_golden_path() -> &'static str {
        "javascript/macro_control_flow.js"
    }

    fn basic_block() -> CodeBlock {
        sigil_quote!(JavaScript {
            const name = $S("Alice");
            const age = $L("30");
            console.log(name, age);
        })
        .unwrap()
    }

    fn basic_golden_path() -> &'static str {
        "javascript/macro_basic.js"
    }

    fn render(block: CodeBlock) -> String {
        FileSpec::builder_with("test.js", JavaScript::new())
            .add_code(block)
            .build()
            .unwrap()
            .render(80)
            .unwrap()
    }

    fn file_spec_name() -> &'static str {
        "test.js"
    }
}
