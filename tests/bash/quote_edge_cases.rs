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
fn test_array_syntax() {
    let block = sigil_quote!(Bash {
        declare -a ITEMS;
        ITEMS=(one two three);
        echo $L("${ITEMS[@]}");
        echo $L("${#ITEMS[@]}");
    })
    .unwrap();
    golden::assert_golden("bash/quote_array.bash", &render(&block));
}

#[test]
fn test_parameter_expansion() {
    let block = sigil_quote!(Bash {
        DEFAULT=$L("${NAME:-guest}");
        UPPER=$L("${NAME^^}");
        SUBSTR=$L("${NAME:0:3}");
    })
    .unwrap();
    golden::assert_golden("bash/quote_parameter_expansion.bash", &render(&block));
}

#[test]
fn test_here_string() {
    let block = sigil_quote!(Bash {
        read -r line <<< $S("hello world");
        echo $L("$line");
    })
    .unwrap();
    golden::assert_golden("bash/quote_here_string.bash", &render(&block));
}

#[test]
fn test_function_with_local() {
    let block = sigil_quote!(Bash {
        function greet() {
            local name=$L("$1");
            echo $S("Hello, ${name}!");
        }
    })
    .unwrap();
    golden::assert_golden("bash/quote_function_local.bash", &render(&block));
}
