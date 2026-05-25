use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::cpp::Cpp;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

use crate::shared::LanguageTestSuite;

pub struct CppSuite;

impl LanguageTestSuite for CppSuite {
    fn control_flow_block() -> CodeBlock {
        sigil_quote!(Cpp {
            if(x > 0) {
                return true;
            } else {
                return false;
            }
        })
        .unwrap()
    }

    fn control_flow_golden_path() -> &'static str {
        "cpp/macro_control_flow.cpp"
    }

    fn basic_block() -> CodeBlock {
        sigil_quote!(Cpp {
            int x = 42;
            std::string name = $S("Alice");
            std::cout << name << std::endl;
        })
        .unwrap()
    }

    fn basic_golden_path() -> &'static str {
        "cpp/macro_basic.cpp"
    }

    fn render(block: CodeBlock) -> String {
        FileSpec::builder_with("test.cpp", Cpp::new())
            .add_code(block)
            .build()
            .unwrap()
            .render(80)
            .unwrap()
    }

    fn file_spec_name() -> &'static str {
        "test.cpp"
    }
}
