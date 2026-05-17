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
    let block = sigil_quote!(Python {
        name = $S("Alice");
        age = 30;
        print(name, age);
    })
    .unwrap();
    golden::assert_golden("python/macro_basic.py", &render(&block));
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
