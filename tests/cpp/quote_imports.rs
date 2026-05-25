use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::cpp::Cpp;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

fn render(block: &CodeBlock) -> String {
    FileSpec::builder_with("test.cpp", Cpp::new())
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

#[test]
fn test_imports() {
    let vector = TypeName::generic(
        TypeName::importable("vector", "std::vector"),
        vec![TypeName::primitive("int")],
    );
    let string = TypeName::importable("string", "std::string");
    let block = sigil_quote!(Cpp {
        $T(vector) items;
        $T(string) name = $S("Alice");
    })
    .unwrap();
    golden::assert_golden("cpp/macro_imports.cpp", &render(&block));
}
