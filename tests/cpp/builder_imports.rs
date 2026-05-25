use sigil_stitch::code_block::{CodeBlock, StringLitArg};
use sigil_stitch::lang::cpp::Cpp;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

#[test]
fn test_includes() {
    let cout = TypeName::importable("iostream", "std::cout");
    let vector = TypeName::importable("vector", "std::vector");
    let myclass = TypeName::importable("./myclass.hpp", "MyClass");

    let mut b = CodeBlock::builder();
    b.add_statement("%T << %S", (cout, StringLitArg("hello".to_string())));
    b.add_statement("%T v", (vector,));
    b.add_statement("%T obj", (myclass,));
    let block = b.build().unwrap();

    let file = FileSpec::builder_with("test.cpp", Cpp::new())
        .add_code(block)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("cpp/includes.cpp", &output);
}
