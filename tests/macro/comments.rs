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
