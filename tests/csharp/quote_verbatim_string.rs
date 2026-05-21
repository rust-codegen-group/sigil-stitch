//! Tests for `$V` verbatim string literal in C#.

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

fn render(block: &CodeBlock) -> String {
    FileSpec::builder("test.cs")
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

#[test]
fn verbatim_uses_interpolated_string() {
    let block = sigil_quote!(CSharp {
        var msg = $V("Hello, {name}!")
    })
    .unwrap();
    let output = render(&block);
    assert!(
        output.contains("$\"Hello, {name}!\""),
        "Expected C# interpolated string, got:\n{output}"
    );
}

// ── @{expr} interpolation ────────────────────────────────

#[test]
fn verbatim_at_interpolation_simple() {
    let greeting = "Hi";
    let block = sigil_quote!(CSharp {
        var msg = $V("@{greeting}, {name}!")
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("$\"Hi, {name}!\""), "got:\n{output}");
}

#[test]
fn verbatim_at_interpolation_multiple() {
    let ns = "MyApp";
    let module = "Auth";
    let block = sigil_quote!(CSharp {
        var pkg = $V("@{ns}.@{module}")
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("$\"MyApp.Auth\""), "got:\n{output}");
}

#[test]
fn verbatim_at_escape() {
    let block = sigil_quote!(CSharp {
        var email = $V("user@@host.com")
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("$\"user@host.com\""), "got:\n{output}");
}
