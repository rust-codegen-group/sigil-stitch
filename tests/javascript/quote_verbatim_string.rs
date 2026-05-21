//! Tests for `$V` verbatim string literal in JavaScript.

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

fn render(block: &CodeBlock) -> String {
    FileSpec::builder("test.js")
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

#[test]
fn verbatim_uses_template_literal() {
    let block = sigil_quote!(JavaScript {
        const msg = $V("Hello, ${name}!")
    })
    .unwrap();
    let output = render(&block);
    assert!(
        output.contains("`Hello, ${name}!`"),
        "Expected template literal, got:\n{output}"
    );
}

// ── @{expr} interpolation ────────────────────────────────

#[test]
fn verbatim_at_interpolation_simple() {
    let greeting = "Hello";
    let block = sigil_quote!(JavaScript {
        const msg = $V("@{greeting}, ${name}!")
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("`Hello, ${name}!`"), "got:\n{output}");
}

#[test]
fn verbatim_at_interpolation_multiple() {
    let base = "https://api.example.com";
    let ver = "v2";
    let block = sigil_quote!(JavaScript {
        const url = $V("@{base}/@{ver}/${endpoint}")
    })
    .unwrap();
    let output = render(&block);
    assert!(
        output.contains("`https://api.example.com/v2/${endpoint}`"),
        "got:\n{output}"
    );
}

#[test]
fn verbatim_at_escape() {
    let block = sigil_quote!(JavaScript {
        const email = $V("user@@example.com")
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("`user@example.com`"), "got:\n{output}");
}

#[test]
fn verbatim_at_expr_method_call() {
    let items = ["a", "b"];
    let block = sigil_quote!(JavaScript {
        const msg = $V("count=@{items.len()}")
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("`count=2`"), "got:\n{output}");
}
