use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::java::Java;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

use super::golden;

fn render(block: &CodeBlock) -> String {
    FileSpec::builder_with("Test.java", Java::new())
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

#[test]
fn test_ternary() {
    let block = sigil_quote!(Java {
        String result = x != null ? x.toString() : "default";
        int value = flag ? 1 : 0;
    })
    .unwrap();
    golden::assert_golden("java/quote_ternary.java", &render(&block));
}

#[test]
fn test_generic_method() {
    let block = sigil_quote!(Java {
        public <T> T fromJson(String json, Class<T> clazz) {
            return gson.fromJson(json, clazz);
        }
    })
    .unwrap();
    golden::assert_golden("java/quote_generic_method.java", &render(&block));
}

#[test]
fn test_annotation() {
    let block = sigil_quote!(Java {
        @Override
        public String toString() {
            return $S("MyClass");
        }
    })
    .unwrap();
    golden::assert_golden("java/quote_annotation.java", &render(&block));
}

#[test]
fn test_stream_api() {
    let block = sigil_quote!(Java {
        List<String> names = users.stream()
            .filter(u -> u.isActive())
            .map(u -> u.getName())
            .collect(Collectors.toList());
    })
    .unwrap();
    golden::assert_golden("java/quote_stream.java", &render(&block));
}

#[test]
fn test_try_with_resources() {
    let block = sigil_quote!(Java {
        try (BufferedReader reader = new BufferedReader(new FileReader(path))) {
            String line = reader.readLine();
            System.out.println(line);
        }
    })
    .unwrap();
    golden::assert_golden("java/quote_try_resources.java", &render(&block));
}

#[test]
fn test_name_keyword_escape_in_macro() {
    let name = "class";
    let block = sigil_quote!(Java {
        $N(name) = 1;
    })
    .unwrap();

    let output = render(&block);
    assert!(output.contains("class_ = 1;"), "got: {output}");
}
