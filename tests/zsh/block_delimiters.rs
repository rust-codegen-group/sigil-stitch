//! Tests for Zsh block delimiter rendering via `begin_control_flow` API.
//! Zsh should emit `then`/`fi`, `do`/`done` — same as Bash.

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::zsh::Zsh;

#[test]
fn if_then_fi() {
    let mut b = CodeBlock::builder();
    b.begin_control_flow("if [ -f \"$1\" ]", ());
    b.add_statement("echo \"exists\"", ());
    b.end_control_flow();
    let block = b.build().unwrap();

    let zsh = Zsh::new();
    let imports = sigil_stitch::import::ImportGroup::new();
    let mut renderer = sigil_stitch::code_renderer::CodeRenderer::new(&zsh, &imports, 80);
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
        "Zsh if-block should NOT use braces, got:\n{output}"
    );
}

#[test]
fn for_do_done() {
    let mut b = CodeBlock::builder();
    b.begin_control_flow("for f in *.txt", ());
    b.add_statement("echo \"$f\"", ());
    b.end_control_flow();
    let block = b.build().unwrap();

    let zsh = Zsh::new();
    let imports = sigil_stitch::import::ImportGroup::new();
    let mut renderer = sigil_stitch::code_renderer::CodeRenderer::new(&zsh, &imports, 80);
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

#[test]
fn while_do_done() {
    let mut b = CodeBlock::builder();
    b.begin_control_flow("while read -r line", ());
    b.add_statement("echo \"$line\"", ());
    b.end_control_flow();
    let block = b.build().unwrap();

    let zsh = Zsh::new();
    let imports = sigil_stitch::import::ImportGroup::new();
    let mut renderer = sigil_stitch::code_renderer::CodeRenderer::new(&zsh, &imports, 80);
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

#[test]
fn if_else_fi() {
    let mut b = CodeBlock::builder();
    b.begin_control_flow("if [ -n \"$1\" ]", ());
    b.add_statement("echo \"yes\"", ());
    b.next_control_flow("else", ());
    b.add_statement("echo \"no\"", ());
    b.end_control_flow();
    let block = b.build().unwrap();

    let zsh = Zsh::new();
    let imports = sigil_stitch::import::ImportGroup::new();
    let mut renderer = sigil_stitch::code_renderer::CodeRenderer::new(&zsh, &imports, 80);
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
