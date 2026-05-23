//! Tests for `$V` verbatim string literal in Python.

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

fn render(block: &CodeBlock) -> String {
    FileSpec::builder("test.py")
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

#[test]
fn verbatim_uses_fstring() {
    let block = sigil_quote!(Python {
        msg = $V("{name} is {age} years old")
    })
    .unwrap();
    let output = render(&block);
    assert!(
        output.contains("f\"{name} is {age} years old\""),
        "Expected f-string, got:\n{output}"
    );
}

// ── @{expr} interpolation ────────────────────────────────

#[test]
fn verbatim_at_interpolation_simple() {
    let module = "auth";
    let block = sigil_quote!(Python {
        msg = $V("@{module}: {user} logged in")
    })
    .unwrap();
    let output = render(&block);
    assert!(
        output.contains("f\"auth: {user} logged in\""),
        "got:\n{output}"
    );
}

#[test]
fn verbatim_at_interpolation_multiple() {
    let prefix = "api";
    let version = "v2";
    let block = sigil_quote!(Python {
        url = $V("@{prefix}/@{version}/{endpoint}")
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("f\"api/v2/{endpoint}\""), "got:\n{output}");
}

#[test]
fn verbatim_at_escape() {
    let block = sigil_quote!(Python {
        email = $V("user@@example.com")
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("f\"user@example.com\""), "got:\n{output}");
}

#[test]
fn verbatim_at_expr_method_call() {
    let items = ["a", "b", "c"];
    let block = sigil_quote!(Python {
        msg = $V("total=@{items.len()}")
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("f\"total=3\""), "got:\n{output}");
}

// ── $L with @{expr} (plain text, no wrapping) ────────────

#[test]
fn literal_at_interpolation_python() {
    let cls = "UserService";
    let block = sigil_quote!(Python {
        obj = $L("@{cls}()");
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("obj = UserService()"), "got:\n{output}");
    assert!(
        !output.contains("f\""),
        "$L should NOT produce f-string, got:\n{output}"
    );
}

// ── $attr() ──────────────────────────────────────────────

#[test]
fn attr_decorator_python() {
    let block = sigil_quote!(Python {
        $attr("classmethod")

        def from_dict(cls, data: dict):
            pass
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("@classmethod"), "got:\n{output}");
    assert!(
        !output.contains("@classmethod\n\n"),
        "blank line should be suppressed, got:\n{output}"
    );
}
