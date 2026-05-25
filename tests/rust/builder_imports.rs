use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::rust::Rust;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

#[test]
fn test_function_with_imports() {
    let hashmap = TypeName::importable("std::collections", "HashMap");
    let serialize = TypeName::importable("serde", "Serialize");

    let mut b = CodeBlock::builder();
    b.add("pub fn create_map() -> %T<String, String> {", (hashmap,));
    b.add_line();
    b.add("%>", ());
    b.add_statement("let mut map = HashMap::new()", ());
    b.add_statement("map.insert(\"key\".to_string(), \"value\".to_string())", ());
    b.add("map", ());
    b.add_line();
    b.add("%<", ());
    b.add("}", ());
    b.add_line();
    let func_block = b.build().unwrap();

    let mut b2 = CodeBlock::builder();
    b2.add("#[derive(%T)]", (serialize,));
    b2.add_line();
    b2.add("pub struct Config {", ());
    b2.add_line();
    b2.add("%>", ());
    b2.add("pub name: String,", ());
    b2.add_line();
    b2.add("%<", ());
    b2.add("}", ());
    b2.add_line();
    let struct_block = b2.build().unwrap();

    let file = FileSpec::builder_with("lib.rs", Rust::new())
        .add_code(func_block)
        .add_code(struct_block)
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("rust/function_with_imports.rs", &output);
}

#[test]
fn test_import_grouping() {
    let hashmap = TypeName::importable("std::collections", "HashMap");
    let btreemap = TypeName::importable("std::collections", "BTreeMap");
    let arc = TypeName::importable("std::sync", "Arc");
    let serialize = TypeName::importable("serde", "Serialize");
    let deserialize = TypeName::importable("serde", "Deserialize");
    let user = TypeName::importable("crate::models", "User");

    let mut b = CodeBlock::builder();
    b.add("fn demo() {", ());
    b.add_line();
    b.add("%>", ());
    b.add_statement(
        "let _h: %T<String, String> = Default::default()",
        (hashmap,),
    );
    b.add_statement(
        "let _b: %T<String, String> = Default::default()",
        (btreemap,),
    );
    b.add_statement("let _a: %T<String> = Arc::new(\"x\".into())", (arc,));
    b.add_statement("let _s = %T::default()", (serialize,));
    b.add_statement("let _d = %T::default()", (deserialize,));
    b.add_statement("let _u: %T = todo!()", (user,));
    b.add("%<", ());
    b.add("}", ());
    b.add_line();
    let block = b.build().unwrap();

    let file = FileSpec::builder_with("main.rs", Rust::new())
        .add_code(block)
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("rust/import_grouping.rs", &output);
}

#[test]
fn test_import_conflict() {
    let user1 = TypeName::importable("models", "User");
    let user2 = TypeName::importable("other_models", "User");

    let mut b = CodeBlock::builder();
    b.add_statement("let u1: %T = todo!()", (user1,));
    b.add_statement("let u2: %T = todo!()", (user2,));
    let block = b.build().unwrap();

    let file = FileSpec::builder_with("conflict.rs", Rust::new())
        .add_code(block)
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("rust/import_conflict.rs", &output);
}
