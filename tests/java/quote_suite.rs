use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::java::Java;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

use crate::shared::LanguageTestSuite;

pub struct JavaSuite;

impl LanguageTestSuite for JavaSuite {
    fn control_flow_block() -> CodeBlock {
        sigil_quote!(Java {
            if(x > 0) {
                return $S("positive");
            } else {
                return $S("negative");
            }
        })
        .unwrap()
    }

    fn control_flow_golden_path() -> &'static str {
        "java/macro_control_flow.java"
    }

    fn basic_block() -> CodeBlock {
        sigil_quote!(Java {
            String name = $S("Alice");
            int age = 30;
            System.out.println(name);
        })
        .unwrap()
    }

    fn basic_golden_path() -> &'static str {
        "java/macro_basic.java"
    }

    fn render(block: CodeBlock) -> String {
        FileSpec::builder_with("Test.java", Java::new())
            .add_code(block)
            .build()
            .unwrap()
            .render(80)
            .unwrap()
    }

    fn file_spec_name() -> &'static str {
        "Test.java"
    }
}
