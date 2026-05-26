use super::*;
use proc_macro2::{TokenStream, TokenTree};

fn annotate_src(src: &str) -> Vec<(String, TokenAnnotation)> {
    let ts: TokenStream = src.parse().unwrap();
    let tokens: Vec<TokenTree> = ts.into_iter().collect();
    let annotations = annotate_tokens(&tokens, MacroLang::Unaware);
    tokens
        .iter()
        .zip(annotations.iter())
        .map(|(tt, ann)| (tt.to_string(), *ann))
        .collect()
}

fn annotate_lang(src: &str, lang: MacroLang) -> Vec<(String, TokenAnnotation)> {
    let ts: TokenStream = src.parse().unwrap();
    let tokens: Vec<TokenTree> = ts.into_iter().collect();
    let annotations = annotate_tokens(&tokens, lang);
    tokens
        .iter()
        .zip(annotations.iter())
        .map(|(tt, ann)| (tt.to_string(), *ann))
        .collect()
}

fn ann_at(anns: &[(String, TokenAnnotation)], idx: usize) -> TokenAnnotation {
    anns[idx].1
}

#[test]
fn generic_open_close() {
    let a = annotate_src("Vec < T >");
    assert_eq!(ann_at(&a, 1), TokenAnnotation::GenericOpen);
    assert_eq!(ann_at(&a, 3), TokenAnnotation::GenericClose);
}

#[test]
fn generic_not_triggered_for_lowercase() {
    let a = annotate_src("x < y >");
    assert_eq!(ann_at(&a, 1), TokenAnnotation::Normal);
}

#[test]
fn macro_bang() {
    let a = annotate_src("println !");
    assert_eq!(ann_at(&a, 1), TokenAnnotation::MacroBang);
}

#[test]
fn path_sep_adjacent() {
    let a = annotate_src("std::fmt");
    // tokens: "std" ":" ":" "fmt"
    // Second ":" gets MethodCallColon (adjacent both sides); both
    // MethodCallColon and PathSepComplete suppress space — same effect.
    assert_eq!(ann_at(&a, 2), TokenAnnotation::MethodCallColon);
}

#[test]
fn double_colon_op_spaced() {
    // fmap :: Type — space before `::` makes it an operator
    let a = annotate_src("fmap :: Type");
    assert_eq!(ann_at(&a, 1), TokenAnnotation::DoubleColonOp);
}

#[test]
fn call_open_adjacent() {
    // f(x) — group adjacent to ident → CallOpen
    let a = annotate_src("f(x)");
    assert_eq!(ann_at(&a, 1), TokenAnnotation::CallOpen);
}

#[test]
fn call_open_not_spaced() {
    // f (x) — space before paren → Normal
    let a = annotate_src("f (x)");
    assert_eq!(ann_at(&a, 1), TokenAnnotation::Normal);
}

#[test]
fn dash_sep_hyphenated() {
    let a = annotate_src("from-oci-layout");
    // "from" "-" "oci" "-" "layout"
    assert_eq!(ann_at(&a, 1), TokenAnnotation::DashSep);
    assert_eq!(ann_at(&a, 3), TokenAnnotation::DashSep);
}

#[test]
fn slash_sep_path() {
    let a = annotate_src("linux/amd64");
    // "linux" "/" "amd64"
    assert_eq!(ann_at(&a, 1), TokenAnnotation::SlashSep);
}

#[test]
fn arrow_op_adjacent() {
    let a = annotate_src("ptr->field");
    // "ptr" "-" ">" "field"
    assert_eq!(ann_at(&a, 1), TokenAnnotation::ArrowOp);
}

#[test]
fn prefix_op_star() {
    let a = annotate_src("= *ptr");
    // "=" "*" "ptr" — * after = is prefix
    assert_eq!(ann_at(&a, 1), TokenAnnotation::PrefixOp);
}

#[test]
fn prefix_op_ampersand() {
    let a = annotate_src("= &x");
    assert_eq!(ann_at(&a, 1), TokenAnnotation::PrefixOp);
}

#[test]
fn postfix_question_adjacent() {
    let a = annotate_src("Int?");
    // "Int" "?" — adjacent → PostfixQuestion
    assert_eq!(ann_at(&a, 1), TokenAnnotation::PostfixQuestion);
}

#[test]
fn postfix_star_adjacent() {
    let a = annotate_src("Config*");
    // "Config" "*" — adjacent → PostfixStar
    assert_eq!(ann_at(&a, 1), TokenAnnotation::PostfixStar);
}

#[test]
fn assign_adjacent() {
    let a = annotate_src("NAME=val");
    // "NAME" "=" "val"
    assert_eq!(ann_at(&a, 1), TokenAnnotation::AssignAdjacent);
}

// --- Language-specific ---

#[test]
fn bash_dash_flag() {
    let a = annotate_lang("-avz", MacroLang::Bash);
    assert_eq!(ann_at(&a, 0), TokenAnnotation::DashFlag);
}

#[test]
fn bash_dot_arg() {
    let a = annotate_lang("find .", MacroLang::Bash);
    assert_eq!(ann_at(&a, 1), TokenAnnotation::DotArg);
}

#[test]
fn bash_slash_leading_path() {
    let a = annotate_lang("/usr/local", MacroLang::Bash);
    // "/" "usr" "/" "local"
    assert_eq!(ann_at(&a, 0), TokenAnnotation::SlashSep);
}

#[test]
fn go_receive_adjacent() {
    // <-ch → < is Joint, - adjacent to ch → PrefixOp on -
    let a = annotate_lang("<-ch", MacroLang::Go);
    assert_eq!(ann_at(&a, 1), TokenAnnotation::PrefixOp);
}

#[test]
fn go_send_not_prefix() {
    // ch <- 42 → - is NOT adjacent to 42 → stays Normal
    let a = annotate_lang("ch <- 42", MacroLang::Go);
    assert_eq!(ann_at(&a, 2), TokenAnnotation::Normal);
}

#[test]
fn ruby_symbol_colon() {
    // space before :, NOT after (:name is adjacent)
    let a = annotate_lang("attr_reader :name", MacroLang::Ruby);
    assert_eq!(
        ann_at(&a, 1),
        TokenAnnotation::SymbolColon,
        "expected SymbolColon, tokens: {}",
        a.iter()
            .map(|(t, a)| format!("{t}({a:?})"))
            .collect::<Vec<_>>()
            .join(" ")
    );
}

#[test]
fn normal_tokens_stay_normal() {
    let a = annotate_src("let x = 42");
    for (_, ann) in &a {
        assert_eq!(*ann, TokenAnnotation::Normal);
    }
}
