mod golden;

use sigil_stitch::code_block::{CodeBlock, StringLitArg};
use sigil_stitch::lang::c_lang::CLang;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::type_name::TypeName;

#[test]
fn test_c_function_with_includes() {
    let printf_type = TypeName::<CLang>::importable("stdio.h", "printf");
    let config_type = TypeName::<CLang>::importable("./config.h", "Config");

    let mut b = CodeBlock::<CLang>::builder();
    b.add("int main(void) {", ());
    b.add_line();
    b.add("%>", ());
    b.add_statement(
        "%T(%S, config.name)",
        (printf_type, StringLitArg("Hello, %s\\n".to_string())),
    );
    b.add_statement("%T cfg", (config_type,));
    b.add_line();
    b.add_statement("return 0", ());
    b.add("%<", ());
    b.add("}", ());
    b.add_line();
    let block = b.build().unwrap();

    let mut fb = FileSpec::builder_with("main.c", CLang::new());
    fb.add_code(block);
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("c/function_with_includes.c", &output);
}

#[test]
fn test_c_include_grouping() {
    let stdio = TypeName::<CLang>::importable("stdio.h", "printf");
    let stdlib = TypeName::<CLang>::importable("stdlib.h", "malloc");
    let string = TypeName::<CLang>::importable("string.h", "strlen");
    let config = TypeName::<CLang>::importable("./config.h", "Config");
    let utils = TypeName::<CLang>::importable("./utils.h", "helper");

    let mut b = CodeBlock::<CLang>::builder();
    b.add_statement(
        "int x = %T(%S)",
        (stdio.clone(), StringLitArg("hello".to_string())),
    );
    b.add_statement("void* p = %T(sizeof(int))", (stdlib,));
    b.add_statement(
        "size_t n = %T(%S)",
        (string, StringLitArg("test".to_string())),
    );
    b.add_statement("%T cfg", (config,));
    b.add_statement("%T()", (utils,));
    let block = b.build().unwrap();

    let mut fb = FileSpec::builder_with("test.c", CLang::new());
    fb.add_code(block);
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("c/include_grouping.c", &output);
}

#[test]
fn test_c_control_flow() {
    let mut b = CodeBlock::<CLang>::builder();
    b.begin_control_flow("if (x > 0)", ());
    b.add_statement("return 1", ());
    b.next_control_flow("else if (x < 0)", ());
    b.add_statement("return -1", ());
    b.next_control_flow("else", ());
    b.add_statement("return 0", ());
    b.end_control_flow();
    let block = b.build().unwrap();

    let mut fb = FileSpec::builder_with("flow.c", CLang::new());
    fb.add_code(block);
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("c/control_flow.c", &output);
}

#[test]
fn test_c_struct_basic() {
    let mut b = CodeBlock::<CLang>::builder();
    b.add("struct Point", ());
    b.add(" {", ());
    b.add_line();
    b.add("%>", ());
    b.add_statement("int x", ());
    b.add_statement("int y", ());
    b.add("%<", ());
    b.add("};", ());
    b.add_line();
    let block = b.build().unwrap();

    let mut fb = FileSpec::builder_with("point.c", CLang::new());
    fb.add_code(block);
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("c/struct_basic.c", &output);
}
