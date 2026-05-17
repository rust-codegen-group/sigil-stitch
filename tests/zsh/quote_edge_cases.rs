use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

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
fn test_percent_escape_in_quote() {
    let block = sigil_quote!(Zsh {
        PROMPT=$S("%F{green}%n@%m%f");
        echo $L("$PROMPT");
    })
    .unwrap();
    golden::assert_golden("zsh/quote_percent_escape.zsh", &render(&block));
}

#[test]
fn test_variable_expansion() {
    let block = sigil_quote!(Zsh {
        DEFAULT=$L("${NAME:-guest}");
        LENGTH=$L("${#ITEMS[@]}");
        echo $L("$DEFAULT") $L("$LENGTH");
    })
    .unwrap();
    golden::assert_golden("zsh/quote_variable_expansion.zsh", &render(&block));
}

#[test]
fn test_array_operations() {
    let block = sigil_quote!(Zsh {
        typeset -a ITEMS;
        ITEMS=(one two three);
        echo $L("${ITEMS[@]}");
        echo $L("${#ITEMS[@]}");
    })
    .unwrap();
    golden::assert_golden("zsh/quote_array_operations.zsh", &render(&block));
}

#[test]
fn test_function_definition() {
    let block = sigil_quote!(Zsh {
        function greet() {
            local name=$L("$1");
            echo $S("Hello, $name!");
        }
    })
    .unwrap();
    golden::assert_golden("zsh/quote_function.zsh", &render(&block));
}
