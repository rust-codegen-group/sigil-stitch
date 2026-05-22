use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::prelude::*;

use crate::shared::LanguageTestSuite;

pub struct BashSuite;

impl LanguageTestSuite for BashSuite {
    fn control_flow_block() -> CodeBlock {
        sigil_quote!(Bash {
            if [ $L("$x") -gt 0 ] {
                echo $S("positive");
            } else {
                echo $S("negative");
            }
        })
        .unwrap()
    }

    fn control_flow_golden_path() -> &'static str {
        "bash/macro_control_flow.bash"
    }

    fn basic_block() -> CodeBlock {
        sigil_quote!(Bash {
            NAME=$S("Alice");
            AGE=30;
            echo $L("$NAME") $L("$AGE");
        })
        .unwrap()
    }

    fn basic_golden_path() -> &'static str {
        "bash/macro_basic.bash"
    }

    fn file_spec_name() -> &'static str {
        "test.bash"
    }
}
