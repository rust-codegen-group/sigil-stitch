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
    let block = sigil_quote!(JavaScript {
        const name = $S("Alice");
        const age = $L("30");
        console.log(name, age);
    })
    .unwrap();
    golden::assert_golden("javascript/macro_basic.js", &render(&block));
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
