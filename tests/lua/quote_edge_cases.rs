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
fn test_string_concat() {
    let block = sigil_quote!(Lua {
        local greeting = "Hello, "..name.."!"
        local path = dir.."/"..file
    })
    .unwrap();
    golden::assert_golden("lua/quote_string_concat.lua", &render(&block));
}

#[test]
fn test_local_function() {
    let block = sigil_quote!(Lua {
        local function add(a, b) {
            return a + b
        }
        local result = add(1, 2)
    })
    .unwrap();
    golden::assert_golden("lua/quote_local_function.lua", &render(&block));
}

#[test]
fn test_method_call() {
    let block = sigil_quote!(Lua {
        local obj = {}
        obj:init("config")
        local result = obj:getValue()
    })
    .unwrap();
    golden::assert_golden("lua/quote_method_call.lua", &render(&block));
}

#[test]
fn test_vararg() {
    let block = sigil_quote!(Lua {
        local function printf(fmt, ...) {
            local args = {...}
            print(string.format(fmt, ...))
        }
    })
    .unwrap();
    golden::assert_golden("lua/quote_vararg.lua", &render(&block));
}

#[test]
fn test_multiline_table() {
    let block = sigil_quote!(Lua {
        local config = {
            host = "localhost",
            port = 8080,
            debug = true,
        }
    })
    .unwrap();
    golden::assert_golden("lua/quote_multiline_table.lua", &render(&block));
}

#[test]
fn test_for_loop() {
    let block = sigil_quote!(Lua {
        for i = 1, 10 do {
            print(i)
        }
        for k, v in pairs(t) do {
            print(k, v)
        }
    })
    .unwrap();
    golden::assert_golden("lua/quote_for_loop.lua", &render(&block));
}
