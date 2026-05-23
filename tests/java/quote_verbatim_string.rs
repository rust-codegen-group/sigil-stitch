//! Tests for `$V` verbatim string literal in Java.
//!
//! Java has no string interpolation, so $V falls back to $S behavior (full escaping).
//! @{expr} still works at the macro level — it's compile-time interpolation.

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

fn render(block: &CodeBlock) -> String {
    FileSpec::builder("test.java")
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
    let block = sigil_quote!(Java {
        String msg = $V("hello @{name}")
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("\"hello world\""), "got:\n{output}");
}

#[test]
fn verbatim_at_interpolation_multiple() {
    let pkg = "com.example";
    let cls = "Main";
    let block = sigil_quote!(Java {
        String fqn = $V("@{pkg}.@{cls}")
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("\"com.example.Main\""), "got:\n{output}");
}

// ── $attr() ──────────────────────────────────────────────

#[test]
fn attr_annotation_java() {
    let block = sigil_quote!(JavaLang {
        $attr("Override")

        public String toString() {
            return $S("hello");
        }
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("@Override"), "got:\n{output}");
}

#[test]
fn verbatim_at_escape() {
    let block = sigil_quote!(Java {
        String email = $V("user@@host.com")
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("\"user@host.com\""), "got:\n{output}");
}
