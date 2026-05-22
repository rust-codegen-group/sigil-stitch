use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::csharp::CSharp;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

use super::golden;

fn render(block: &CodeBlock) -> String {
    FileSpec::builder_with("Test.cs", CSharp::new())
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

#[test]
fn test_if_else() {
    crate::shared::run_control_flow_test::<super::quote_suite::CSharpSuite>();
}

#[test]
fn test_for_loop() {
    let block = sigil_quote!(CSharp {
        for (var i = 0; i < items.Length; i++) {
            Console.WriteLine(items[i]);
        }
    })
    .unwrap();
    golden::assert_golden("csharp/macro_for_loop.cs", &render(&block));
}

#[test]
fn test_try_catch() {
    let block = sigil_quote!(CSharp {
        try {
            DoSomething();
        } catch (Exception ex) {
            Log(ex.Message);
        } finally {
            Cleanup();
        }
    })
    .unwrap();
    golden::assert_golden("csharp/macro_try_catch.cs", &render(&block));
}

#[test]
fn test_foreach() {
    let block = sigil_quote!(CSharp {
        foreach (var item in items) {
            Process(item);
        }
    })
    .unwrap();
    golden::assert_golden("csharp/macro_foreach.cs", &render(&block));
}
