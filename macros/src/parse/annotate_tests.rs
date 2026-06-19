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
    let a = annotate_lang("Int?", MacroLang::CSharp);
    // "Int" "?" — adjacent → PostfixQuestion (C# nullable)
    assert_eq!(ann_at(&a, 1), TokenAnnotation::PostfixQuestion);
}

#[test]
fn postfix_star_adjacent() {
    let a = annotate_lang("Config*", MacroLang::C);
    // "Config" "*" — adjacent → PostfixStar (C pointer)
    assert_eq!(ann_at(&a, 1), TokenAnnotation::PostfixStar);
}

#[test]
fn assign_adjacent() {
    let a = annotate_lang("NAME=val", MacroLang::Bash);
    // "NAME" "=" "val" — shell adjacent assign
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
fn go_pointer_star_after_field_name() {
    let a = annotate_lang("Raw *http.Response", MacroLang::Go);
    assert_eq!(ann_at(&a, 1), TokenAnnotation::PrefixOp);
}

#[test]
fn go_compact_star_multiplication_not_prefix() {
    assert_eq!(
        ann_at(&annotate_lang("a*b", MacroLang::Go), 1),
        TokenAnnotation::Normal
    );
    assert_eq!(
        ann_at(&annotate_lang("a*(b+c)", MacroLang::Go), 1),
        TokenAnnotation::Normal
    );
    assert_eq!(
        ann_at(&annotate_lang("1*2", MacroLang::Go), 1),
        TokenAnnotation::Normal
    );
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

// --- Language-gated annotation tests ---

#[test]
fn postfix_star_c_languages() {
    // C, C++, C# use postfix * for pointer types
    assert_eq!(
        ann_at(&annotate_lang("Config*", MacroLang::C), 1),
        TokenAnnotation::PostfixStar
    );
    assert_eq!(
        ann_at(&annotate_lang("Config*", MacroLang::Cpp), 1),
        TokenAnnotation::PostfixStar
    );
    assert_eq!(
        ann_at(&annotate_lang("int*", MacroLang::CSharp), 1),
        TokenAnnotation::PostfixStar
    );
}

#[test]
fn postfix_star_not_in_other_languages() {
    assert_eq!(
        ann_at(&annotate_lang("Config*", MacroLang::Go), 1),
        TokenAnnotation::Normal
    );
    assert_eq!(
        ann_at(&annotate_lang("Config*", MacroLang::Ruby), 1),
        TokenAnnotation::Normal
    );
    assert_eq!(
        ann_at(&annotate_lang("Config*", MacroLang::Bash), 1),
        TokenAnnotation::Normal
    );
}

#[test]
fn postfix_ampersand_cpp_only() {
    assert_eq!(
        ann_at(&annotate_lang("auto&", MacroLang::Cpp), 1),
        TokenAnnotation::PostfixAmpersand
    );
}

#[test]
fn postfix_ampersand_not_in_other_languages() {
    // C has no references, & is address-of (prefix)
    assert_eq!(
        ann_at(&annotate_lang("auto&", MacroLang::C), 1),
        TokenAnnotation::Normal
    );
    assert_eq!(
        ann_at(&annotate_lang("auto&", MacroLang::Go), 1),
        TokenAnnotation::Normal
    );
}

#[test]
fn postfix_question_nullable_languages() {
    assert_eq!(
        ann_at(&annotate_lang("Int?", MacroLang::CSharp), 1),
        TokenAnnotation::PostfixQuestion
    );
    assert_eq!(
        ann_at(&annotate_lang("string?", MacroLang::TypeScript), 1),
        TokenAnnotation::PostfixQuestion
    );
    assert_eq!(
        ann_at(&annotate_lang("Int?", MacroLang::Swift), 1),
        TokenAnnotation::PostfixQuestion
    );
    assert_eq!(
        ann_at(&annotate_lang("Int?", MacroLang::Kotlin), 1),
        TokenAnnotation::PostfixQuestion
    );
    assert_eq!(
        ann_at(&annotate_lang("int?", MacroLang::Dart), 1),
        TokenAnnotation::PostfixQuestion
    );
}

#[test]
fn postfix_question_not_in_other_languages() {
    assert_eq!(
        ann_at(&annotate_lang("foo?", MacroLang::Ruby), 1),
        TokenAnnotation::Normal
    );
    assert_eq!(
        ann_at(&annotate_lang("x?", MacroLang::Php), 1),
        TokenAnnotation::Normal
    );
}

#[test]
fn assign_adjacent_shell_only() {
    assert_eq!(
        ann_at(&annotate_lang("NAME=val", MacroLang::Bash), 1),
        TokenAnnotation::AssignAdjacent
    );
    assert_eq!(
        ann_at(&annotate_lang("NAME=val", MacroLang::Zsh), 1),
        TokenAnnotation::AssignAdjacent
    );
}

#[test]
fn assign_adjacent_not_in_non_shell() {
    assert_eq!(
        ann_at(&annotate_lang("x=5", MacroLang::Go), 1),
        TokenAnnotation::Normal
    );
    assert_eq!(
        ann_at(&annotate_lang("x=5", MacroLang::C), 1),
        TokenAnnotation::Normal
    );
    assert_eq!(
        ann_at(&annotate_lang("x=5", MacroLang::Cpp), 1),
        TokenAnnotation::Normal
    );
    assert_eq!(
        ann_at(&annotate_lang("x=5", MacroLang::Unaware), 1),
        TokenAnnotation::Normal
    );
}

// --- Compound operator tests ---

#[test]
fn compound_and_infix() {
    // a && b: second & should be Normal (infix)
    assert_eq!(
        ann_at(&annotate_lang("a && b", MacroLang::Cpp), 2),
        TokenAnnotation::Normal
    );
}

#[test]
fn compound_and_prefix() {
    // &&str: second & should be PrefixOp (prefix compound)
    assert_eq!(
        ann_at(&annotate_lang("&&str", MacroLang::Unaware), 1),
        TokenAnnotation::PrefixOp
    );
}

#[test]
fn compound_and_postfix() {
    // auto&& item: second & should be Normal (postfix)
    assert_eq!(
        ann_at(&annotate_lang("auto&& item", MacroLang::Cpp), 2),
        TokenAnnotation::Normal
    );
}

#[test]
fn compound_star_infix() {
    // a ** b: second * should be Normal (infix)
    assert_eq!(
        ann_at(&annotate_lang("a ** b", MacroLang::Unaware), 2),
        TokenAnnotation::Normal
    );
}

#[test]
fn compound_star_prefix() {
    // **ptr: second * should be PrefixOp (prefix compound)
    assert_eq!(
        ann_at(&annotate_lang("**ptr", MacroLang::Unaware), 1),
        TokenAnnotation::PrefixOp
    );
}

// --- PostfixQuestion ternary tests ---

#[test]
fn postfix_question_ternary_literal() {
    // x?1:2: ? should be Normal (compact ternary with literal)
    assert_eq!(
        ann_at(&annotate_lang("x?1:2", MacroLang::CSharp), 1),
        TokenAnnotation::Normal
    );
}

#[test]
fn postfix_question_ternary_ident() {
    // x?y:z: ? should be Normal (compact ternary with ident)
    assert_eq!(
        ann_at(&annotate_lang("x?y:z", MacroLang::CSharp), 1),
        TokenAnnotation::Normal
    );
}

#[test]
fn postfix_question_nullable_with_space() {
    // Int? name: ? followed by space then ident → nullable, not ternary
    assert_eq!(
        ann_at(&annotate_lang("Int? name", MacroLang::CSharp), 1),
        TokenAnnotation::PostfixQuestion
    );
}

#[test]
fn postfix_question_nullable_comma() {
    // Int?,: ? followed by comma → nullable
    assert_eq!(
        ann_at(&annotate_lang("Int?, x", MacroLang::CSharp), 1),
        TokenAnnotation::PostfixQuestion
    );
}

#[test]
fn postfix_question_optional_property() {
    // name?: string: ? followed by : is TS optional property, not ternary
    assert_eq!(
        ann_at(&annotate_lang("name?: string", MacroLang::TypeScript), 1),
        TokenAnnotation::PostfixQuestion
    );
}

// --- GenericOpen gate tests ---

#[test]
fn generic_open_gated_for_c() {
    // C has no angle generics — < should be Normal
    assert_eq!(
        ann_at(&annotate_lang("Array<T>", MacroLang::C), 1),
        TokenAnnotation::Normal
    );
}

#[test]
fn generic_open_allowed_for_generic_lang() {
    // Unaware (C-like) — < should be GenericOpen
    assert_eq!(
        ann_at(&annotate_lang("Vec<T>", MacroLang::Unaware), 1),
        TokenAnnotation::GenericOpen
    );
}

#[test]
fn ruby_inheritance_angle_preserved() {
    assert_eq!(
        ann_at(&annotate_lang("class Dog < Animal", MacroLang::Ruby), 2),
        TokenAnnotation::InheritanceAngle
    );
}
