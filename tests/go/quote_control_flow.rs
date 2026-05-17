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
fn test_control_flow() {
    let block = sigil_quote!(GoLang {
        if x > 0 {
            return $S("positive");
        } else {
            return $S("negative");
        }
    })
    .unwrap();
    golden::assert_golden("go/macro_control_flow.go", &render(&block));
}

#[test]
fn test_if_init_semicolon() {
    let block = sigil_quote!(GoLang {
        if err := doStuff(); err != nil {
            return err;
        }
    })
    .unwrap();
    golden::assert_golden("go/quote_if_init.go", &render(&block));
}

#[test]
fn test_for_init_semicolon() {
    let block = sigil_quote!(GoLang {
        for i := 0; i < 10; i++ {
            fmt.Println(i);
        }
    })
    .unwrap();
    golden::assert_golden("go/quote_for_init.go", &render(&block));
}

#[test]
fn test_switch_init_semicolon() {
    let block = sigil_quote!(GoLang {
        switch v := getValue(); v {
        case "hello":
            fmt.Println(v);
        }
    })
    .unwrap();
    golden::assert_golden("go/quote_switch_init.go", &render(&block));
}
