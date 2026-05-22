use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::prelude::*;

use crate::shared::LanguageTestSuite;

pub struct DartSuite;

impl LanguageTestSuite for DartSuite {
    fn control_flow_block() -> CodeBlock {
        sigil_quote!(DartLang {
            if(x > 0) {
                return true;
            } else {
                return false;
            }
        })
        .unwrap()
    }

    fn control_flow_golden_path() -> &'static str {
        "dart/macro_control_flow.dart"
    }

    fn basic_block() -> CodeBlock {
        sigil_quote!(DartLang {
            final name = $S("Alice");
            final age = 30;
            print(name);
        })
        .unwrap()
    }

    fn basic_golden_path() -> &'static str {
        "dart/macro_basic.dart"
    }

    fn file_spec_name() -> &'static str {
        "test.dart"
    }
}
