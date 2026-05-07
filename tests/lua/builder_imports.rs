use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::lua::Lua;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

#[test]
fn test_import() {
    let json = TypeName::importable("dkjson", "json");
    let inspect = TypeName::importable("inspect", "inspect");

    let mut b = CodeBlock::builder();
    b.add_statement("-- %T %T", (json, inspect));
    let block = b.build().unwrap();

    let file = FileSpec::builder_with("test.lua", Lua::new())
        .add_code(block)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("lua/import.lua", &output);
}
