use super::golden;
use super::helpers::*;

#[test]
fn test_comment_with_semicolon() {
    let block = sigil_quote!(TypeScript {
        $comment("Initialize the value");
        const x = 0;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("// Initialize the value"), "got: {output}");
    assert!(output.contains("const x = 0;"), "got: {output}");
}

#[test]
fn test_comment_without_semicolon() {
    let block = sigil_quote!(TypeScript {
        $comment("no semicolon")
        const x = 0;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("// no semicolon"), "got: {output}");
    assert!(output.contains("const x = 0;"), "got: {output}");
}

#[test]
fn test_comment_only() {
    let block = sigil_quote!(TypeScript {
        $comment("just a comment");
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("// just a comment"), "got: {output}");
}

#[test]
fn test_multiple_comments() {
    let block = sigil_quote!(TypeScript {
        $comment("line 1");
        $comment("line 2");
        const x = 0;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("// line 1"), "got: {output}");
    assert!(output.contains("// line 2"), "got: {output}");
}

#[test]
fn test_comment_with_newline_escape() {
    let block = sigil_quote!(TypeScript {
        $comment("first line\nsecond line");
        const x = 1;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(
        output.contains("// first line\nsecond line"),
        "got: {output}"
    );
}

#[test]
fn test_comment_with_tab_escape() {
    let block = sigil_quote!(TypeScript {
        $comment("indented\ttab");
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("indented\ttab"), "got: {output}");
}

#[test]
fn test_comment_with_backslash_escape() {
    let block = sigil_quote!(TypeScript {
        $comment("path\\to\\file");
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("path\\to\\file"), "got: {output}");
}

#[test]
fn test_comment_golden() {
    let block = sigil_quote!(TypeScript {
        $comment("Initialize values");
        const x = 0;
        $comment("Process result");
        const y = x + 1;
    })
    .unwrap();

    let output = render_ts(&block);
    golden::assert_golden("macro/quote_comment.txt", &output);
}

// ── Comment attachment (no blank line after comment) ──────

#[test]
fn test_comment_attaches_to_declaration_after_blank_line() {
    // A blank line in the macro body (for readability) should NOT
    // produce a blank line between the comment and the declaration.
    let block = sigil_quote!(TypeScript {
        $comment("Doc comment for Foo")

        const x = 0;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("// Doc comment for Foo"), "got:\n{output}");
    assert!(output.contains("const x = 0;"), "got:\n{output}");
    // The comment must attach directly — no blank line
    assert!(
        !output.contains("// Doc comment for Foo\n\n"),
        "blank line should be suppressed after comment, got:\n{output}"
    );
}

// ── $attr() — language-aware attributes ──────────────────

#[test]
fn test_attr_basic_typescript() {
    let block = sigil_quote!(TypeScript {
        $attr("override")

        process(): void {
            return;
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("@override"), "got:\n{output}");
    assert!(
        !output.contains("@override\n\n"),
        "blank line should be suppressed after attribute, got:\n{output}"
    );
}

#[test]
fn test_attr_basic_rust() {
    let block = sigil_quote!(RustLang {
        $attr("derive(Debug, Clone)")

        struct Foo;
    })
    .unwrap();

    let output = render_rs(&block);
    assert!(output.contains("#[derive(Debug, Clone)]"), "got:\n{output}");
}

#[test]
fn test_attr_multiple() {
    let block = sigil_quote!(TypeScript {
        $attr("injectable()")
        $attr("singleton()")

        class MyService {}
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("@injectable()"), "got:\n{output}");
    assert!(output.contains("@singleton()"), "got:\n{output}");
}

#[test]
fn test_attr_cpp_double_bracket() {
    let block = sigil_quote!(CppLang {
        $attr("nodiscard")

        int getValue() {
            return 42;
        }
    })
    .unwrap();

    let output = render_cpp(&block);
    assert!(output.contains("[[nodiscard]]"), "got:\n{output}");
}

#[test]
fn test_attr_rust_derive() {
    let block = sigil_quote!(RustLang {
        $attr("derive(Debug, Clone, Serialize, Deserialize)")

        pub struct $N("User") {
            name: String,
        }
    })
    .unwrap();

    let output = render_rs(&block);
    assert!(
        output.contains("#[derive(Debug, Clone, Serialize, Deserialize)]"),
        "got:\n{output}"
    );
}

#[test]
fn test_attr_with_if_conditional() {
    let needs_serde = true;
    let block = sigil_quote!(RustLang {
        $attr("derive(Debug, Clone)")

        $if(needs_serde) {
            $attr("serde(rename_all = \"camelCase\")")
        }

        struct $N("Config") {
            name: String,
        }
    })
    .unwrap();

    let output = render_rs(&block);
    assert!(output.contains("#[derive(Debug, Clone)]"), "got:\n{output}");
    assert!(
        output.contains("#[serde(rename_all = \"camelCase\")]"),
        "got:\n{output}"
    );
}
