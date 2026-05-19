//! Tests that expose bugs in Bash control-flow rendering:
//! - Bracket groups `[ ... ]` need inner spaces in shell languages

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

fn render_block(block: &CodeBlock) -> String {
    FileSpec::builder("test.bash")
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

/// The `sigil_quote!` macro with bracket groups `[ ... ]` should produce
/// spaces inside the brackets for shell test expressions.
#[test]
fn bash_sigil_quote_bracket_spacing() {
    let block = sigil_quote!(Bash {
        if [ $L("$x") -gt 0 ] {
            echo $S("positive");
        } else {
            echo $S("negative");
        }
    })
    .unwrap();
    let output = render_block(&block);

    // The `[` test command requires a space after it and before `]`
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
        "Missing space after '[' is a bug, got:\n{output}"
    );
}
