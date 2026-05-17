use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::type_name::TypeName;

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
fn test_imports() {
    let log_fn = TypeName::importable("./lib/log.sh", "log_info");
    let config_fn = TypeName::importable("./lib/config.sh", "load_config");
    let block = sigil_quote!(Bash {
        $T(config_fn);
        $T(log_fn) $S("started");
    })
    .unwrap();
    golden::assert_golden("bash/quote_imports.bash", &render(&block));
}
