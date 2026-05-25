use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::c::C;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

fn render(block: &CodeBlock) -> String {
    FileSpec::builder_with("test.c", C::new())
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

#[test]
fn test_imports() {
    let stdio = TypeName::importable("stdio.h", "printf");
    let stdlib = TypeName::importable("stdlib.h", "malloc");
    let block = sigil_quote!(C {
        $T(stdio)($S("hello"));
        void* p = $T(stdlib)(sizeof(int));
    })
    .unwrap();
    golden::assert_golden("c/macro_imports.c", &render(&block));
}
