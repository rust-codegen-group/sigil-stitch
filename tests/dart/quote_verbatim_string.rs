//! Tests for `$V` verbatim string literal in Dart.

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

fn render(block: &CodeBlock) -> String {
    FileSpec::builder("test.dart")
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

#[test]
fn verbatim_preserves_dollar_interpolation() {
    let block = sigil_quote!(Dart {
        var msg = $V("Hello, ${name}!")
    })
    .unwrap();
    let output = render(&block);
    assert!(
        output.contains("'Hello, ${name}!'"),
        "Expected Dart interpolation preserved, got:\n{output}"
    );
}

// ── @{expr} interpolation ────────────────────────────────

#[test]
fn verbatim_at_interpolation_simple() {
    let greeting = "Hi";
    let block = sigil_quote!(Dart {
        var msg = $V("@{greeting}, ${name}!")
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("'Hi, ${name}!'"), "got:\n{output}");
}

#[test]
fn verbatim_at_interpolation_multiple() {
    let base = "com.example";
    let module = "auth";
    let block = sigil_quote!(Dart {
        var pkg = $V("@{base}.@{module}")
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("'com.example.auth'"), "got:\n{output}");
}

#[test]
fn verbatim_at_escape() {
    let block = sigil_quote!(Dart {
        var email = $V("user@@host.com")
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("'user@host.com'"), "got:\n{output}");
}
