use proc_macro2::{Delimiter, TokenStream, TokenTree};

use super::parse_body;
use super::types::{CompileError, MacroLang, MetaBranch, Statement};
use super::util::is_ident;

type ForComponents = (
    usize,
    TokenStream,
    TokenStream,
    Option<TokenStream>,
    Option<TokenStream>,
    Vec<Statement>,
);

type ForRawComponents = (
    usize,
    TokenStream,
    TokenStream,
    Option<TokenStream>,
    Option<TokenStream>,
    Vec<TokenTree>,
);

/// Parse `(pat in expr[; separator = expr[, trailing = bool]]) { body }` at
/// `paren_pos` (after `$for` confirmed).
///
/// Returns `(next_pos, pat, iter_expr, separator, trailing, body_statements)`
/// where `next_pos` is the position after the closing `}` group.
pub(super) fn parse_for_components(
    tokens: &[TokenTree],
    paren_pos: usize,
    lang: MacroLang,
) -> Result<ForComponents, CompileError> {
    let (next_pos, pat, iter_expr, separator, trailing, body_tokens) =
        parse_for_raw_components(tokens, paren_pos)?;
    let body = parse_body(&body_tokens, lang)?;

    Ok((next_pos, pat, iter_expr, separator, trailing, body))
}

/// Parse `(pat in expr[; separator = expr[, trailing = bool]]) { body }` and
/// return raw body tokens. Statement-level callers parse the body as statements;
/// inline callers parse it as a format fragment.
pub(super) fn parse_for_raw_components(
    tokens: &[TokenTree],
    paren_pos: usize,
) -> Result<ForRawComponents, CompileError> {
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

    // Split paren contents on the first `in` keyword, then split loop options
    // after a top-level `;` so iterator expressions can still contain commas.
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
    let after_in = &paren_tokens[in_pos + 1..];
    let options_pos = after_in
        .iter()
        .position(|tt| matches!(tt, TokenTree::Punct(p) if p.as_char() == ';'));
    let (iter_tokens, option_tokens) = match options_pos {
        Some(pos) => (&after_in[..pos], Some(&after_in[pos + 1..])),
        None => (after_in, None),
    };

    let iter_expr: TokenStream = iter_tokens.iter().cloned().collect();
    if iter_expr.is_empty() {
        return Err(CompileError::new(
            paren_group.span(),
            "$for iterator expression cannot be empty: $for(pat in expr) { ... }",
        ));
    }

    let (separator, trailing) = match option_tokens {
        Some(tokens) => parse_for_options(tokens, paren_group.span())?,
        None => (None, None),
    };

    let body_tokens: Vec<TokenTree> = body_group.stream().into_iter().collect();

    Ok((
        paren_pos + 2,
        pat,
        iter_expr,
        separator,
        trailing,
        body_tokens,
    ))
}

fn parse_for_options(
    tokens: &[TokenTree],
    span: proc_macro2::Span,
) -> Result<(Option<TokenStream>, Option<TokenStream>), CompileError> {
    if tokens.is_empty() {
        return Err(CompileError::new(
            span,
            "$for options cannot be empty after ';'; expected separator = expr or trailing = bool",
        ));
    }

    let mut separator = None;
    let mut trailing = None;

    for option in split_for_options(tokens) {
        if option.is_empty() {
            return Err(CompileError::new(
                span,
                "$for option cannot be empty; expected separator = expr or trailing = bool",
            ));
        }

        let name = match option.first() {
            Some(TokenTree::Ident(id)) => id.to_string(),
            Some(tt) => {
                return Err(CompileError::new(
                    tt.span(),
                    "$for options must be named: separator = expr, trailing = bool",
                ));
            }
            None => unreachable!(),
        };

        let equals_pos = option
            .iter()
            .position(|tt| matches!(tt, TokenTree::Punct(p) if p.as_char() == '='));
        let equals_pos = match equals_pos {
            Some(pos) => pos,
            None => {
                return Err(CompileError::new(
                    option[0].span(),
                    format!("$for option '{name}' requires '='"),
                ));
            }
        };

        if equals_pos != 1 {
            return Err(CompileError::new(
                option[0].span(),
                format!("$for option '{name}' requires '=' immediately after the name"),
            ));
        }

        let value: TokenStream = option[equals_pos + 1..].iter().cloned().collect();
        if value.is_empty() {
            return Err(CompileError::new(
                option[0].span(),
                format!("$for option '{name}' requires a value"),
            ));
        }

        match name.as_str() {
            "separator" => {
                if separator.is_some() {
                    return Err(CompileError::new(
                        option[0].span(),
                        "duplicate $for option 'separator'",
                    ));
                }
                separator = Some(value);
            }
            "trailing" => {
                if trailing.is_some() {
                    return Err(CompileError::new(
                        option[0].span(),
                        "duplicate $for option 'trailing'",
                    ));
                }
                trailing = Some(value);
            }
            _ => {
                return Err(CompileError::new(
                    option[0].span(),
                    format!("unknown $for option '{name}'; expected 'separator' or 'trailing'"),
                ));
            }
        }
    }

    if trailing.is_some() && separator.is_none() {
        return Err(CompileError::new(
            span,
            "$for option 'trailing' requires separator = expr",
        ));
    }

    Ok((separator, trailing))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn for_error(src: &str) -> String {
        let ts: TokenStream = src.parse().unwrap();
        let tokens: Vec<TokenTree> = ts.into_iter().collect();
        parse_for_raw_components(&tokens, 0)
            .unwrap_err()
            .message()
            .to_owned()
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
}
