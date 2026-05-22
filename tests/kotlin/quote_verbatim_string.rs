//! Tests for `$V` verbatim string literal in Kotlin.

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

fn render(block: &CodeBlock) -> String {
    FileSpec::builder("test.kt")
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

#[test]
fn verbatim_preserves_dollar_interpolation() {
    let block = sigil_quote!(Kotlin {
        val msg = $V("Hello, ${name}!")
    })
    .unwrap();
    let output = render(&block);
    assert!(
        output.contains("\"Hello, ${name}!\""),
        "Expected Kotlin string interpolation preserved, got:\n{output}"
    );
}

// ── @{expr} interpolation ────────────────────────────────

#[test]
fn verbatim_at_interpolation_simple() {
    let greeting = "Hi";
    let block = sigil_quote!(Kotlin {
        val msg = $V("@{greeting}, ${name}!")
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("\"Hi, ${name}!\""), "got:\n{output}");
}

#[test]
fn verbatim_at_interpolation_multiple() {
    let base = "com.example";
    let module = "auth";
    let block = sigil_quote!(Kotlin {
        val pkg = $V("@{base}.@{module}")
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("\"com.example.auth\""), "got:\n{output}");
}

#[test]
fn verbatim_at_escape() {
    let block = sigil_quote!(Kotlin {
        val email = $V("user@@host.com")
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("\"user@host.com\""), "got:\n{output}");
}

#[test]
fn verbatim_at_mixed_with_kotlin_interpolation() {
    let prefix = "api";
    let block = sigil_quote!(Kotlin {
        val url = $V("@{prefix}/${version}/${endpoint}")
    })
    .unwrap();
    let output = render(&block);
    assert!(
        output.contains("\"api/${version}/${endpoint}\""),
        "got:\n{output}"
    );
}

// ── $L with @{expr} (plain text, no quotes) ──────────────

#[test]
fn literal_at_interpolation_kotlin() {
    let pkg = "com.example.auth";
    let block = sigil_quote!(Kotlin {
        val user = $L("@{pkg}.User()");
    })
    .unwrap();
    let output = render(&block);
    assert!(
        output.contains("val user = com.example.auth.User()"),
        "got:\n{output}"
    );
    assert!(
        !output.contains("\"com.example.auth.User()\""),
        "$L should NOT wrap in quotes, got:\n{output}"
    );
}
