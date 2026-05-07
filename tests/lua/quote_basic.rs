use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::lua::Lua;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

use super::golden;

fn render(block: &CodeBlock) -> String {
    FileSpec::builder_with("test.lua", Lua::new())
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

#[test]
fn test_basic() {
    let block = sigil_quote!(Lua {
        local name = $S("Alice")
        local age = 30
        print(name, age)
    })
    .unwrap();
    golden::assert_golden("lua/macro_basic.lua", &render(&block));
}

#[test]
fn test_function() {
    let block = sigil_quote!(Lua {
        function greet(name) {
            return "Hello, "..name
        }
    })
    .unwrap();
    golden::assert_golden("lua/macro_function.lua", &render(&block));
}

#[test]
fn test_table() {
    let block = sigil_quote!(Lua {
        local user = {
            name = $S("Bob"),
            age = 42,
        }
        print(user.name)
    })
    .unwrap();
    golden::assert_golden("lua/macro_table.lua", &render(&block));
}
