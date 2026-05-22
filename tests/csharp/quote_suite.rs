use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::csharp::CSharp;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

use crate::shared::LanguageTestSuite;

pub struct CSharpSuite;

impl LanguageTestSuite for CSharpSuite {
    fn control_flow_block() -> CodeBlock {
        sigil_quote!(CSharp {
            if (x > 0) {
                return 1;
            } else if (x < 0) {
                return -1;
            } else {
                return 0;
            }
        })
        .unwrap()
    }

    fn control_flow_golden_path() -> &'static str {
        "csharp/macro_control_flow.cs"
    }

    fn basic_block() -> CodeBlock {
        sigil_quote!(CSharp {
            string name = $S("Alice");
            int age = 30;
            Console.WriteLine(name);
        })
        .unwrap()
    }

    fn basic_golden_path() -> &'static str {
        "csharp/macro_basic.cs"
    }

    fn render(block: CodeBlock) -> String {
        FileSpec::builder_with("Test.cs", CSharp::new())
            .add_code(block)
            .build()
            .unwrap()
            .render(80)
            .unwrap()
    }

    fn file_spec_name() -> &'static str {
        "Test.cs"
    }
}
