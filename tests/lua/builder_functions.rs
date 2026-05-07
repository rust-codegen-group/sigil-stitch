use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::lua::Lua;
use sigil_stitch::spec::file_spec::FileSpec;

use super::golden;

#[test]
fn test_function() {
    let mut b = CodeBlock::builder();
    b.add_statement("function greet(name)", ());
    b.add_statement("  return $\"Hello, {name}!\"", ());
    b.add_statement("end", ());

    let file = FileSpec::builder_with("greeter.lua", Lua::new())
        .add_code(b.build().unwrap())
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("lua/function.lua", &output);
}

#[test]
fn test_function_with_doc() {
    let mut b = CodeBlock::builder();
    b.add_statement("--- Returns the user's name.", ());
    b.add_statement("function get_name(self)", ());
    b.add_statement("  return self.name", ());
    b.add_statement("end", ());

    let file = FileSpec::builder_with("user.lua", Lua::new())
        .add_code(b.build().unwrap())
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("lua/function_with_doc.lua", &output);
}
