use proc_macro2::{Delimiter, TokenStream, TokenTree};

use super::parse_body;
use super::types::{CompileError, MacroLang, MetaBranch, Statement};
use super::util::is_ident;

/// Parse `(pat in expr) { body }` at `paren_pos` (after `$for` confirmed).
///
/// Returns `(next_pos, pat, iter_expr, body_statements)` where `next_pos`
/// is the position after the closing `}` group.
pub(super) fn parse_for_components(
    tokens: &[TokenTree],
    paren_pos: usize,
    lang: MacroLang,
) -> Result<(usize, TokenStream, TokenStream, Vec<Statement>), CompileError> {
    // Bounds checks.
    if paren_pos >= tokens.len() {
        return Err(CompileError::new(
            proc_macro2::Span::call_site(),
            "expected parenthesized pattern after $for",
        ));
    }
    if paren_pos + 1 >= tokens.len() {
        return Err(CompileError::new(
            proc_macro2::Span::call_site(),
            "$for requires a brace body: $for(pat in expr) { ... }",
        ));
    }

    let paren_group = match &tokens[paren_pos] {
        TokenTree::Group(g) if g.delimiter() == Delimiter::Parenthesis => g.clone(),
        _ => {
            return Err(CompileError::new(
                tokens[paren_pos].span(),
                "$for requires a parenthesized pattern: $for(pat in expr) { ... }",
            ));
        }
    };

    let body_group = match &tokens[paren_pos + 1] {
        TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => g.clone(),
        _ => {
            return Err(CompileError::new(
                tokens[paren_pos + 1].span(),
                "$for requires a brace body: $for(pat in expr) { ... }",
            ));
        }
    };

    // Split paren contents on the first `in` keyword.
    let paren_tokens: Vec<TokenTree> = paren_group.stream().into_iter().collect();
    let in_pos = paren_tokens.iter().position(|tt| is_ident(tt, "in"));
    let in_pos = match in_pos {
        Some(p) => p,
        None => {
            return Err(CompileError::new(
                paren_group.span(),
                "$for requires `in` keyword: $for(pat in expr) { ... }",
            ));
        }
    };

    if in_pos == 0 {
        return Err(CompileError::new(
            paren_group.span(),
            "$for pattern cannot be empty: $for(pat in expr) { ... }",
        ));
    }
    if in_pos + 1 >= paren_tokens.len() {
        return Err(CompileError::new(
            paren_group.span(),
            "$for iterator expression cannot be empty: $for(pat in expr) { ... }",
        ));
    }

    let pat: TokenStream = paren_tokens[..in_pos].iter().cloned().collect();
    let iter_expr: TokenStream = paren_tokens[in_pos + 1..].iter().cloned().collect();

    let body_tokens: Vec<TokenTree> = body_group.stream().into_iter().collect();
    let body = parse_body(&body_tokens, lang)?;

    Ok((paren_pos + 2, pat, iter_expr, body))
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
) -> Result<(usize, Vec<MetaBranch>), CompileError> {
    // Bounds check for first branch.
    if cond_pos >= tokens.len() {
        return Err(CompileError::new(
            proc_macro2::Span::call_site(),
            "expected parenthesized condition after $if",
        ));
    }
    if cond_pos + 1 >= tokens.len() {
        return Err(CompileError::new(
            proc_macro2::Span::call_site(),
            "$if requires a brace body: $if(condition) { ... }",
        ));
    }

    let mut pos = cond_pos;
    let mut branches = Vec::new();

    // Parse `(cond) { body }` for the $if branch.
    let cond_group = match &tokens[pos] {
        TokenTree::Group(g) if g.delimiter() == Delimiter::Parenthesis => g.clone(),
        _ => {
            return Err(CompileError::new(
                tokens[pos].span(),
                "$if requires a parenthesized condition: $if(condition) { ... }",
            ));
        }
    };
    if cond_group.stream().is_empty() {
        return Err(CompileError::new(
            cond_group.span(),
            "$if condition cannot be empty",
        ));
    }
    pos += 1;

    let body_group = match &tokens[pos] {
        TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => g.clone(),
        _ => {
            return Err(CompileError::new(
                tokens[pos].span(),
                "$if requires a brace body: $if(condition) { ... }",
            ));
        }
    };
    pos += 1;

    let body_tokens: Vec<TokenTree> = body_group.stream().into_iter().collect();
    let body = parse_body(&body_tokens, lang)?;
    branches.push(MetaBranch {
        condition: Some(cond_group.stream()),
        body,
    });

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
                return Err(CompileError::new(
                    tokens[pos - 1].span(),
                    "$else_if requires a parenthesized condition",
                ));
            }
            let cond_group = match &tokens[pos] {
                TokenTree::Group(g) if g.delimiter() == Delimiter::Parenthesis => g.clone(),
                _ => {
                    return Err(CompileError::new(
                        tokens[pos].span(),
                        "$else_if requires a parenthesized condition: $else_if(condition) { ... }",
                    ));
                }
            };
            if cond_group.stream().is_empty() {
                return Err(CompileError::new(
                    cond_group.span(),
                    "$else_if condition cannot be empty",
                ));
            }
            pos += 1;
            if pos >= tokens.len() {
                return Err(CompileError::new(
                    tokens[pos - 1].span(),
                    "$else_if requires a brace body",
                ));
            }
            let body_group = match &tokens[pos] {
                TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => g.clone(),
                _ => {
                    return Err(CompileError::new(
                        tokens[pos].span(),
                        "$else_if requires a brace body: $else_if(condition) { ... }",
                    ));
                }
            };
            pos += 1;

            let body_tokens: Vec<TokenTree> = body_group.stream().into_iter().collect();
            let body = parse_body(&body_tokens, lang)?;
            branches.push(MetaBranch {
                condition: Some(cond_group.stream()),
                body,
            });
        } else if is_ident(&tokens[pos + 1], "else") {
            pos += 2;
            if pos >= tokens.len() {
                return Err(CompileError::new(
                    tokens[pos - 1].span(),
                    "$else requires a brace body",
                ));
            }
            let body_group = match &tokens[pos] {
                TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => g.clone(),
                _ => {
                    return Err(CompileError::new(
                        tokens[pos].span(),
                        "$else requires a brace body: $else { ... }",
                    ));
                }
            };
            pos += 1;

            let body_tokens: Vec<TokenTree> = body_group.stream().into_iter().collect();
            let body = parse_body(&body_tokens, lang)?;
            branches.push(MetaBranch {
                condition: None,
                body,
            });
            break;
        } else {
            break;
        }
    }

    Ok((pos, branches))
}
