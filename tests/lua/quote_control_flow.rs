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
fn test_if_else() {
    let block = sigil_quote!(Lua {
        if x > 0 then {
            return $S("positive")
        } elseif x < 0 then {
            return $S("negative")
        } else {
            return $S("zero")
        }
    })
    .unwrap();
    golden::assert_golden("lua/macro_control_flow.lua", &render(&block));
}

#[test]
fn test_for_loop() {
    let block = sigil_quote!(Lua {
        for i = 1, 10 do {
            print(i)
        }
    })
    .unwrap();
    golden::assert_golden("lua/macro_for_loop.lua", &render(&block));
}

#[test]
fn test_while_loop() {
    let block = sigil_quote!(Lua {
        while x > 0 do {
            x = x - 1
        }
    })
    .unwrap();
    golden::assert_golden("lua/macro_while_loop.lua", &render(&block));
}

// Note: repeat/until is not supported via sigil_quote! because `{...}` after
// `repeat` is treated as control flow, and `end_control_flow()` emits
// `BlockClose` ("end") instead of `until condition`. Use manual formatting:
//   $>  x = x - 1
//   $<until x == 0
