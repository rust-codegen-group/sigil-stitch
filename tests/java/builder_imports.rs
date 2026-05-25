use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::java::Java;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

#[test]
fn test_function_with_imports() {
    let list = TypeName::importable("java.util", "List");
    let user = TypeName::importable("com.example.model", "User");

    let mut b = CodeBlock::builder();
    b.add_statement("%T<%T> users = getAll()", (list, user));
    let block = b.build().unwrap();

    let file = FileSpec::builder_with("App.java", Java::new())
        .add_code(block)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("java/function_with_imports.java", &output);
}

#[test]
fn test_import_grouping() {
    let list = TypeName::importable("java.util", "List");
    let file_type = TypeName::importable("java.io", "File");
    let entity = TypeName::importable("javax.persistence", "Entity");
    let user = TypeName::importable("com.example.model", "User");
    let helper = TypeName::importable("com.example.util", "Helper");

    let mut b = CodeBlock::builder();
    b.add("// %T %T %T %T %T", (list, file_type, entity, user, helper));
    b.add_line();
    let block = b.build().unwrap();

    let file = FileSpec::builder_with("Imports.java", Java::new())
        .add_code(block)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("java/import_grouping.java", &output);
}
