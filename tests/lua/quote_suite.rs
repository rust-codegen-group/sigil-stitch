use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::lua::Lua;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

use crate::shared::LanguageTestSuite;

pub struct LuaSuite;

impl LanguageTestSuite for LuaSuite {
    fn control_flow_block() -> CodeBlock {
        sigil_quote!(Lua {
            if x > 0 then {
                return $S("positive")
            } elseif x < 0 then {
                return $S("negative")
            } else {
                return $S("zero")
            }
        })
        .unwrap()
    }

    fn control_flow_golden_path() -> &'static str {
        "lua/macro_control_flow.lua"
    }

    fn basic_block() -> CodeBlock {
        sigil_quote!(Lua {
            local name = $S("Alice")
            local age = 30
            print(name, age)
        })
        .unwrap()
    }

    fn basic_golden_path() -> &'static str {
        "lua/macro_basic.lua"
    }

    fn render(block: CodeBlock) -> String {
        FileSpec::builder_with("test.lua", Lua::new())
            .add_code(block)
            .build()
            .unwrap()
            .render(80)
            .unwrap()
    }

    fn file_spec_name() -> &'static str {
        "test.lua"
    }
}
