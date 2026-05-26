use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::php::Php;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

fn render(block: &CodeBlock) -> String {
    FileSpec::builder_with("test.php", Php::new())
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

#[test]
fn test_imports() {
    let logger = TypeName::importable("Psr\\Log", "LoggerInterface");
    let request = TypeName::importable("Symfony\\Component\\HttpFoundation", "Request");
    let block = sigil_quote!(Php {
        $$logger = new $T(logger)();
        $$req = $T(request)::createFromGlobals();
    })
    .unwrap();
    golden::assert_golden("php/quote_imports.php", &render(&block));
}
