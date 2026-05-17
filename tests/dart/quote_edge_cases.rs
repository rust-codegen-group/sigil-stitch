use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::dart::DartLang;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

use super::golden;

fn render(block: &CodeBlock) -> String {
    FileSpec::builder_with("test.dart", DartLang::new())
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

#[test]
fn test_cascade() {
    let block = sigil_quote!(DartLang {
        final builder = StringBuffer()
            ..write("hello")
            ..write(" world");
    })
    .unwrap();
    golden::assert_golden("dart/quote_cascade.dart", &render(&block));
}

#[test]
fn test_named_params() {
    let block = sigil_quote!(DartLang {
        void configure({required String host, int port = 8080}) {
            print(host);
        }
    })
    .unwrap();
    golden::assert_golden("dart/quote_named_params.dart", &render(&block));
}

#[test]
fn test_null_aware() {
    let block = sigil_quote!(DartLang {
        String name = user?.name ?? $S("anonymous");
        list ??= [];
        final length = items?.length ?? 0;
    })
    .unwrap();
    golden::assert_golden("dart/quote_null_aware.dart", &render(&block));
}

#[test]
fn test_async_await() {
    let block = sigil_quote!(DartLang {
        Future<String> fetchData(String url) async {
            final response = await http.get(Uri.parse(url));
            return response.body;
        }
    })
    .unwrap();
    golden::assert_golden("dart/quote_async_await.dart", &render(&block));
}
