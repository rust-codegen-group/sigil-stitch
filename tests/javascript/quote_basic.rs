use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::javascript::JavaScript;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

use super::golden;

fn render(block: &CodeBlock) -> String {
    FileSpec::builder_with("test.js", JavaScript::new())
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

#[test]
fn test_basic() {
    crate::shared::run_basic_test::<super::quote_suite::JavaScriptSuite>();
}

#[test]
fn test_postfix_increment() {
    let block = sigil_quote!(JavaScript {
        for (let i = 0; i < 10; i++) {
            count++;
        }
    })
    .unwrap();
    golden::assert_golden("javascript/quote_postfix_increment.js", &render(&block));
}
