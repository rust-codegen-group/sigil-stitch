use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

use super::golden;

fn render(block: &CodeBlock) -> String {
    FileSpec::builder("test.py")
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

#[test]
fn test_basic() {
    crate::shared::run_basic_test::<super::quote_suite::PythonSuite>();
}

#[test]
fn test_classmethod_decorator() {
    let block = sigil_quote!(Python {
        @classmethod
        def from_dict(cls, data: dict) -> "User": {
            return cls(data["name"], data["age"])
        }
    })
    .unwrap();
    golden::assert_golden("python/macro_classmethod.py", &render(&block));
}
