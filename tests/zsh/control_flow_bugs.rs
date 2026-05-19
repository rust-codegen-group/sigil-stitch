//! Tests that expose bugs in Zsh control-flow rendering:
//! - begin_control_flow should emit `then`/`fi`/`do`/`done`, not `{`/`}`
//! - Bracket groups `[ ... ]` need inner spaces in shell languages

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::zsh::Zsh;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

fn render_block(block: &CodeBlock) -> String {
    FileSpec::builder("test.zsh")
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

/// Using `begin_control_flow` with Zsh should emit `; then` / `fi`,
/// not `{` / `}`. Currently fails because Zsh is missing
/// `block_open_for` / `block_close_for` overrides.
#[test]
fn zsh_if_then_fi_via_begin_control_flow() {
    let mut b = CodeBlock::builder();
    b.begin_control_flow("if [ -f \"$1\" ]", ());
    b.add_statement("echo \"exists\"", ());
    b.end_control_flow();
    let block = b.build().unwrap();

    let zsh = Zsh::new();
    let imports = sigil_stitch::import::ImportGroup::new();
    let mut renderer =
        sigil_stitch::code_renderer::CodeRenderer::new(&zsh, &imports, 80);
    let output = renderer.render(&block).unwrap();

    assert!(
        output.contains("; then"),
        "Zsh if-block should use '; then', got:\n{output}"
    );
    assert!(
        output.contains("fi"),
        "Zsh if-block should close with 'fi', got:\n{output}"
    );
    assert!(
        !output.contains('{'),
        "Zsh if-block should NOT use '{{', got:\n{output}"
    );
}

/// Zsh for-loop should emit `; do` / `done`.
#[test]
fn zsh_for_do_done_via_begin_control_flow() {
    let mut b = CodeBlock::builder();
    b.begin_control_flow("for f in *.txt", ());
    b.add_statement("echo \"$f\"", ());
    b.end_control_flow();
    let block = b.build().unwrap();

    let zsh = Zsh::new();
    let imports = sigil_stitch::import::ImportGroup::new();
    let mut renderer =
        sigil_stitch::code_renderer::CodeRenderer::new(&zsh, &imports, 80);
    let output = renderer.render(&block).unwrap();

    assert!(
        output.contains("; do"),
        "Zsh for-loop should use '; do', got:\n{output}"
    );
    assert!(
        output.contains("done"),
        "Zsh for-loop should close with 'done', got:\n{output}"
    );
}

/// Zsh while-loop should emit `; do` / `done`.
#[test]
fn zsh_while_do_done_via_begin_control_flow() {
    let mut b = CodeBlock::builder();
    b.begin_control_flow("while read -r line", ());
    b.add_statement("echo \"$line\"", ());
    b.end_control_flow();
    let block = b.build().unwrap();

    let zsh = Zsh::new();
    let imports = sigil_stitch::import::ImportGroup::new();
    let mut renderer =
        sigil_stitch::code_renderer::CodeRenderer::new(&zsh, &imports, 80);
    let output = renderer.render(&block).unwrap();

    assert!(
        output.contains("; do"),
        "Zsh while-loop should use '; do', got:\n{output}"
    );
    assert!(
        output.contains("done"),
        "Zsh while-loop should close with 'done', got:\n{output}"
    );
}

/// Zsh if/else via `begin_control_flow` + `next_control_flow` should NOT
/// emit `} else {` — it should just emit `else` between fi-less blocks.
#[test]
fn zsh_if_else_via_control_flow_api() {
    let mut b = CodeBlock::builder();
    b.begin_control_flow("if [ -n \"$1\" ]", ());
    b.add_statement("echo \"yes\"", ());
    b.next_control_flow("else", ());
    b.add_statement("echo \"no\"", ());
    b.end_control_flow();
    let block = b.build().unwrap();

    let zsh = Zsh::new();
    let imports = sigil_stitch::import::ImportGroup::new();
    let mut renderer =
        sigil_stitch::code_renderer::CodeRenderer::new(&zsh, &imports, 80);
    let output = renderer.render(&block).unwrap();

    assert!(
        !output.contains("} else {"),
        "Zsh should NOT emit '}} else {{', got:\n{output}"
    );
    assert!(
        output.contains("else"),
        "Zsh if/else should contain 'else', got:\n{output}"
    );
    assert!(
        output.contains("fi"),
        "Zsh if/else should close with 'fi', got:\n{output}"
    );
}

/// The `sigil_quote!` macro with bracket groups `[ ... ]` should produce
/// spaces inside the brackets for shell test expressions.
#[test]
fn zsh_sigil_quote_bracket_spacing() {
    let block = sigil_quote!(Zsh {
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
