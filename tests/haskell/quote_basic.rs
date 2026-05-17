use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::haskell::Haskell;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

use super::golden;

fn render(block: &CodeBlock) -> String {
    FileSpec::builder_with("test.hs", Haskell::new())
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

#[test]
fn test_basic() {
    let block = sigil_quote!(Haskell {
        let x = 42;
        putStrLn $S("hello");
    })
    .unwrap();
    golden::assert_golden("haskell/macro_basic.hs", &render(&block));
}

#[test]
fn test_newtype_declaration() {
    let block = sigil_quote!(Haskell {
        newtype UserId = UserId Int;
        newtype Email = Email String;
    })
    .unwrap();
    golden::assert_golden("haskell/quote_newtype.hs", &render(&block));
}
