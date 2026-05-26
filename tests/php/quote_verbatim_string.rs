//! Tests for `$V` verbatim string literal in PHP.

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

fn render(block: &CodeBlock) -> String {
    FileSpec::builder("test.php")
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

#[test]
fn verbatim_at_interpolation_simple() {
    let name = "world";
    let block = sigil_quote!(Php {
        $$msg = $V("hello @{name}")
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("\"hello world\""), "got:\n{output}");
}

#[test]
fn verbatim_at_interpolation_multiple() {
    let host = "localhost";
    let port = "8080";
    let block = sigil_quote!(Php {
        $$addr = $V("@{host}:@{port}")
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("\"localhost:8080\""), "got:\n{output}");
}

#[test]
fn verbatim_at_escape() {
    let block = sigil_quote!(Php {
        $$email = $V("user@@host.com")
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("\"user@host.com\""), "got:\n{output}");
}

#[test]
fn verbatim_escapes_dollar() {
    let block = sigil_quote!(Php {
        $$msg = $V("Hello $$name")
    })
    .unwrap();
    let output = render(&block);
    // $V renders verbatim, so $ inside is escaped as \$ in PHP double-quoted strings
    assert!(output.contains("\\$"), "got:\n{output}");
}
