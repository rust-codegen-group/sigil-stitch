mod golden;

use sigil_stitch::code_block::{CodeBlock, StringLitArg};
use sigil_stitch::lang::cpp_lang::CppLang;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::type_name::TypeName;

#[test]
fn test_cpp_includes() {
    let cout = TypeName::<CppLang>::importable("iostream", "std::cout");
    let vector = TypeName::<CppLang>::importable("vector", "std::vector");
    let myclass = TypeName::<CppLang>::importable("./myclass.hpp", "MyClass");

    let mut b = CodeBlock::<CppLang>::builder();
    b.add_statement("%T << %S", (cout, StringLitArg("hello".to_string())));
    b.add_statement("%T v", (vector,));
    b.add_statement("%T obj", (myclass,));
    let block = b.build().unwrap();

    let mut fb = FileSpec::builder_with("test.cpp", CppLang::new());
    fb.add_code(block);
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("cpp/includes.cpp", &output);
}

#[test]
fn test_cpp_namespace_wrapping() {
    let mut b = CodeBlock::<CppLang>::builder();
    b.add("int square(int x) {", ());
    b.add_line();
    b.add("%>", ());
    b.add("return x * x;", ());
    b.add_line();
    b.add("%<", ());
    b.add("}", ());
    b.add_line();
    let block = b.build().unwrap();

    let mut fb = FileSpec::builder_with("math.hpp", CppLang::header());
    fb.header(CodeBlock::<CppLang>::of("#pragma once", ()).unwrap());
    fb.add_raw("\nnamespace math {\n");
    fb.add_code(block);
    fb.add_raw("\n} // namespace math\n");
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("cpp/namespace_wrapping.cpp", &output);
}

#[test]
fn test_cpp_control_flow() {
    let mut b = CodeBlock::<CppLang>::builder();
    b.begin_control_flow("if (x > 0)", ());
    b.add_statement("return 1", ());
    b.next_control_flow("else if (x < 0)", ());
    b.add_statement("return -1", ());
    b.next_control_flow("else", ());
    b.add_statement("return 0", ());
    b.end_control_flow();
    let block = b.build().unwrap();

    let mut fb = FileSpec::builder_with("flow.cpp", CppLang::new());
    fb.add_code(block);
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("cpp/control_flow.cpp", &output);
}
