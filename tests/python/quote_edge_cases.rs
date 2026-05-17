use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

use super::golden;

fn render(block: &CodeBlock) -> String {
    FileSpec::builder("test.py")
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

#[test]
fn test_decorator() {
    let block = sigil_quote!(Python {
        @app.route("/users")
        def get_users(): {
            return jsonify(users)
        }
    })
    .unwrap();
    golden::assert_golden("python/quote_decorator.py", &render(&block));
}

#[test]
fn test_type_hints() {
    let block = sigil_quote!(Python {
        def process(items: list[str], count: int = 10) -> dict[str, int]: {
            result: dict[str, int] = {}
            return result
        }
    })
    .unwrap();
    golden::assert_golden("python/quote_type_hints.py", &render(&block));
}

#[test]
fn test_comprehension() {
    let block = sigil_quote!(Python {
        squares = [x * x for x in range(10)]
        evens = [x for x in items if x % 2 == 0]
    })
    .unwrap();
    golden::assert_golden("python/quote_comprehension.py", &render(&block));
}

#[test]
fn test_async_def() {
    let block = sigil_quote!(Python {
        async def fetch_data(url: str) -> bytes: {
            async with aiohttp.ClientSession() as session: {
                response = await session.get(url)
                return await response.read()
            }
        }
    })
    .unwrap();
    golden::assert_golden("python/quote_async.py", &render(&block));
}

#[test]
fn test_with_statement() {
    let block = sigil_quote!(Python {
        with open($S("file.txt"), $S("r")) as f: {
            content = f.read()
        }
    })
    .unwrap();
    golden::assert_golden("python/quote_with.py", &render(&block));
}
