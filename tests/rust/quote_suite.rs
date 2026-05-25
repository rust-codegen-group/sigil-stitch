use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::rust::Rust;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

use crate::shared::LanguageTestSuite;

pub struct RustSuite;

impl LanguageTestSuite for RustSuite {
    fn control_flow_block() -> CodeBlock {
        sigil_quote!(Rust {
            if x > 0 {
                return Ok(x);
            } else {
                return Err($S("negative"));
            }
        })
        .unwrap()
    }

    fn control_flow_golden_path() -> &'static str {
        "rust/macro_control_flow.rs"
    }

    fn basic_block() -> CodeBlock {
        sigil_quote!(Rust {
            let x: i32 = 42;
            let name = $S("Alice");
            println!($S("{}: {}"), name, x);
        })
        .unwrap()
    }

    fn basic_golden_path() -> &'static str {
        "rust/macro_basic.rs"
    }

    fn render(block: CodeBlock) -> String {
        FileSpec::builder_with("test.rs", Rust::new())
            .add_code(block)
            .build()
            .unwrap()
            .render(80)
            .unwrap()
    }

    fn file_spec_name() -> &'static str {
        "test.rs"
    }
}
