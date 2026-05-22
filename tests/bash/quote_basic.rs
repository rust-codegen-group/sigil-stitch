use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

use super::golden;

fn render(block: &CodeBlock) -> String {
    FileSpec::builder("test.bash")
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

#[test]
fn test_basic() {
    crate::shared::run_basic_test::<super::quote_suite::BashSuite>();
}

#[test]
fn test_dollar_escape_no_space() {
    // `$$` followed by an identifier or number should NOT insert a space.
    let block = sigil_quote!(Bash {
        local level=$$1;
        echo $$level;
    })
    .unwrap();
    golden::assert_golden("bash/quote_dollar_escape.bash", &render(&block));
}
