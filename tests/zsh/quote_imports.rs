use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

fn render(block: &CodeBlock) -> String {
    FileSpec::builder("test.zsh")
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

#[test]
fn test_imports() {
    let log_fn = TypeName::importable("./lib/log.zsh", "log_info");
    let config_fn = TypeName::importable("./lib/config.zsh", "get_config");
    let block = sigil_quote!(Zsh {
        $T(config_fn) --reload;
        $T(log_fn) $S("config loaded");
    })
    .unwrap();
    golden::assert_golden("zsh/quote_imports.zsh", &render(&block));
}
