use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::prelude::*;

use crate::shared::LanguageTestSuite;

pub struct ZshSuite;

impl LanguageTestSuite for ZshSuite {
    fn control_flow_block() -> CodeBlock {
        sigil_quote!(Zsh {
            if [ $L("$x") -gt 0 ] {
                echo $S("positive");
            } else {
                echo $S("negative");
            }
        })
        .unwrap()
    }

    fn control_flow_golden_path() -> &'static str {
        "zsh/macro_control_flow.zsh"
    }

    fn basic_block() -> CodeBlock {
        sigil_quote!(Zsh {
            NAME=$S("Alice");
            AGE=30;
            echo $L("$NAME") $L("$AGE");
        })
        .unwrap()
    }

    fn basic_golden_path() -> &'static str {
        "zsh/macro_basic.zsh"
    }

    fn file_spec_name() -> &'static str {
        "test.zsh"
    }
}
