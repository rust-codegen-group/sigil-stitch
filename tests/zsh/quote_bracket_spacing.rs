//! Tests for bracket spacing in shell `sigil_quote!` expressions.
//! The `[` test command requires spaces: `[ $x -gt 0 ]`, not `[$x -gt 0]`.

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

fn render(block: &CodeBlock) -> String {
    FileSpec::builder("test.zsh")
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

#[test]
fn bracket_test_has_inner_spaces() {
    let block = sigil_quote!(Zsh {
        if [ $L("$x") -gt 0 ] {
            echo $S("positive");
        } else {
            echo $S("negative");
        }
    })
    .unwrap();
    let output = render(&block);

    assert!(
        output.contains("[ $x"),
        "Shell test bracket needs space after '[', got:\n{output}"
    );
    assert!(
        output.contains("0 ]"),
        "Shell test bracket needs space before ']', got:\n{output}"
    );
    assert!(
        !output.contains("[$x"),
        "Missing space after '[' is a syntax error, got:\n{output}"
    );
}
