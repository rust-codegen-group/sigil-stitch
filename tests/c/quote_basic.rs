use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::c_lang::CLang;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

use super::golden;

fn render(block: &CodeBlock) -> String {
    FileSpec::builder_with("test.c", CLang::new())
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

#[test]
fn test_basic() {
    let block = sigil_quote!(CLang {
        int x = 42;
        float y = 3.14;
        printf($S("x=%d y=%f"), x, y);
    })
    .unwrap();
    golden::assert_golden("c/macro_basic.c", &render(&block));
}

#[test]
fn test_function_pointer_usage() {
    let block = sigil_quote!(CLang {
        typedef void (*Callback)(int, const char*);
        Callback cb = NULL;
        cb(42, $S("hello"));
    })
    .unwrap();
    golden::assert_golden("c/quote_function_pointer.c", &render(&block));
}
