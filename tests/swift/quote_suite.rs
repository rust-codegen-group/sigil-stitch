use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::prelude::*;

use crate::shared::LanguageTestSuite;

pub struct SwiftSuite;

impl LanguageTestSuite for SwiftSuite {
    fn control_flow_block() -> CodeBlock {
        sigil_quote!(Swift {
            if x > 0 {
                return true;
            } else {
                return false;
            }
        })
        .unwrap()
    }

    fn control_flow_golden_path() -> &'static str {
        "swift/macro_control_flow.swift"
    }

    fn basic_block() -> CodeBlock {
        sigil_quote!(Swift {
            let name: String = $S("Alice");
            let age: Int = 30;
            print(name, age);
        })
        .unwrap()
    }

    fn basic_golden_path() -> &'static str {
        "swift/macro_basic.swift"
    }

    fn file_spec_name() -> &'static str {
        "test.swift"
    }
}
