use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::haskell::Haskell;
use sigil_stitch::spec::file_spec::FileSpec;

use super::golden;

#[test]
fn test_type_annotation() {
    let mut b = CodeBlock::builder();
    b.add_statement("map :: (a -> b) -> f a -> f b", ());
    b.add_statement("id :: a -> a", ());
    let block = b.build().unwrap();

    let file = FileSpec::builder_with("test.hs", Haskell::new())
        .add_code(block)
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("haskell/builder_type_annotation.hs", &output);
}
