use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::php::Php;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

use crate::shared::LanguageTestSuite;

pub struct PhpSuite;

impl LanguageTestSuite for PhpSuite {
    fn control_flow_block() -> CodeBlock {
        sigil_quote!(Php {
            if ($$x > 0) {
                return $S("positive");
            } else {
                return $S("negative");
            }
        })
        .unwrap()
    }

    fn control_flow_golden_path() -> &'static str {
        "php/macro_control_flow.php"
    }

    fn basic_block() -> CodeBlock {
        sigil_quote!(Php {
            $$x = 42;
            $$name = $S("Alice");
            echo $$name, $$x;
        })
        .unwrap()
    }

    fn basic_golden_path() -> &'static str {
        "php/macro_basic.php"
    }

    fn render(block: CodeBlock) -> String {
        FileSpec::builder_with("test.php", Php::new())
            .add_code(block)
            .build()
            .unwrap()
            .render(80)
            .unwrap()
    }

    fn file_spec_name() -> &'static str {
        "test.php"
    }
}
