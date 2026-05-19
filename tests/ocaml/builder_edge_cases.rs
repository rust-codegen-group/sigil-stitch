use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::RendererLang;
use sigil_stitch::lang::ocaml::OCaml;
use sigil_stitch::spec::file_spec::FileSpec;

use super::golden;

#[test]
fn test_comment_closes() {
    let mut b = CodeBlock::builder();
    b.add_comment("this is a comment");
    let block = b.build().unwrap();

    let file = FileSpec::builder_with("comment.ml", OCaml::new())
        .add_code(block)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("ocaml/comment.ml", &output);
}

#[test]
fn test_line_comment_suffix() {
    let ml = OCaml::new();
    assert_eq!(ml.line_comment_suffix(), " *)");
}
