use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::kotlin::Kotlin;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

use crate::shared::LanguageTestSuite;

pub struct KotlinSuite;

impl LanguageTestSuite for KotlinSuite {
    fn control_flow_block() -> CodeBlock {
        sigil_quote!(Kotlin {
            if(x > 0) {
                return $S("positive");
            } else {
                return $S("negative");
            }
        })
        .unwrap()
    }

    fn control_flow_golden_path() -> &'static str {
        "kotlin/macro_control_flow.kt"
    }

    fn basic_block() -> CodeBlock {
        sigil_quote!(Kotlin {
            val name = $S("Alice");
            val age = 30;
            println(name);
        })
        .unwrap()
    }

    fn basic_golden_path() -> &'static str {
        "kotlin/macro_basic.kt"
    }

    fn render(block: CodeBlock) -> String {
        FileSpec::builder_with("test.kt", Kotlin::new())
            .add_code(block)
            .build()
            .unwrap()
            .render(80)
            .unwrap()
    }

    fn file_spec_name() -> &'static str {
        "test.kt"
    }
}
