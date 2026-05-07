use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::lua::Lua;
use sigil_stitch::spec::file_spec::FileSpec;

use super::golden;

#[test]
fn test_if_else() {
    let mut cb = CodeBlock::builder();
    cb.begin_control_flow("if x > 0 then", ());
    cb.add_statement("return 'positive'", ());
    cb.next_control_flow("elseif x < 0 then", ());
    cb.add_statement("return 'negative'", ());
    cb.next_control_flow("else", ());
    cb.add_statement("return 'zero'", ());
    cb.end_control_flow();

    let file = FileSpec::builder_with("test.lua", Lua::new())
        .add_code(cb.build().unwrap())
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("lua/control_flow.lua", &output);
}

#[test]
fn test_for_loop() {
    let mut cb = CodeBlock::builder();
    cb.begin_control_flow("for i = 1, 10 do", ());
    cb.add_statement("print(i)", ());
    cb.end_control_flow();

    let file = FileSpec::builder_with("test.lua", Lua::new())
        .add_code(cb.build().unwrap())
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("lua/for_loop.lua", &output);
}

#[test]
fn test_while_loop() {
    let mut cb = CodeBlock::builder();
    cb.begin_control_flow("while x > 0 do", ());
    cb.add_statement("x = x - 1", ());
    cb.end_control_flow();

    let file = FileSpec::builder_with("test.lua", Lua::new())
        .add_code(cb.build().unwrap())
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("lua/while_loop.lua", &output);
}

// Note: repeat/until is not supported via the control-flow API because
// `end_control_flow()` emits `BlockClose` ("end"), but `repeat` blocks
// are closed by `until`, not `end`. Use `$>`/`$<` indentation directives
// or manually indented code strings instead.
//
// Example workaround:
//   cb.add("repeat", ());
//   cb.add_statement("  x = x - 1", ());
//   cb.add_statement("until x == 0", ());
