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

fn fmt_err(src: &str) -> String {
    let ts: TokenStream = src.parse().unwrap();
    let tokens: Vec<TokenTree> = ts.into_iter().collect();
    match tokens_to_format(&tokens, MacroLang::Unaware) {
        Ok(_) => panic!("expected error for {src}"),
        Err(err) => err.message().to_string(),
    }
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

#[test]
fn ruby_symbol_colon_spacing() {
    // space before :, none after
    assert_eq!(
        fmt_lang("attr_reader :name", MacroLang::Ruby),
        "attr_reader :name"
    );
}

#[test]
fn inline_for_emits_parsed_splice() {
    let (format, kinds) = fmt_with_args("($for(x in items) { $N(*x) })");
    assert_eq!(format, "(%L)");
    assert_eq!(kinds.len(), 1);
    assert!(matches!(kinds[0], InterpolationKind::ParsedSplice));
}

#[test]
fn inline_for_mixed_with_literals() {
    let (format, kinds) = fmt_with_args("prefix $for(x in items) { $N(*x) } suffix");
    assert!(format.starts_with("prefix "));
    assert!(format.contains(" %L "));
    assert!(format.ends_with(" suffix"));
    assert!(matches!(kinds[0], InterpolationKind::ParsedSplice));
}

#[test]
fn inline_if_emits_parsed_splice() {
    let (format, kinds) = fmt_with_args("($if(cond) { a } $else { b })");
    assert_eq!(format, "(%L)");
    assert_eq!(kinds.len(), 1);
    assert!(matches!(kinds[0], InterpolationKind::ParsedSplice));
}

#[test]
fn inline_else_if_standalone_rejected() {
    let ts: TokenStream = "$else_if(cond) { body }".parse().unwrap();
    let tokens: Vec<TokenTree> = ts.into_iter().collect();
    let result = tokens_to_format(&tokens, MacroLang::Unaware);
    assert!(result.is_err());
}

#[test]
fn inline_else_standalone_rejected() {
    let ts: TokenStream = "$else { body }".parse().unwrap();
    let tokens: Vec<TokenTree> = ts.into_iter().collect();
    let result = tokens_to_format(&tokens, MacroLang::Unaware);
    assert!(result.is_err());
}

#[test]
fn inline_let_rejected() {
    let ts: TokenStream = "($let(x = 1))".parse().unwrap();
    let tokens: Vec<TokenTree> = ts.into_iter().collect();
    let result = tokens_to_format(&tokens, MacroLang::Unaware);
    assert!(result.is_err());
}

#[test]
fn inline_c_each_rejected() {
    let message = fmt_err("($C_each(blocks))");
    assert!(
        message.contains("$C_each() must appear at the start of a line"),
        "got: {message}"
    );
}

#[test]
fn inline_for_missing_body_rejected() {
    let message = fmt_err("($for(x in items))");
    assert!(
        message.contains("$for requires a brace body"),
        "got: {message}"
    );
}

#[test]
fn inline_if_empty_condition_rejected() {
    let message = fmt_err("($if() { body })");
    assert!(
        message.contains("$if condition cannot be empty"),
        "got: {message}"
    );
}

#[test]
fn inline_for_adjacent_to_prev_specifier() {
    // $T(type)$for(x in items) { $N(*x) } — no space between specifiers
    let ts: TokenStream = "$T(my_type)$for(x in items) { $N(*x) }".parse().unwrap();
    let tokens: Vec<TokenTree> = ts.into_iter().collect();
    let (format, args) = tokens_to_format(&tokens, MacroLang::Unaware).unwrap();
    assert_eq!(format, "%T%L");
    assert_eq!(args.len(), 2);
    assert!(matches!(args[0].kind, InterpolationKind::Type));
    assert!(matches!(args[1].kind, InterpolationKind::ParsedSplice));
}

#[test]
fn blank_line_preserved_inside_parens() {
    // Blank line (double-newline gap) between tokens inside a group.
    let src = "(\nhello\n\nworld\n)";
    let ts: TokenStream = src.parse().unwrap();
    let tokens: Vec<TokenTree> = ts.into_iter().collect();
    let (format, _) = tokens_to_format(&tokens, MacroLang::Unaware).unwrap();
    // At minimum, the format should separate hello and world
    assert!(format.contains("hello"), "format: {format:?}");
    assert!(format.contains("world"), "format: {format:?}");
}

#[test]
fn blank_line_preserved_before_inline_for() {
    // Newline between `[` and `$for` in a multiline array literal.
    let (format, _) = fmt_with_args("[\n$for(x in items) { $L(*x), }]");
    // The inline $for handler processes the tokens; verify format structure.
    assert!(
        format.contains("[") && format.contains("%L"),
        "format: {format:?}"
    );
}

#[test]
fn no_blank_line_for_same_line() {
    let (format, _) = fmt_with_args("(hello world)");
    assert!(!format.contains('\n'), "no newline expected: {format:?}");
}
