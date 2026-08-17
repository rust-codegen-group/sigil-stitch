use proc_macro2::{Delimiter, TokenStream, TokenTree};
use syn::parse::Parser;

use super::MacroLang;
use super::parse_body;
use super::recovery::{Recovered, combine};
use super::rust_interpolation::parse_expr;
use super::util::is_ident;
use crate::ir::{ConditionalBranch, LoopSeparator, MetaIf, Statement};

type ForComponents = (
    usize,
    syn::Pat,
    syn::Expr,
    Option<LoopSeparator>,
    Vec<Statement>,
);

pub(super) type ForHeader = (syn::Pat, syn::Expr, Option<LoopSeparator>);

pub(super) struct ForRawComponents {
    pub(super) next_pos: usize,
    pub(super) header: syn::Result<ForHeader>,
    pub(super) body_tokens: Vec<TokenTree>,
}

pub(super) type IfComponents = Recovered<MetaIf>;

/// Parse `(pat in expr[; separator = expr[, trailing = bool]]) { body }` at
/// `paren_pos` (after `$for` confirmed).
///
/// Returns `(next_pos, pat, iter_expr, separator, trailing, body_statements)`
/// where `next_pos` is the position after the closing `}` group.
pub(super) fn parse_for_components(
    tokens: &[TokenTree],
    paren_pos: usize,
    lang: MacroLang,
) -> Result<ForComponents, syn::Error> {
    let parts = parse_for_raw_components(tokens, paren_pos)?;
    let body = parse_body(&parts.body_tokens, lang);

    match (parts.header, body) {
        (Ok((pat, iter_expr, separator)), Ok(body)) => {
            Ok((parts.next_pos, pat, iter_expr, separator, body))
        }
        (header, body) => {
            let mut errors = None;
            if let Err(error) = header {
                combine(&mut errors, error);
            }
            if let Err(error) = body {
                combine(&mut errors, error);
            }
            let Some(error) = errors else {
                return Err(syn::Error::new(
                    proc_macro2::Span::call_site(),
                    "invalid $for directive",
                ));
            };
            Err(error)
        }
    }
}

/// Parse `(pat in expr[; separator = expr[, trailing = bool]]) { body }` and
/// return raw body tokens. Statement-level callers parse the body as statements;
/// inline callers parse it as a format fragment.
pub(super) fn parse_for_raw_components(
    tokens: &[TokenTree],
    paren_pos: usize,
) -> Result<ForRawComponents, syn::Error> {
    // Bounds checks.
    if paren_pos >= tokens.len() {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "expected parenthesized pattern after $for",
        ));
    }
    if paren_pos + 1 >= tokens.len() {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "$for requires a brace body: $for(pat in expr) { ... }",
        ));
    }

    let paren_group = match &tokens[paren_pos] {
        TokenTree::Group(g) if g.delimiter() == Delimiter::Parenthesis => g.clone(),
        _ => {
            return Err(syn::Error::new(
                tokens[paren_pos].span(),
                "$for requires a parenthesized pattern: $for(pat in expr) { ... }",
            ));
        }
    };

    let body_group = match &tokens[paren_pos + 1] {
        TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => g.clone(),
        _ => {
            return Err(syn::Error::new(
                tokens[paren_pos + 1].span(),
                "$for requires a brace body: $for(pat in expr) { ... }",
            ));
        }
    };

    let header = parse_for_header(&paren_group);
    let body_tokens: Vec<TokenTree> = body_group.stream().into_iter().collect();

    Ok(ForRawComponents {
        next_pos: paren_pos + 2,
        header,
        body_tokens,
    })
}

fn parse_for_header(paren_group: &proc_macro2::Group) -> syn::Result<ForHeader> {
    // Split paren contents on the first `in` keyword, then split loop options
    // after a top-level `;` so iterator expressions can still contain commas.
    let paren_tokens: Vec<TokenTree> = paren_group.stream().into_iter().collect();
    let in_pos = paren_tokens.iter().position(|tt| is_ident(tt, "in"));
    let in_pos = match in_pos {
        Some(p) => p,
        None => {
            return Err(syn::Error::new(
                paren_group.span(),
                "$for requires `in` keyword: $for(pat in expr) { ... }",
            ));
        }
    };

    if in_pos == 0 {
        return Err(syn::Error::new(
            paren_group.span(),
            "$for pattern cannot be empty: $for(pat in expr) { ... }",
        ));
    }
    if in_pos + 1 >= paren_tokens.len() {
        return Err(syn::Error::new(
            paren_group.span(),
            "$for iterator expression cannot be empty: $for(pat in expr) { ... }",
        ));
    }

    let pat_tokens: TokenStream = paren_tokens[..in_pos].iter().cloned().collect();
    let pat = syn::Pat::parse_multi_with_leading_vert
        .parse2(pat_tokens)
        .map_err(|error| syn::Error::new(error.span(), format!("invalid $for pattern: {error}")));
    let after_in = &paren_tokens[in_pos + 1..];
    let options_pos = after_in
        .iter()
        .position(|tt| matches!(tt, TokenTree::Punct(p) if p.as_char() == ';'));
    let (iter_tokens, option_tokens) = match options_pos {
        Some(pos) => (&after_in[..pos], Some(&after_in[pos + 1..])),
        None => (after_in, None),
    };

    let iter_tokens: TokenStream = iter_tokens.iter().cloned().collect();
    if iter_tokens.is_empty() {
        return Err(syn::Error::new(
            paren_group.span(),
            "$for iterator expression cannot be empty: $for(pat in expr) { ... }",
        ));
    }
    let iter_expr = parse_expr(iter_tokens, "$for iterator", paren_group.span());

    let separator = match option_tokens {
        Some(tokens) => parse_for_options(tokens, paren_group.span()),
        None => Ok(None),
    };

    match (pat, iter_expr, separator) {
        (Ok(pat), Ok(iter_expr), Ok(separator)) => Ok((pat, iter_expr, separator)),
        results => {
            let mut errors = None;
            if let Err(error) = results.0 {
                combine(&mut errors, error);
            }
            if let Err(error) = results.1 {
                combine(&mut errors, error);
            }
            if let Err(error) = results.2 {
                combine(&mut errors, error);
            }
            let Some(error) = errors else {
                return Err(syn::Error::new(
                    paren_group.span(),
                    "invalid $for directive",
                ));
            };
            Err(error)
        }
    }
}

fn parse_for_options(
    tokens: &[TokenTree],
    span: proc_macro2::Span,
) -> syn::Result<Option<LoopSeparator>> {
    if tokens.is_empty() {
        return Err(syn::Error::new(
            span,
            "$for options cannot be empty after ';'; expected separator = expr or trailing = bool",
        ));
    }

    let mut separator: Option<syn::Expr> = None;
    let mut trailing: Option<syn::Expr> = None;
    let mut seen_separator = false;
    let mut seen_trailing = false;
    let mut errors = None;

    for option in split_for_options(tokens) {
        if option.is_empty() {
            combine(
                &mut errors,
                syn::Error::new(
                    span,
                    "$for option cannot be empty; expected separator = expr or trailing = bool",
                ),
            );
            continue;
        }

        let name = match option.first() {
            Some(TokenTree::Ident(id)) => id.to_string(),
            Some(tt) => {
                combine(
                    &mut errors,
                    syn::Error::new(
                        tt.span(),
                        "$for options must be named: separator = expr, trailing = bool",
                    ),
                );
                continue;
            }
            None => continue,
        };
        let (kind, duplicate) = match name.as_str() {
            "separator" => {
                let duplicate = seen_separator;
                seen_separator = true;
                if duplicate {
                    combine(
                        &mut errors,
                        syn::Error::new(option[0].span(), "duplicate $for option 'separator'"),
                    );
                }
                (Some(ForOptionKind::Separator), duplicate)
            }
            "trailing" => {
                let duplicate = seen_trailing;
                seen_trailing = true;
                if duplicate {
                    combine(
                        &mut errors,
                        syn::Error::new(option[0].span(), "duplicate $for option 'trailing'"),
                    );
                }
                (Some(ForOptionKind::Trailing), duplicate)
            }
            _ => {
                combine(
                    &mut errors,
                    syn::Error::new(
                        option[0].span(),
                        format!("unknown $for option '{name}'; expected 'separator' or 'trailing'"),
                    ),
                );
                (None, false)
            }
        };

        let equals_pos = option
            .iter()
            .position(|tt| matches!(tt, TokenTree::Punct(p) if p.as_char() == '='));
        let equals_pos = match equals_pos {
            Some(pos) => pos,
            None => {
                combine(
                    &mut errors,
                    syn::Error::new(
                        option[0].span(),
                        format!("$for option '{name}' requires '='"),
                    ),
                );
                continue;
            }
        };

        if equals_pos != 1 {
            combine(
                &mut errors,
                syn::Error::new(
                    option[0].span(),
                    format!("$for option '{name}' requires '=' immediately after the name"),
                ),
            );
            continue;
        }

        let value_tokens: TokenStream = option[equals_pos + 1..].iter().cloned().collect();
        if value_tokens.is_empty() {
            combine(
                &mut errors,
                syn::Error::new(
                    option[0].span(),
                    format!("$for option '{name}' requires a value"),
                ),
            );
            continue;
        }
        let value = match parse_expr(
            value_tokens,
            &format!("$for option '{name}'"),
            option[0].span(),
        ) {
            Ok(value) => value,
            Err(error) => {
                combine(&mut errors, error);
                continue;
            }
        };

        match (kind, duplicate) {
            (Some(ForOptionKind::Separator), false) => separator = Some(value),
            (Some(ForOptionKind::Trailing), false) => trailing = Some(value),
            _ => {}
        }
    }

    if trailing.is_some() && !seen_separator {
        combine(
            &mut errors,
            syn::Error::new(span, "$for option 'trailing' requires separator = expr"),
        );
    }

    if let Some(error) = errors {
        return Err(error);
    }

    Ok(separator.map(|expr| LoopSeparator { expr, trailing }))
}

#[derive(Clone, Copy)]
enum ForOptionKind {
    Separator,
    Trailing,
}

fn split_for_options(tokens: &[TokenTree]) -> Vec<&[TokenTree]> {
    let mut parts = Vec::new();
    let mut start = 0;
    for (i, tt) in tokens.iter().enumerate() {
        if matches!(tt, TokenTree::Punct(p) if p.as_char() == ',')
            && starts_for_option(&tokens[i + 1..])
        {
            parts.push(&tokens[start..i]);
            start = i + 1;
        }
    }
    parts.push(&tokens[start..]);
    parts
}

fn starts_for_option(tokens: &[TokenTree]) -> bool {
    matches!(tokens, [TokenTree::Ident(_), TokenTree::Punct(eq), ..] if eq.as_char() == '=')
}

/// Parse `(cond) { body } [$else_if(cond) { body }]* [$else { body }]`
/// at `cond_pos` (after `$if` confirmed).
///
/// Returns `(next_pos, branches)` where `next_pos` is the position after
/// the last consumed token.
pub(super) fn parse_if_components(
    tokens: &[TokenTree],
    cond_pos: usize,
    lang: MacroLang,
) -> Result<IfComponents, syn::Error> {
    // Bounds check for first branch.
    if cond_pos >= tokens.len() {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "expected parenthesized condition after $if",
        ));
    }
    if cond_pos + 1 >= tokens.len() {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "$if requires a brace body: $if(condition) { ... }",
        ));
    }

    let mut pos = cond_pos;
    let mut else_if = Vec::new();
    let mut otherwise = None;
    let mut errors = None;

    // Parse `(cond) { body }` for the $if branch.
    let cond_group = match &tokens[pos] {
        TokenTree::Group(g) if g.delimiter() == Delimiter::Parenthesis => g.clone(),
        _ => {
            return Err(syn::Error::new(
                tokens[pos].span(),
                "$if requires a parenthesized condition: $if(condition) { ... }",
            ));
        }
    };
    pos += 1;

    let body_group = match &tokens[pos] {
        TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => g.clone(),
        _ => {
            return Err(syn::Error::new(
                tokens[pos].span(),
                "$if requires a brace body: $if(condition) { ... }",
            ));
        }
    };
    pos += 1;

    let first = parse_conditional_branch(
        &cond_group,
        "$if condition",
        "$if condition cannot be empty",
        &body_group,
        lang,
        &mut errors,
    );

    // Parse optional $else_if / $else continuations.
    loop {
        if pos + 1 >= tokens.len() {
            break;
        }
        let is_dollar = matches!(&tokens[pos], TokenTree::Punct(p) if p.as_char() == '$');
        if !is_dollar {
            break;
        }

        if is_ident(&tokens[pos + 1], "else_if") {
            pos += 2;
            if pos >= tokens.len() {
                return Ok(invalid_if_components(
                    pos,
                    &mut errors,
                    syn::Error::new(
                        tokens[pos - 1].span(),
                        "$else_if requires a parenthesized condition",
                    ),
                ));
            }
            let cond_group = match &tokens[pos] {
                TokenTree::Group(g) if g.delimiter() == Delimiter::Parenthesis => g.clone(),
                _ => {
                    let next_pos = malformed_else_if_boundary(tokens, pos);
                    return Ok(invalid_if_components(
                        next_pos,
                        &mut errors,
                        syn::Error::new(
                            tokens[pos].span(),
                            "$else_if requires a parenthesized condition: $else_if(condition) { ... }",
                        ),
                    ));
                }
            };
            pos += 1;
            if pos >= tokens.len() {
                return Ok(invalid_if_components(
                    pos,
                    &mut errors,
                    syn::Error::new(tokens[pos - 1].span(), "$else_if requires a brace body"),
                ));
            }
            let body_group = match &tokens[pos] {
                TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => g.clone(),
                _ => {
                    return Ok(invalid_if_components(
                        pos,
                        &mut errors,
                        syn::Error::new(
                            tokens[pos].span(),
                            "$else_if requires a brace body: $else_if(condition) { ... }",
                        ),
                    ));
                }
            };
            pos += 1;

            if let Some(branch) = parse_conditional_branch(
                &cond_group,
                "$else_if condition",
                "$else_if condition cannot be empty",
                &body_group,
                lang,
                &mut errors,
            ) {
                else_if.push(branch);
            }
        } else if is_ident(&tokens[pos + 1], "else") {
            pos += 2;
            if pos >= tokens.len() {
                return Ok(invalid_if_components(
                    pos,
                    &mut errors,
                    syn::Error::new(tokens[pos - 1].span(), "$else requires a brace body"),
                ));
            }
            let body_group = match &tokens[pos] {
                TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => g.clone(),
                _ => {
                    return Ok(invalid_if_components(
                        pos,
                        &mut errors,
                        syn::Error::new(
                            tokens[pos].span(),
                            "$else requires a brace body: $else { ... }",
                        ),
                    ));
                }
            };
            pos += 1;

            let body_tokens: Vec<TokenTree> = body_group.stream().into_iter().collect();
            match parse_body(&body_tokens, lang) {
                Ok(body) => otherwise = Some(body),
                Err(error) => combine(&mut errors, error),
            }
            break;
        } else {
            break;
        }
    }

    let meta_if = match (first, errors) {
        (_, Some(error)) => Err(error),
        (Some(first), None) => Ok(MetaIf {
            first,
            else_if,
            otherwise,
        }),
        (None, None) => Err(syn::Error::new(cond_group.span(), "invalid $if directive")),
    };

    Ok(Recovered {
        next_pos: pos,
        value: meta_if,
    })
}

fn invalid_if_components(
    next_pos: usize,
    errors: &mut Option<syn::Error>,
    error: syn::Error,
) -> IfComponents {
    combine(errors, error);
    Recovered {
        next_pos,
        value: Err(errors
            .take()
            .expect("the structural error was just recorded")),
    }
}

fn malformed_else_if_boundary(tokens: &[TokenTree], condition_pos: usize) -> usize {
    if matches!(tokens.get(condition_pos + 1), Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Brace)
    {
        condition_pos + 2
    } else {
        condition_pos
    }
}

fn parse_conditional_branch(
    condition_group: &proc_macro2::Group,
    condition_context: &str,
    empty_message: &str,
    body_group: &proc_macro2::Group,
    lang: MacroLang,
    errors: &mut Option<syn::Error>,
) -> Option<ConditionalBranch> {
    let condition = if condition_group.stream().is_empty() {
        Err(syn::Error::new(condition_group.span(), empty_message))
    } else {
        parse_expr(
            condition_group.stream(),
            condition_context,
            condition_group.span(),
        )
    };
    let body_tokens: Vec<TokenTree> = body_group.stream().into_iter().collect();
    let body = parse_body(&body_tokens, lang);

    match (condition, body) {
        (Ok(condition), Ok(body)) => Some(ConditionalBranch { condition, body }),
        (condition, body) => {
            if let Err(error) = condition {
                combine(errors, error);
            }
            if let Err(error) = body {
                combine(errors, error);
            }
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn for_parse_error(src: &str) -> syn::Error {
        let ts: TokenStream = src.parse().unwrap();
        let tokens: Vec<TokenTree> = ts.into_iter().collect();
        match parse_for_raw_components(&tokens, 0) {
            Ok(parts) => match parts.header {
                Ok(_) => panic!("expected $for parse error"),
                Err(error) => error,
            },
            Err(error) => error,
        }
    }

    fn for_error(src: &str) -> String {
        for_parse_error(src).to_string()
    }

    fn for_errors(src: &str) -> Vec<String> {
        for_parse_error(src)
            .into_iter()
            .map(|error| error.to_string())
            .collect()
    }

    fn assert_for_error_contains(src: &str, expected: &str) {
        let actual = for_error(src);
        assert!(
            actual.contains(expected),
            "expected error to contain {expected:?}, got {actual:?}"
        );
    }

    #[test]
    fn for_options_reject_empty_after_semicolon() {
        assert_for_error_contains(
            "(item in items;) { item }",
            "$for options cannot be empty after ';'; expected separator = expr or trailing = bool",
        );
    }

    #[test]
    fn for_options_reject_unknown_name() {
        assert_for_error_contains(
            r#"(item in items; sep = ",") { item }"#,
            "unknown $for option 'sep'; expected 'separator' or 'trailing'",
        );
    }

    #[test]
    fn for_options_reject_missing_equals() {
        assert_for_error_contains(
            r#"(item in items; separator ",") { item }"#,
            "$for option 'separator' requires '='",
        );
    }

    #[test]
    fn for_options_reject_empty_value() {
        assert_for_error_contains(
            "(item in items; separator =) { item }",
            "$for option 'separator' requires a value",
        );
    }

    #[test]
    fn for_options_reject_duplicate_separator() {
        assert_for_error_contains(
            r#"(item in items; separator = ",", separator = ";") { item }"#,
            "duplicate $for option 'separator'",
        );
    }

    #[test]
    fn for_options_reject_duplicate_trailing() {
        assert_for_error_contains(
            r#"(item in items; separator = ",", trailing = true, trailing = false) { item }"#,
            "duplicate $for option 'trailing'",
        );
    }

    #[test]
    fn for_options_reject_trailing_without_separator() {
        assert_for_error_contains(
            "(item in items; trailing = true) { item }",
            "$for option 'trailing' requires separator = expr",
        );
    }

    #[test]
    fn for_header_errors_are_combined() {
        let errors: Vec<String> =
            for_parse_error("(item + 1 in let; separator = struct, trailing = match) { item }")
                .into_iter()
                .map(|error| error.to_string())
                .collect();

        assert_eq!(errors.len(), 4, "{errors:?}");
        assert!(errors[0].contains("invalid $for pattern"), "{errors:?}");
        assert!(errors[1].contains("invalid $for iterator"), "{errors:?}");
        assert!(errors[2].contains("option 'separator'"), "{errors:?}");
        assert!(errors[3].contains("option 'trailing'"), "{errors:?}");
    }

    #[test]
    fn independent_for_option_errors_are_combined() {
        let errors = for_errors(
            r#"(item in items; separator = ",", separator = ";", unknown = true) { item }"#,
        );

        assert_eq!(errors.len(), 2, "{errors:?}");
        assert!(errors[0].contains("duplicate"), "{errors:?}");
        assert!(errors[1].contains("unknown"), "{errors:?}");
    }

    #[test]
    fn malformed_duplicate_option_reports_both_errors() {
        let errors = for_errors(r#"(item in items; separator = let, separator = ",") { item }"#);

        assert_eq!(errors.len(), 2, "{errors:?}");
        assert!(errors.iter().any(|error| error.contains("duplicate")));
        assert!(
            errors
                .iter()
                .any(|error| error.contains("option 'separator' expression"))
        );
    }

    #[test]
    fn malformed_unknown_option_reports_name_and_value_errors() {
        let errors = for_errors("(item in items; unknown = let) { item }");

        assert_eq!(errors.len(), 2, "{errors:?}");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("unknown $for option"))
        );
        assert!(
            errors
                .iter()
                .any(|error| error.contains("option 'unknown' expression"))
        );
    }

    #[test]
    fn conditional_branch_errors_are_combined() {
        let ts: TokenStream =
            "(let) { const first = $L(struct); } $else_if(match) { const second = $N(let); }"
                .parse()
                .unwrap();
        let tokens: Vec<TokenTree> = ts.into_iter().collect();
        let errors: Vec<String> = match parse_if_components(&tokens, 0, MacroLang::Unaware) {
            Err(error) => error.into_iter().map(|error| error.to_string()).collect(),
            Ok(parts) => match parts.value {
                Ok(_) => panic!("expected combined $if errors"),
                Err(error) => error.into_iter().map(|error| error.to_string()).collect(),
            },
        };

        assert_eq!(errors.len(), 4, "{errors:?}");
        assert!(errors[0].contains("$if condition"), "{errors:?}");
        assert!(errors[1].contains("invalid $L"), "{errors:?}");
        assert!(errors[2].contains("$else_if condition"), "{errors:?}");
        assert!(errors[3].contains("invalid $N"), "{errors:?}");
    }
}
