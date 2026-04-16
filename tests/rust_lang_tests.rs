use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::rust_lang::RustLang;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::type_name::TypeName;

mod golden;

// === Simple function with imports ===

#[test]
fn test_function_with_imports() {
    let hashmap = TypeName::<RustLang>::importable("std::collections", "HashMap");
    let serialize = TypeName::<RustLang>::importable("serde", "Serialize");

    let mut b = CodeBlock::<RustLang>::builder();
    b.add("pub fn create_map() -> %T<String, String> {", (hashmap,));
    b.add_line();
    b.add("%>", ());
    b.add_statement("let mut map = HashMap::new()", ());
    b.add_statement("map.insert(\"key\".to_string(), \"value\".to_string())", ());
    b.add_statement("map", ());
    b.add("%<", ());
    b.add("}", ());
    b.add_line();
    let func_block = b.build().unwrap();

    let mut b2 = CodeBlock::<RustLang>::builder();
    b2.add("#[derive(%T)]", (serialize,));
    b2.add_line();
    b2.add("pub struct Config {", ());
    b2.add_line();
    b2.add("%>", ());
    b2.add_statement("pub name: String", ());
    b2.add("%<", ());
    b2.add("}", ());
    b2.add_line();
    let struct_block = b2.build().unwrap();

    let mut fb = FileSpec::builder_with("lib.rs", RustLang::new());
    fb.add_code(func_block);
    fb.add_code(struct_block);
    let file = fb.build().unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("rust/function_with_imports.rs", &output);
}

// === Use grouping: std, external, crate ===

#[test]
fn test_import_grouping() {
    let hashmap = TypeName::<RustLang>::importable("std::collections", "HashMap");
    let btreemap = TypeName::<RustLang>::importable("std::collections", "BTreeMap");
    let arc = TypeName::<RustLang>::importable("std::sync", "Arc");
    let serialize = TypeName::<RustLang>::importable("serde", "Serialize");
    let deserialize = TypeName::<RustLang>::importable("serde", "Deserialize");
    let user = TypeName::<RustLang>::importable("crate::models", "User");

    let mut b = CodeBlock::<RustLang>::builder();
    b.add_statement(
        "let _h: %T<String, String> = Default::default()",
        (hashmap,),
    );
    b.add_statement(
        "let _b: %T<String, String> = Default::default()",
        (btreemap,),
    );
    b.add_statement("let _a: %T<String> = Arc::new(\"x\".into())", (arc,));
    b.add_statement("let _s: %T = todo!()", (serialize,));
    b.add_statement("let _d: %T = todo!()", (deserialize,));
    b.add_statement("let _u: %T = todo!()", (user,));
    let block = b.build().unwrap();

    let mut fb = FileSpec::builder_with("main.rs", RustLang::new());
    fb.add_code(block);
    let file = fb.build().unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("rust/import_grouping.rs", &output);
}

// === Conflict resolution ===

#[test]
fn test_import_conflict() {
    let user1 = TypeName::<RustLang>::importable("models", "User");
    let user2 = TypeName::<RustLang>::importable("other_models", "User");

    let mut b = CodeBlock::<RustLang>::builder();
    b.add_statement("let u1: %T = todo!()", (user1,));
    b.add_statement("let u2: %T = todo!()", (user2,));
    let block = b.build().unwrap();

    let mut fb = FileSpec::builder_with("conflict.rs", RustLang::new());
    fb.add_code(block);
    let file = fb.build().unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("rust/import_conflict.rs", &output);
}

// === Control flow ===

#[test]
fn test_rust_control_flow() {
    let mut b = CodeBlock::<RustLang>::builder();
    b.add("pub fn classify(x: i32) -> &'static str {", ());
    b.add_line();
    b.add("%>", ());
    b.begin_control_flow("if x > 0", ());
    b.add_statement("\"positive\"", ());
    b.next_control_flow("else if x < 0", ());
    b.add_statement("\"negative\"", ());
    b.next_control_flow("else", ());
    b.add_statement("\"zero\"", ());
    b.end_control_flow();
    b.add("%<", ());
    b.add("}", ());
    b.add_line();
    let block = b.build().unwrap();

    let mut fb = FileSpec::builder_with("classify.rs", RustLang::new());
    fb.add_code(block);
    let file = fb.build().unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("rust/control_flow.rs", &output);
}
