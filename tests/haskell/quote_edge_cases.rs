use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::haskell::Haskell;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

use super::golden;

fn render(block: &CodeBlock) -> String {
    FileSpec::builder_with("test.hs", Haskell::new())
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

#[test]
fn test_open_where() {
    let block = sigil_quote!(Haskell {
        class Functor f {
            fmap :: (a -> b) -> f a -> f b;
        }
    })
    .unwrap();
    golden::assert_golden("haskell/macro_open_where.hs", &render(&block));
}

#[test]
fn test_type_annotation() {
    let block = sigil_quote!(Haskell {
        map :: (a -> b) -> f a -> f b;
        id :: a -> a;
    })
    .unwrap();
    golden::assert_golden("haskell/quote_type_annotation.hs", &render(&block));
}

#[test]
fn test_do_notation() {
    let block = sigil_quote!(Haskell {
        main :: IO ();
        main = do {
            putStrLn $S("Enter name:");
            name <- getLine;
            putStrLn ($S("Hello, ") ++ name);
        }
    })
    .unwrap();
    golden::assert_golden("haskell/quote_do_notation.hs", &render(&block));
}

#[test]
fn test_guards() {
    let block = sigil_quote!(Haskell {
        classify :: Int -> String;
        classify n
            | n < 0 = $S("negative")
            | n == 0 = $S("zero")
            | otherwise = $S("positive");
    })
    .unwrap();
    golden::assert_golden("haskell/quote_guards.hs", &render(&block));
}

#[test]
fn test_list_comprehension() {
    let block = sigil_quote!(Haskell {
        evens :: [Int] -> [Int];
        evens xs = [x | x <- xs, even x];
    })
    .unwrap();
    golden::assert_golden("haskell/quote_list_comprehension.hs", &render(&block));
}

#[test]
fn test_typeclass_instance() {
    let block = sigil_quote!(Haskell {
        instance Show Point {
            show (Point x y) = $S("(") ++ show x ++ $S(", ") ++ show y ++ $S(")");
        }
    })
    .unwrap();
    golden::assert_golden("haskell/quote_typeclass_instance.hs", &render(&block));
}

#[test]
fn test_dollar_operator() {
    let block = sigil_quote!(Haskell {
        main = putStrLn $$ show 42;
    })
    .unwrap();
    golden::assert_golden("haskell/quote_dollar_operator.hs", &render(&block));
}

#[test]
fn test_name_keyword_escape_in_macro() {
    let name = "data";
    let block = sigil_quote!(Haskell {
        $N(name) = 1
    })
    .unwrap();

    let output = render(&block);
    assert!(output.contains("data' = 1"), "got: {output}");
}

// ── Dollar operator: tokenizer-level spacing fix ────────

#[test]
fn test_dollar_op_has_space_after() {
    let block = sigil_quote!(Haskell {
        main = putStrLn $$ show 42;
    })
    .unwrap();
    let output = render(&block);
    assert!(
        output.contains("$ show"),
        "dollar should have space after, got: {output}"
    );
    assert!(
        !output.contains("$show"),
        "dollar must not glue to next word, got: {output}"
    );
}

#[test]
fn test_dollar_op_in_expression() {
    let block = sigil_quote!(Haskell {
        result = f $$ g $$ h x;
    })
    .unwrap();
    let output = render(&block);
    assert!(
        output.contains("f $ g $ h"),
        "chained dollar ops need spaces, got: {output}"
    );
}

#[test]
fn test_bind_arrow_has_spaces() {
    let block = sigil_quote!(Haskell {
        x <- getLine;
    })
    .unwrap();
    let output = render(&block);
    assert!(
        output.contains("x <- getLine"),
        "bind arrow needs spaces on both sides, got: {output}"
    );
}
