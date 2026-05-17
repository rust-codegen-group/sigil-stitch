use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::rust_lang::RustLang;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

use super::golden;

fn render(block: &CodeBlock) -> String {
    FileSpec::builder_with("test.rs", RustLang::new())
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

#[test]
fn test_path_separator() {
    let block = sigil_quote!(RustLang {
        let size = std::mem::size_of::<u32>();
        let x = std::cmp::max(1, 2);
    })
    .unwrap();
    golden::assert_golden("rust/macro_path_separator.rs", &render(&block));
}

#[test]
fn test_lifetime() {
    let block = sigil_quote!(RustLang {
        fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
            if x.len() > y.len() {
                x
            } else {
                y
            }
        }
    })
    .unwrap();
    golden::assert_golden("rust/quote_lifetime.rs", &render(&block));
}

#[test]
fn test_trait_bound() {
    let block = sigil_quote!(RustLang {
        fn process<T: Clone + Send + 'static>(item: T) -> T {
            item.clone()
        }
    })
    .unwrap();
    golden::assert_golden("rust/quote_trait_bound.rs", &render(&block));
}

#[test]
fn test_pattern_matching() {
    let block = sigil_quote!(RustLang {
        match value {
            Some(x) if x > 0 => println!($S("positive: {}"), x),
            Some(0) => println!($S("zero")),
            None => println!($S("nothing")),
            _ => unreachable!(),
        }
    })
    .unwrap();
    golden::assert_golden("rust/quote_pattern_match.rs", &render(&block));
}

#[test]
fn test_closure() {
    let block = sigil_quote!(RustLang {
        let add = |a: i32, b: i32| -> i32 { a + b };
        let result: Vec<i32> = items.iter().map(|x| x * 2).collect();
    })
    .unwrap();
    golden::assert_golden("rust/quote_closure.rs", &render(&block));
}

#[test]
fn test_impl_block() {
    let block = sigil_quote!(RustLang {
        impl<T: Display> fmt::Display for Wrapper<T> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, $S("({})"), self.0)
            }
        }
    })
    .unwrap();
    golden::assert_golden("rust/quote_impl_block.rs", &render(&block));
}
