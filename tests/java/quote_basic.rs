use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::java_lang::JavaLang;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

use super::golden;

fn render(block: &CodeBlock) -> String {
    FileSpec::builder_with("Test.java", JavaLang::new())
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

#[test]
fn test_basic() {
    let block = sigil_quote!(JavaLang {
        String name = $S("Alice");
        int age = 30;
        System.out.println(name);
    })
    .unwrap();
    golden::assert_golden("java/macro_basic.java", &render(&block));
}

#[test]
fn test_override_method() {
    let block = sigil_quote!(JavaLang {
        @Override
        public String speak() {
            return "Woof!";
        }
    })
    .unwrap();
    golden::assert_golden("java/macro_override_method.java", &render(&block));
}

#[test]
fn test_generic_static_method() {
    let block = sigil_quote!(JavaLang {
        public static <T extends Comparable> List<T> sortList(List<T> list) {
            Collections.sort(list);
            return list;
        }
    })
    .unwrap();
    golden::assert_golden("java/macro_generic_static.java", &render(&block));
}
