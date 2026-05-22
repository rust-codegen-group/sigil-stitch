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
fn test_basic() {
    crate::shared::run_basic_test::<super::quote_suite::CSharpSuite>();
}

#[test]
fn test_variable_declarations() {
    let block = sigil_quote!(CSharp {
        var items = new List<string>();
        int count = items.Count;
        bool isEmpty = count == 0;
    })
    .unwrap();
    golden::assert_golden("csharp/macro_variables.cs", &render(&block));
}

#[test]
fn test_method_calls() {
    let block = sigil_quote!(CSharp {
        var result = await client.GetAsync(url);
        var content = await result.Content.ReadAsStringAsync();
        Console.WriteLine(content);
    })
    .unwrap();
    golden::assert_golden("csharp/macro_method_calls.cs", &render(&block));
}

#[test]
fn test_interface_method_signatures() {
    let block = sigil_quote!(CSharp {
        Task<User> GetUserAsync(string id);
        Task SaveUserAsync(User user);
        void Delete(string id);
    })
    .unwrap();
    golden::assert_golden("csharp/macro_interface_methods.cs", &render(&block));
}
