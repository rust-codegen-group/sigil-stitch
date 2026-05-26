use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::php::Php;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

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
fn test_control_flow() {
    crate::shared::run_control_flow_test::<super::quote_suite::PhpSuite>();
}

#[test]
fn test_if_elseif_else() {
    let block = sigil_quote!(Php {
        if ($$score >= 90) {
            $$grade = $S("A");
        } elseif ($$score >= 80) {
            $$grade = $S("B");
        } else {
            $$grade = $S("F");
        }
    })
    .unwrap();
    golden::assert_golden("php/quote_if_elseif.php", &render(&block));
}

#[test]
fn test_foreach() {
    let block = sigil_quote!(Php {
        foreach ($$items as $$item) {
            echo $$item;
        }
    })
    .unwrap();
    golden::assert_golden("php/quote_foreach.php", &render(&block));
}

#[test]
fn test_for_loop() {
    let block = sigil_quote!(Php {
        for ($$i = 0; $$i < 10; $$i++) {
            echo $$i;
        }
    })
    .unwrap();
    golden::assert_golden("php/quote_for_loop.php", &render(&block));
}

#[test]
fn test_while_loop() {
    let block = sigil_quote!(Php {
        while ($$running) {
            $$running = false;
        }
    })
    .unwrap();
    golden::assert_golden("php/quote_while.php", &render(&block));
}
