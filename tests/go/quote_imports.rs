use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::go::Go;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

fn render(block: &CodeBlock) -> String {
    FileSpec::builder_with("test.go", Go::new())
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

#[test]
fn test_imports() {
    let fmt_println = TypeName::importable("fmt", "Println");
    let http_server = TypeName::importable("net/http", "Server");
    let block = sigil_quote!(Go {
        srv := &$T(http_server){};
        $T(fmt_println)(srv);
    })
    .unwrap();
    golden::assert_golden("go/quote_imports.go", &render(&block));
}
