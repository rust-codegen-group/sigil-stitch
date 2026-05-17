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
fn test_open_where() {
    let block = sigil_quote!(Haskell {
        class Functor f {
            fmap :: (a -> b) -> f a -> f b;
        }
    })
    .unwrap();
    golden::assert_golden("haskell/macro_open_where.hs", &render(&block));
}

#[test]
fn test_type_annotation() {
    let block = sigil_quote!(Haskell {
        map :: (a -> b) -> f a -> f b;
        id :: a -> a;
    })
    .unwrap();
    golden::assert_golden("haskell/quote_type_annotation.hs", &render(&block));
}
