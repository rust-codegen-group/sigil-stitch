//! Tests for `$V` verbatim string literal in Scala.

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

fn render(block: &CodeBlock) -> String {
    FileSpec::builder("test.scala")
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

#[test]
fn verbatim_uses_s_string() {
    let block = sigil_quote!(Scala {
        val msg = $V("Hello, ${name}!")
    })
    .unwrap();
    let output = render(&block);
    assert!(
        output.contains("s\"Hello, ${name}!\""),
        "Expected s-string interpolation, got:\n{output}"
    );
}

// ── @{expr} interpolation ────────────────────────────────

#[test]
fn verbatim_at_interpolation_simple() {
    let greeting = "Hi";
    let block = sigil_quote!(Scala {
        val msg = $V("@{greeting}, ${name}!")
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("s\"Hi, ${name}!\""), "got:\n{output}");
}

#[test]
fn verbatim_at_interpolation_multiple() {
    let base = "com.example";
    let module = "auth";
    let block = sigil_quote!(Scala {
        val pkg = $V("@{base}.@{module}")
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("s\"com.example.auth\""), "got:\n{output}");
}

#[test]
fn verbatim_at_escape() {
    let block = sigil_quote!(Scala {
        val email = $V("user@@host.com")
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("s\"user@host.com\""), "got:\n{output}");
}
