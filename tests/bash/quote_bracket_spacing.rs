//! Tests for bracket spacing in shell `sigil_quote!` expressions.
//! `[ ... ]` requires inner spaces; `[[ ... ]]` also needs inner spaces.

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

fn render(block: &CodeBlock) -> String {
    FileSpec::builder("test.bash")
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

#[test]
fn single_bracket_quoted_has_inner_spaces() {
    let block = sigil_quote!(Bash {
        if [ "$$x" -gt 0 ] {
            echo $S("positive");
        } else {
            echo $S("negative");
        }
    })
    .unwrap();
    let output = render(&block);

    assert!(
        output.contains("[ \"$x\""),
        "Shell test bracket needs space after '[', got:\n{output}"
    );
    assert!(
        output.contains("0 ]"),
        "Shell test bracket needs space before ']', got:\n{output}"
    );
}

#[test]
fn double_bracket_has_inner_spaces() {
    let block = sigil_quote!(Bash {
        if [[ $$x -gt 0 ]] {
            echo $S("positive");
        } else {
            echo $S("negative");
        }
    })
    .unwrap();
    let output = render(&block);

    assert!(
        output.contains("[[ $x"),
        "Shell [[ ]] needs space after '[[', got:\n{output}"
    );
    assert!(
        output.contains("0 ]]"),
        "Shell [[ ]] needs space before ']]', got:\n{output}"
    );
}
