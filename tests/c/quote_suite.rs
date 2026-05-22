use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::c_lang::CLang;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

use crate::shared::LanguageTestSuite;

pub struct CSuite;

impl LanguageTestSuite for CSuite {
    fn control_flow_block() -> CodeBlock {
        sigil_quote!(CLang {
            if(x > 0) {
                return 1;
            } else if (x < 0) {
                return -1;
            } else {
                return 0;
            }
        })
        .unwrap()
    }

    fn control_flow_golden_path() -> &'static str {
        "c/macro_control_flow.c"
    }

    fn basic_block() -> CodeBlock {
        sigil_quote!(CLang {
            int x = 42;
            float y = 3.14;
            printf($S("x=%d y=%f"), x, y);
        })
        .unwrap()
    }

    fn basic_golden_path() -> &'static str {
        "c/macro_basic.c"
    }

    fn render(block: CodeBlock) -> String {
        FileSpec::builder_with("test.c", CLang::new())
            .add_code(block)
            .build()
            .unwrap()
            .render(80)
            .unwrap()
    }

    fn file_spec_name() -> &'static str {
        "test.c"
    }
}
