//! Tests for `$V` verbatim string literal in Go.
//!
//! Go has no string interpolation, so $V falls back to $S behavior (full escaping).
//! @{expr} still works at the macro level — it's compile-time interpolation.

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

fn render(block: &CodeBlock) -> String {
    FileSpec::builder("test.go")
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

// ── @{expr} interpolation ────────────────────────────────

#[test]
fn verbatim_at_interpolation_simple() {
    let name = "world";
    let block = sigil_quote!(Go {
        msg := $V("hello @{name}")
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("\"hello world\""), "got:\n{output}");
}

#[test]
fn verbatim_at_interpolation_multiple() {
    let host = "localhost";
    let port = "8080";
    let block = sigil_quote!(Go {
        addr := $V("@{host}:@{port}")
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("\"localhost:8080\""), "got:\n{output}");
}

#[test]
fn verbatim_at_escape() {
    let block = sigil_quote!(Go {
        email := $V("user@@host.com")
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("\"user@host.com\""), "got:\n{output}");
}
