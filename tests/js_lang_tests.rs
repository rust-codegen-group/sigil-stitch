mod golden;

use sigil_stitch::code_block::{CodeBlock, StringLitArg};
use sigil_stitch::lang::javascript::JavaScript;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::type_name::TypeName;

#[test]
fn test_js_function_with_imports() {
    let format_date = TypeName::<JavaScript>::importable("./utils", "formatDate");
    let logger = TypeName::<JavaScript>::importable("./logger", "Logger");

    let mut b = CodeBlock::<JavaScript>::builder();
    b.add("function greet(name) {", ());
    b.add_line();
    b.add("%>", ());
    b.add("const date = %T();", (format_date,));
    b.add_line();
    b.add(
        "%T.log(%S + name);",
        (logger, StringLitArg("Hello, ".to_string())),
    );
    b.add_line();
    b.add("%<", ());
    b.add("}", ());
    b.add_line();
    let block = b.build().unwrap();

    let mut fb = FileSpec::builder_with("greet.js", JavaScript::new());
    fb.add_code(block);
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("javascript/function_with_imports.js", &output);
}

#[test]
fn test_js_no_import_type() {
    // Even when is_type_only is true, JS should NOT emit `import type`.
    let user = TypeName::<JavaScript>::importable_type("./models", "User");
    let create = TypeName::<JavaScript>::importable("./models", "createUser");

    let mut b = CodeBlock::<JavaScript>::builder();
    b.add("const u = new %T();", (user,));
    b.add_line();
    b.add("%T();", (create,));
    b.add_line();
    let block = b.build().unwrap();

    let mut fb = FileSpec::builder_with("test.js", JavaScript::new());
    fb.add_code(block);
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("javascript/no_import_type.js", &output);
}

#[test]
fn test_js_control_flow() {
    let mut b = CodeBlock::<JavaScript>::builder();
    b.begin_control_flow("if (x > 0)", ());
    b.add_statement("return 1", ());
    b.next_control_flow("else if (x < 0)", ());
    b.add_statement("return -1", ());
    b.next_control_flow("else", ());
    b.add_statement("return 0", ());
    b.end_control_flow();
    let block = b.build().unwrap();

    let mut fb = FileSpec::builder_with("flow.js", JavaScript::new());
    fb.add_code(block);
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("javascript/control_flow.js", &output);
}
