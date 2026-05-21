//! Tests for `$V` verbatim string literal in Swift.
//!
//! Swift's `render_verbatim_string` escapes `\` and `"` — this means `\(name)`
//! becomes `\\(name)` in the output (not working Swift interpolation).
//! Use `$V` in Swift for strings where you want minimal escaping but don't need
//! the interpolation sigil preserved.

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

fn render(block: &CodeBlock) -> String {
    FileSpec::builder("test.swift")
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

#[test]
fn verbatim_escapes_backslash() {
    let block = sigil_quote!(Swift {
        let msg = $V("Hello, \\(name)!")
    })
    .unwrap();
    let output = render(&block);
    // Swift render_verbatim_string escapes \ to \\, so \(name) becomes \\(name)
    assert!(
        output.contains("\"Hello, \\\\(name)!\""),
        "Expected backslash to be escaped, got:\n{output}"
    );
}

// ── @{expr} interpolation ────────────────────────────────

#[test]
fn verbatim_at_interpolation_simple() {
    let greeting = "Hi";
    let block = sigil_quote!(Swift {
        let msg = $V("@{greeting} there")
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("\"Hi there\""), "got:\n{output}");
}

#[test]
fn verbatim_at_interpolation_multiple() {
    let base = "com.example";
    let module = "auth";
    let block = sigil_quote!(Swift {
        let pkg = $V("@{base}.@{module}")
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("\"com.example.auth\""), "got:\n{output}");
}

#[test]
fn verbatim_at_escape() {
    let block = sigil_quote!(Swift {
        let email = $V("user@@host.com")
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("\"user@host.com\""), "got:\n{output}");
}

#[test]
fn verbatim_at_mixed_with_backslash() {
    let greeting = "Hi";
    let block = sigil_quote!(Swift {
        let msg = $V("@{greeting}, \\(name)!")
    })
    .unwrap();
    let output = render(&block);
    // @{greeting} resolves to "Hi", then \(name) gets backslash-escaped by Swift renderer
    assert!(output.contains("\"Hi, \\\\(name)!\""), "got:\n{output}");
}
