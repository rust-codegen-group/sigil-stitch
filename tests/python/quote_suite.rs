use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::prelude::*;

use crate::shared::LanguageTestSuite;

pub struct PythonSuite;

impl LanguageTestSuite for PythonSuite {
    fn control_flow_block() -> CodeBlock {
        sigil_quote!(Python {
            if x > 0 {
                return $S("positive");
            } else {
                return $S("negative");
            }
        })
        .unwrap()
    }

    fn control_flow_golden_path() -> &'static str {
        "python/macro_control_flow.py"
    }

    fn basic_block() -> CodeBlock {
        sigil_quote!(Python {
            name = $S("Alice");
            age = 30;
            print(name, age);
        })
        .unwrap()
    }

    fn basic_golden_path() -> &'static str {
        "python/macro_basic.py"
    }

    fn file_spec_name() -> &'static str {
        "test.py"
    }
}
