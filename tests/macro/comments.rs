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
        output.contains("// first line\n// second line"),
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

// ── $comment(expr) — dynamic expressions ──────────────────

#[test]
fn test_comment_with_dynamic_expression() {
    let msg = "dynamic comment";
    let block = sigil_quote!(TypeScript {
        $comment(msg);
        const x = 0;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("// dynamic comment"), "got: {output}");
}

#[test]
fn test_comment_with_format_expression() {
    let name = "Foo";
    let block = sigil_quote!(TypeScript {
        $comment(format!("Class: {name}"));
        const x = 0;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("// Class: Foo"), "got: {output}");
}

#[test]
fn test_comment_with_to_string_expression() {
    let code = 200;
    let block = sigil_quote!(TypeScript {
        $comment(code.to_string());
        const x = 0;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("// 200"), "got: {output}");
}

// ── $comment(expr) — inline interpolation ──────────────────

#[test]
fn test_comment_inline_within_statement() {
    let msg = "cleanup";
    let block = sigil_quote!(TypeScript {
        doStuff($S("x")) $comment(msg)
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("doStuff('x') // cleanup"), "got: {output}");
}

#[test]
fn test_comment_inline_with_format() {
    let name = "validate";
    let block = sigil_quote!(TypeScript {
        process($S("y")) $comment(format!("TODO: {name}"))
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("// TODO: validate"), "got: {output}");
}

// ── $comment(expr) — @{…} interpolation ──────────────────

#[test]
fn test_comment_with_at_interpolation() {
    let name = "World";
    let block = sigil_quote!(TypeScript {
        $comment("Hello @{name}");
        const x = 0;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("// Hello World"), "got: {output}");
}

#[test]
fn test_comment_with_at_interpolation_inline() {
    let count = 42;
    let block = sigil_quote!(TypeScript {
        doStuff() $comment("processed @{count} items")
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("// processed 42 items"), "got: {output}");
}

#[test]
fn test_comment_with_double_at_escape() {
    let block = sigil_quote!(TypeScript {
        $comment("user@@host");
        const x = 0;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("// user@host"), "got: {output}");
}

// ── $attr(expr) — dynamic expressions ───────────────────

#[test]
fn test_attr_with_dynamic_expression() {
    let attr_name = "override";
    let block = sigil_quote!(TypeScript {
        $attr(attr_name);

        process(): void {
            return;
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("@override"), "got:\n{output}");
}

#[test]
fn test_attr_with_format_expression() {
    let name = "Debug";
    let block = sigil_quote!(Rust {
        $attr(format!("derive({name}, Clone)"));

        struct Foo;
    })
    .unwrap();

    let output = render_rs(&block);
    assert!(output.contains("#[derive(Debug, Clone)]"), "got:\n{output}");
}

#[test]
fn test_attr_with_to_string_expression() {
    let code = 200;
    let block = sigil_quote!(TypeScript {
        $attr(code.to_string());

        process(): void {
            return;
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("@200"), "got:\n{output}");
}

// ── $attr(expr) — @{…} interpolation ────────────────────

#[test]
fn test_attr_with_at_interpolation() {
    let trait_name = "Debug";
    let block = sigil_quote!(Rust {
        $attr("derive(@{trait_name}, Clone)");

        struct Foo;
    })
    .unwrap();

    let output = render_rs(&block);
    assert!(output.contains("#[derive(Debug, Clone)]"), "got:\n{output}");
}

#[test]
fn test_attr_with_double_at_escape() {
    let block = sigil_quote!(TypeScript {
        $attr("user@@host");

        process(): void {
            return;
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("@user@host"), "got:\n{output}");
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
    let block = sigil_quote!(Rust {
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
    let block = sigil_quote!(Cpp {
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
    let block = sigil_quote!(Rust {
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
    let block = sigil_quote!(Rust {
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
