use proc_macro2::{Delimiter, TokenTree};

use crate::ir::BranchIntent;

use super::lang::MacroLang;

/// Classify a parsed control-flow condition into a language-neutral block
/// role. Language-specific branches add only their own token shapes; common
/// keyword recognition is shared.
pub(super) fn classify_branch(tokens: &[TokenTree], lang: MacroLang) -> BranchIntent {
    match lang {
        MacroLang::Go => go_intent(tokens),
        MacroLang::Cpp => cpp_intent(tokens),
        MacroLang::Haskell => haskell_intent(tokens),
        MacroLang::OCaml => ocaml_intent(tokens),
        MacroLang::Bash | MacroLang::Zsh => shell_intent(tokens),
        _ => common_intent(tokens),
    }
}

fn go_intent(tokens: &[TokenTree]) -> BranchIntent {
    if first_ident_is(tokens, "go") && second_ident_is(tokens, "func") {
        BranchIntent::Function
    } else {
        common_intent(tokens)
    }
}

fn cpp_intent(tokens: &[TokenTree]) -> BranchIntent {
    if looks_like_lambda(tokens) {
        BranchIntent::Lambda
    } else {
        common_intent(tokens)
    }
}

fn haskell_intent(tokens: &[TokenTree]) -> BranchIntent {
    let common = common_intent(tokens);
    if common != BranchIntent::Generic {
        return common;
    }
    if last_ident_is(tokens, "do") {
        BranchIntent::Do
    } else {
        BranchIntent::Generic
    }
}

fn ocaml_intent(tokens: &[TokenTree]) -> BranchIntent {
    let common = common_intent(tokens);
    if common != BranchIntent::Generic {
        return common;
    }
    // `let describe x = match x with` is an OCaml control-flow opener whose
    // leading token is `let`, so scan the header for `match` / `try`.
    if contains_ident(tokens, "match") {
        BranchIntent::Match
    } else if contains_ident(tokens, "try") {
        BranchIntent::Try
    } else {
        BranchIntent::Generic
    }
}

fn shell_intent(tokens: &[TokenTree]) -> BranchIntent {
    // Shell needs the same leading-keyword labels as the common classifier.
    // Delimiter punctuation is handled by the runtime shell adapters.
    common_intent(tokens)
}

fn common_intent(tokens: &[TokenTree]) -> BranchIntent {
    let Some(first) = first_ident(tokens) else {
        return BranchIntent::Generic;
    };
    match first.as_str() {
        "if" => BranchIntent::If,
        "elif" | "elseif" => BranchIntent::ElseIf,
        "else" => {
            if second_ident_is(tokens, "if") {
                BranchIntent::ElseIf
            } else {
                BranchIntent::Else
            }
        }
        "for" => BranchIntent::For,
        "while" => BranchIntent::While,
        "until" => BranchIntent::Until,
        "case" => BranchIntent::Case,
        "match" => BranchIntent::Match,
        "try" => BranchIntent::Try,
        "class" => BranchIntent::Class,
        "instance" => BranchIntent::Instance,
        "do" => BranchIntent::Do,
        "module" => {
            if second_ident_is(tokens, "type") {
                BranchIntent::ModuleType
            } else {
                BranchIntent::Module
            }
        }
        "function" | "func" | "fn" | "def" | "fun" => BranchIntent::Function,
        _ => BranchIntent::Generic,
    }
}

fn first_ident(tokens: &[TokenTree]) -> Option<String> {
    tokens.iter().find_map(|tt| match tt {
        TokenTree::Ident(id) => Some(id.to_string()),
        TokenTree::Group(g) if g.delimiter() == Delimiter::None => {
            let inner: Vec<TokenTree> = g.stream().into_iter().collect();
            first_ident(&inner)
        }
        _ => None,
    })
}

fn first_ident_is(tokens: &[TokenTree], expected: &str) -> bool {
    first_ident(tokens).as_deref() == Some(expected)
}

fn second_ident_is(tokens: &[TokenTree], expected: &str) -> bool {
    let mut idents = tokens.iter().filter_map(|tt| match tt {
        TokenTree::Ident(id) => Some(id.to_string()),
        TokenTree::Group(g) if g.delimiter() == Delimiter::None => {
            let inner: Vec<TokenTree> = g.stream().into_iter().collect();
            first_ident(&inner)
        }
        _ => None,
    });
    let _first = idents.next();
    idents.next().as_deref() == Some(expected)
}

fn last_ident_is(tokens: &[TokenTree], expected: &str) -> bool {
    tokens.iter().rev().find_map(|tt| match tt {
        TokenTree::Ident(id) => Some(id.to_string()),
        TokenTree::Group(g) if g.delimiter() == Delimiter::None => {
            let inner: Vec<TokenTree> = g.stream().into_iter().collect();
            last_ident_is(&inner, expected).then(|| expected.to_string())
        }
        _ => None,
    }) == Some(expected.to_string())
}

fn contains_ident(tokens: &[TokenTree], expected: &str) -> bool {
    tokens.iter().any(|tt| match tt {
        TokenTree::Ident(id) => id == expected,
        TokenTree::Group(g) => {
            let inner: Vec<TokenTree> = g.stream().into_iter().collect();
            contains_ident(&inner, expected)
        }
        _ => false,
    })
}

fn looks_like_lambda(tokens: &[TokenTree]) -> bool {
    for (index, token) in tokens.iter().enumerate() {
        let TokenTree::Group(capture) = token else {
            continue;
        };
        if capture.delimiter() != Delimiter::Bracket {
            continue;
        }
        let has_params = matches!(
            tokens.get(index + 1),
            Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Parenthesis
        );
        if !has_params {
            continue;
        }
        let after_assignment = index > 0
            && matches!(
                tokens.get(index - 1),
                Some(TokenTree::Punct(p)) if p.as_char() == '='
            );
        let after_return = index > 0
            && matches!(
                tokens.get(index - 1),
                Some(TokenTree::Ident(id)) if *id == "return"
            );
        if index == 0 || after_assignment || after_return {
            return true;
        }
    }
    false
}
