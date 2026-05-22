use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::prelude::*;

use crate::shared::LanguageTestSuite;

pub struct ScalaSuite;

impl LanguageTestSuite for ScalaSuite {
    fn control_flow_block() -> CodeBlock {
        sigil_quote!(Scala {
            if(x > 0) {
                return $S("positive");
            } else {
                return $S("negative");
            }
        })
        .unwrap()
    }

    fn control_flow_golden_path() -> &'static str {
        "scala/macro_control_flow.scala"
    }

    fn basic_block() -> CodeBlock {
        sigil_quote!(Scala {
            val name = $S("Alice");
            val age = 30;
            println(name);
        })
        .unwrap()
    }

    fn basic_golden_path() -> &'static str {
        "scala/macro_basic.scala"
    }

    fn file_spec_name() -> &'static str {
        "test.scala"
    }
}
