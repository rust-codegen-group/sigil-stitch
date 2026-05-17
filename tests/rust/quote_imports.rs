use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::rust_lang::RustLang;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

fn render(block: &CodeBlock) -> String {
    FileSpec::builder_with("test.rs", RustLang::new())
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

#[test]
fn test_imports() {
    let hashmap = TypeName::generic(
        TypeName::importable("std::collections", "HashMap"),
        vec![TypeName::primitive("String"), TypeName::primitive("i32")],
    );
    let vec_deque = TypeName::generic(
        TypeName::importable("std::collections", "VecDeque"),
        vec![TypeName::primitive("String")],
    );
    let block = sigil_quote!(RustLang {
        fn demo() {
            let map: $T(hashmap) = HashMap::new();
            let deque: $T(vec_deque) = VecDeque::new();
        }
    })
    .unwrap();
    golden::assert_golden("rust/macro_imports.rs", &render(&block));
}
