mod golden;

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::java_lang::JavaLang;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::type_name::TypeName;

#[test]
fn test_java_function_with_imports() {
    let list = TypeName::<JavaLang>::importable("java.util", "List");
    let user = TypeName::<JavaLang>::importable("com.example.model", "User");

    let mut b = CodeBlock::<JavaLang>::builder();
    b.add_statement("%T<%T> users = getAll()", (list, user));
    let block = b.build().unwrap();

    let mut fb = FileSpec::builder_with("App.java", JavaLang::new());
    fb.add_code(block);
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("java/function_with_imports.java", &output);
}

#[test]
fn test_java_import_grouping() {
    let list = TypeName::<JavaLang>::importable("java.util", "List");
    let file_type = TypeName::<JavaLang>::importable("java.io", "File");
    let entity = TypeName::<JavaLang>::importable("javax.persistence", "Entity");
    let user = TypeName::<JavaLang>::importable("com.example.model", "User");
    let helper = TypeName::<JavaLang>::importable("com.example.util", "Helper");

    let mut b = CodeBlock::<JavaLang>::builder();
    b.add("// %T %T %T %T %T", (list, file_type, entity, user, helper));
    b.add_line();
    let block = b.build().unwrap();

    let mut fb = FileSpec::builder_with("Imports.java", JavaLang::new());
    fb.add_code(block);
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("java/import_grouping.java", &output);
}

#[test]
fn test_java_control_flow() {
    let mut b = CodeBlock::<JavaLang>::builder();
    b.begin_control_flow("if (x > 0)", ());
    b.add_statement("return 1", ());
    b.next_control_flow("else if (x < 0)", ());
    b.add_statement("return -1", ());
    b.next_control_flow("else", ());
    b.add_statement("return 0", ());
    b.end_control_flow();
    let block = b.build().unwrap();

    let mut fb = FileSpec::builder_with("Flow.java", JavaLang::new());
    fb.add_code(block);
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("java/control_flow.java", &output);
}
