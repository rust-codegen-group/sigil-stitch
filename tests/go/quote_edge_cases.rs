use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::go_lang::GoLang;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

use super::golden;

fn render(block: &CodeBlock) -> String {
    FileSpec::builder_with("test.go", GoLang::new())
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

#[test]
fn test_indent() {
    let block = sigil_quote!(GoLang {
        func printDirections() {
        $>
        fmt.Println("North");
        fmt.Println("East");
        fmt.Println("South");
        fmt.Println("West");
        $<
        }
    })
    .unwrap();
    golden::assert_golden("go/macro_indent.go", &render(&block));
}

#[test]
fn test_name_escape_in_macro() {
    let name = "type";
    let block = sigil_quote!(GoLang {
        var $N(name) string
    })
    .unwrap();

    let output = render(&block);
    assert!(
        output.contains("var type_ string"),
        "Expected 'var type_ string', got: {output}"
    );
    golden::assert_golden("go/quote_keyword_escape.go", &output);
}

#[test]
fn test_name_escape_multiple_keywords_in_macro() {
    let pkg = "package";
    let ret = "return";
    let block = sigil_quote!(GoLang {
        $N(pkg) = $N(ret)
    })
    .unwrap();

    let output = render(&block);
    assert!(output.contains("package_"), "Expected 'package_': {output}");
    assert!(output.contains("return_"), "Expected 'return_': {output}");
    golden::assert_golden("go/quote_keyword_escape_multi.go", &output);
}

#[test]
fn test_name_no_escape_in_macro() {
    let name = "myHandler";
    let block = sigil_quote!(GoLang {
        func $N(name)()
    })
    .unwrap();

    let output = render(&block);
    assert!(
        output.contains("func myHandler()"),
        "Expected 'func myHandler()', got: {output}"
    );
    golden::assert_golden("go/quote_no_escape.go", &output);
}

#[test]
fn test_goroutine() {
    let block = sigil_quote!(GoLang {
        go func() {
            fmt.Println($S("hello from goroutine"));
        }();
    })
    .unwrap();
    golden::assert_golden("go/quote_goroutine.go", &render(&block));
}

#[test]
fn test_channel() {
    let block = sigil_quote!(GoLang {
        ch := make(chan int, 10);
        ch <- 42;
        val := <-ch;
    })
    .unwrap();
    golden::assert_golden("go/quote_channel.go", &render(&block));
}

#[test]
fn test_interface() {
    let block = sigil_quote!(GoLang {
        type Reader interface {
            Read(p []byte) (int, error);
        }
    })
    .unwrap();
    golden::assert_golden("go/quote_interface.go", &render(&block));
}

#[test]
fn test_defer() {
    let block = sigil_quote!(GoLang {
        f, err := os.Open(path);
        defer f.Close();
    })
    .unwrap();
    golden::assert_golden("go/quote_defer.go", &render(&block));
}

// ── Channel receive: tokenizer-level <- fix ─────────────

#[test]
fn test_channel_receive_tight() {
    let block = sigil_quote!(GoLang { val := <-ch; }).unwrap();
    let output = render(&block);
    assert!(
        output.contains("<-ch"),
        "receive should be tight, got: {output}"
    );
    assert!(
        !output.contains("<- ch"),
        "no space after <- in receive, got: {output}"
    );
}

#[test]
fn test_channel_send_has_space() {
    let block = sigil_quote!(GoLang { ch <- 42; }).unwrap();
    let output = render(&block);
    assert!(
        output.contains("<- 42") || output.contains("ch <- 42"),
        "send should keep space, got: {output}"
    );
}

#[test]
fn test_channel_receive_standalone() {
    let block = sigil_quote!(GoLang { <-done; }).unwrap();
    let output = render(&block);
    assert!(
        output.contains("<-done"),
        "standalone receive should be tight, got: {output}"
    );
}
