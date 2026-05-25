use super::tokens_to_format;
use crate::parse::types::{InterpolationKind, MacroLang};
use proc_macro2::{TokenStream, TokenTree};

fn fmt(src: &str) -> String {
    let ts: TokenStream = src.parse().unwrap();
    let tokens: Vec<TokenTree> = ts.into_iter().collect();
    let (format, _args) = tokens_to_format(&tokens, MacroLang::Unaware).unwrap();
    format
}

fn fmt_lang(src: &str, lang: MacroLang) -> String {
    let ts: TokenStream = src.parse().unwrap();
    let tokens: Vec<TokenTree> = ts.into_iter().collect();
    let (format, _args) = tokens_to_format(&tokens, lang).unwrap();
    format
}

fn fmt_with_args(src: &str) -> (String, Vec<InterpolationKind>) {
    let ts: TokenStream = src.parse().unwrap();
    let tokens: Vec<TokenTree> = ts.into_iter().collect();
    let (format, args) = tokens_to_format(&tokens, MacroLang::Unaware).unwrap();
    let kinds: Vec<InterpolationKind> = args.into_iter().map(|a| a.kind).collect();
    (format, kinds)
}

#[test]
fn simple_assignment() {
    assert_eq!(fmt("const x = 42"), "const x = 42");
}

#[test]
fn function_call() {
    assert_eq!(fmt("foo(x, y)"), "foo(x, y)");
}

#[test]
fn method_chain() {
    assert_eq!(fmt("a.b.c()"), "a.b.c()");
}

#[test]
fn generic_type() {
    assert_eq!(fmt("Vec<T>"), "Vec<T>");
}

#[test]
fn nested_generics() {
    assert_eq!(fmt("Map<String, Vec<T>>"), "Map<String, Vec<T>>");
}

#[test]
fn path_separator() {
    assert_eq!(fmt("std::mem::replace"), "std::mem::replace");
}

#[test]
fn percent_escaped() {
    assert_eq!(fmt("x % y"), "x %% y");
}

#[test]
fn interpolation_type() {
    let (format, kinds) = fmt_with_args("$T(user_type)");
    assert_eq!(format, "%T");
    assert_eq!(kinds.len(), 1);
    assert!(matches!(kinds[0], InterpolationKind::Type));
}

#[test]
fn interpolation_name() {
    let (format, kinds) = fmt_with_args("$N(field)");
    assert_eq!(format, "%N");
    assert!(matches!(kinds[0], InterpolationKind::Name));
}

#[test]
fn interpolation_string() {
    let (format, kinds) = fmt_with_args("$S(\"hello\")");
    assert_eq!(format, "%S");
    assert!(matches!(kinds[0], InterpolationKind::StringLit));
}

#[test]
fn interpolation_literal() {
    let (format, kinds) = fmt_with_args("$L(value)");
    assert_eq!(format, "%L");
    assert!(matches!(kinds[0], InterpolationKind::Literal));
}

#[test]
fn interpolation_code() {
    let (format, kinds) = fmt_with_args("$C(block)");
    assert_eq!(format, "%L");
    assert!(matches!(kinds[0], InterpolationKind::Code));
}

#[test]
fn soft_break() {
    let format = fmt("$W");
    assert_eq!(format, "%W");
}

#[test]
fn dollar_escape() {
    let format = fmt("$$");
    assert_eq!(format, "$");
}

#[test]
fn mixed_interpolation_and_text() {
    let (format, kinds) = fmt_with_args("const x: $T(t) = $L(v)");
    assert_eq!(format, "const x: %T = %L");
    assert_eq!(kinds.len(), 2);
    assert!(matches!(kinds[0], InterpolationKind::Type));
    assert!(matches!(kinds[1], InterpolationKind::Literal));
}

#[test]
fn arrow_no_space_between_parts() {
    let format = fmt("a -> b");
    assert_eq!(format, "a -> b");
}

#[test]
fn comma_no_space_before() {
    assert_eq!(fmt("(a, b, c)"), "(a, b, c)");
}

#[test]
fn ternary_colon_spaced() {
    assert_eq!(fmt("x ? a : b"), "x ? a : b");
}

#[test]
fn type_annotation_colon_no_space_before() {
    assert_eq!(fmt("name: String"), "name: String");
}

#[test]
fn prefix_operators() {
    assert_eq!(fmt("!x"), "!x");
    assert_eq!(fmt("~bits"), "~bits");
}

#[test]
fn array_index() {
    assert_eq!(fmt("arr[0]"), "arr[0]");
}

#[test]
fn comparison_operators() {
    assert_eq!(fmt("a === b"), "a === b");
    assert_eq!(fmt("a !== b"), "a !== b");
}

#[test]
fn return_keyword_spacing() {
    assert_eq!(fmt("return x"), "return x");
}

#[test]
fn if_with_parens() {
    assert_eq!(fmt("if (x > 0)"), "if (x > 0)");
}

#[test]
fn go_slice_type() {
    assert_eq!(fmt_lang("[]byte", MacroLang::Go), "[]byte");
}

#[test]
fn go_map_type() {
    assert_eq!(fmt_lang("map[string]int", MacroLang::Go), "map[string]int");
}
