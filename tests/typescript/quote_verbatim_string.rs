//! Tests for `$V` verbatim string literal in TypeScript.

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

fn render(block: &CodeBlock) -> String {
    FileSpec::builder("test.ts")
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

#[test]
fn verbatim_uses_template_literal() {
    let block = sigil_quote!(TypeScript {
        const msg = $V("Hello, ${name}!")
    })
    .unwrap();
    let output = render(&block);
    assert!(
        output.contains("`Hello, ${name}!`"),
        "Expected template literal with interpolation, got:\n{output}"
    );
}

#[test]
fn verbatim_escapes_backtick() {
    let block = sigil_quote!(TypeScript {
        const msg = $V("use \\` for templates")
    })
    .unwrap();
    let output = render(&block);
    // Input: "use \` for templates" → escapes \ to \\ and ` to \`
    assert!(
        output.contains("`use \\\\\\` for templates`"),
        "Expected escaped backtick in template literal, got:\n{output}"
    );
}

// ── @{expr} interpolation ────────────────────────────────

#[test]
fn verbatim_at_interpolation_simple() {
    let greeting = "Hello";
    let block = sigil_quote!(TypeScript {
        const msg = $V("@{greeting}, ${name}!")
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("`Hello, ${name}!`"), "got:\n{output}");
}

#[test]
fn verbatim_at_interpolation_multiple() {
    let base_url = "https://api.example.com";
    let version = "v2";
    let block = sigil_quote!(TypeScript {
        const url = $V("@{base_url}/@{version}/${endpoint}")
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
    let block = sigil_quote!(TypeScript {
        const email = $V("user@@example.com")
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("`user@example.com`"), "got:\n{output}");
}

#[test]
fn verbatim_at_expr_method_call() {
    let items = ["a", "b", "c"];
    let block = sigil_quote!(TypeScript {
        const msg = $V("total: @{items.len()} items")
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("`total: 3 items`"), "got:\n{output}");
}

// ── $L with @{expr} (plain text, no backticks) ────────────

#[test]
fn literal_at_interpolation_plain_text() {
    let disc = "foo.bar";
    let block = sigil_quote!(TypeScript {
        switch ($L("@{disc}")) {
            $L("case 1:") {
                break;
            }
        }
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("switch (foo.bar)"), "got:\n{output}");
    assert!(
        !output.contains("`foo.bar`"),
        "$L should NOT wrap in backticks, got:\n{output}"
    );
}

#[test]
fn literal_at_vs_verbatim_at_typescript() {
    let name = "Alice";
    let block = sigil_quote!(TypeScript {
        const a = $V("Hello, @{name}");
        const b = $L("@{name}");
    })
    .unwrap();
    let output = render(&block);
    // $V wraps in backticks
    assert!(
        output.contains("const a = `Hello, Alice`;"),
        "got:\n{output}"
    );
    // $L does NOT wrap
    assert!(output.contains("const b = Alice;"), "got:\n{output}");
}
