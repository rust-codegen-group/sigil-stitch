use proc_macro2::{Delimiter, Spacing, TokenStream, TokenTree};

use super::types::{CompileError, InterpolationKind, TypedArg};
use super::util::is_ident;

/// What kind of token was just emitted (for spacing decisions).
#[derive(Clone, Copy, PartialEq)]
pub(super) enum PrevTokenKind {
    None,
    Ident,
    Keyword,
    Punct(char, Spacing),
    Literal,
    GroupOpen,
    Specifier,
}

#[rustfmt::skip]
pub(super) const CONTROL_FLOW_KEYWORDS: &[&str] = &[
    "if", "else", "for", "while", "do", "switch", "catch",
    "synchronized", "when", "guard", "unless", "until",
    "elif", "elsif", "match", "case", "try", "with",
    "return", "throw", "yield", "await", "typeof", "instanceof",
    "in", "as", "is",
];

/// Convert a sequence of tokens into a format string and typed argument list.
///
/// Handles interpolation markers (`$T(expr)`, `$W`, `$$`, etc.) and
/// escapes `%` to `%%` in literal text. Recursively handles groups.
pub(crate) fn tokens_to_format(
    tokens: &[TokenTree],
) -> Result<(String, Vec<TypedArg>), CompileError> {
    let mut format = String::new();
    let mut args: Vec<TypedArg> = Vec::new();
    let mut prev_kind = PrevTokenKind::None;

    tokens_to_format_inner(tokens, &mut format, &mut args, &mut prev_kind)?;

    Ok((format, args))
}

fn tokens_to_format_inner(
    tokens: &[TokenTree],
    format: &mut String,
    args: &mut Vec<TypedArg>,
    prev_kind: &mut PrevTokenKind,
) -> Result<(), CompileError> {
    let mut pos = 0;

    while pos < tokens.len() {
        let tt = &tokens[pos];

        // Check for `$` interpolation.
        if let TokenTree::Punct(p) = tt
            && p.as_char() == '$'
        {
            pos += 1;
            if pos >= tokens.len() {
                return Err(CompileError::new(
                    p.span(),
                    "unexpected `$` at end of input",
                ));
            }

            let next = &tokens[pos];

            // `$$` -> literal `$`
            if let TokenTree::Punct(p2) = next
                && p2.as_char() == '$'
            {
                maybe_space(format, *prev_kind, PrevTokenKind::Literal);
                format.push('$');
                *prev_kind = PrevTokenKind::Literal;
                pos += 1;
                continue;
            }

            // `$>` -> `%>` (indent)
            if let TokenTree::Punct(p2) = next
                && p2.as_char() == '>'
            {
                format.push_str("%>");
                *prev_kind = PrevTokenKind::Specifier;
                pos += 1;
                continue;
            }

            // `$<` -> `%<` (dedent)
            if let TokenTree::Punct(p2) = next
                && p2.as_char() == '<'
            {
                format.push_str("%<");
                *prev_kind = PrevTokenKind::Specifier;
                pos += 1;
                continue;
            }

            // `$W` -> `%W` (no arg, no parens)
            if is_ident(next, "W") {
                format.push_str("%W");
                *prev_kind = PrevTokenKind::Specifier;
                pos += 1;
                continue;
            }

            // `$comment(...)` should have been caught earlier.
            if is_ident(next, "comment") {
                return Err(CompileError::new(
                    next.span(),
                    "$comment() must appear at the start of a line",
                ));
            }

            // `$C_each(...)` should have been caught earlier (statement-level).
            if is_ident(next, "C_each") {
                return Err(CompileError::new(
                    next.span(),
                    "$C_each() must appear at the start of a line",
                ));
            }

            // `$if`/`$else_if`/`$else` should have been caught earlier (statement-level).
            if is_ident(next, "if") || is_ident(next, "else_if") || is_ident(next, "else") {
                return Err(CompileError::new(
                    next.span(),
                    "$if/$else_if/$else must appear at the start of a line",
                ));
            }

            // `$join(sep, iter)` — inline join expression, emits as %L.
            if is_ident(next, "join") {
                pos += 1;
                if pos >= tokens.len() {
                    return Err(CompileError::new(
                        next.span(),
                        "$join requires parenthesized arguments: $join(sep, iter)",
                    ));
                }
                let group = match &tokens[pos] {
                    TokenTree::Group(g) if g.delimiter() == Delimiter::Parenthesis => g,
                    _ => {
                        return Err(CompileError::new(
                            tokens[pos].span(),
                            "$join requires parenthesized arguments: $join(sep, iter)",
                        ));
                    }
                };

                let (sep_expr, iter_expr) = split_join_args(group)?;

                maybe_space(format, *prev_kind, PrevTokenKind::Specifier);
                format.push_str("%L");
                *prev_kind = PrevTokenKind::Specifier;

                let join_expr: TokenStream = quote::quote! {
                    {
                        let __sigil_items: ::std::vec::Vec<::std::string::String> = (#iter_expr)
                            .into_iter()
                            .map(|__sigil_i| ::std::string::ToString::to_string(&__sigil_i))
                            .collect();
                        __sigil_items.join(#sep_expr)
                    }
                };

                args.push(TypedArg {
                    kind: InterpolationKind::Literal,
                    expr: join_expr,
                });

                pos += 1;
                continue;
            }

            // `$T(expr)`, `$N(expr)`, `$S(expr)`, `$L(expr)`, `$C(expr)`
            if let TokenTree::Ident(id) = next {
                let kind_str = id.to_string();
                let kind = match kind_str.as_str() {
                    "T" => InterpolationKind::Type,
                    "N" => InterpolationKind::Name,
                    "S" => InterpolationKind::StringLit,
                    "L" => InterpolationKind::Literal,
                    "C" => InterpolationKind::Code,
                    _ => {
                        return Err(CompileError::new(
                            id.span(),
                            format!(
                                "unknown interpolation kind `${kind_str}`. \
                                     Expected $T, $N, $S, $L, $C, $W, $join, or $C_each"
                            ),
                        ));
                    }
                };

                pos += 1;
                if pos >= tokens.len() {
                    return Err(CompileError::new(
                        id.span(),
                        format!(
                            "${kind_str} requires a parenthesized expression: ${kind_str}(expr)"
                        ),
                    ));
                }

                let group = match &tokens[pos] {
                    TokenTree::Group(g) if g.delimiter() == Delimiter::Parenthesis => g,
                    _ => {
                        return Err(CompileError::new(
                            tokens[pos].span(),
                            format!(
                                "${kind_str} requires a parenthesized expression: ${kind_str}(expr)"
                            ),
                        ));
                    }
                };

                let specifier = match kind {
                    InterpolationKind::Type => "%T",
                    InterpolationKind::Name => "%N",
                    InterpolationKind::StringLit => "%S",
                    InterpolationKind::Literal | InterpolationKind::Code => "%L",
                };

                maybe_space(format, *prev_kind, PrevTokenKind::Specifier);
                format.push_str(specifier);
                *prev_kind = PrevTokenKind::Specifier;

                args.push(TypedArg {
                    kind,
                    expr: group.stream(),
                });

                pos += 1;
                continue;
            }

            return Err(CompileError::new(
                next.span(),
                "expected interpolation kind after `$`: $T, $N, $S, $L, $C, $W, $join, $C_each, or $$",
            ));
        }

        // Regular tokens.
        match tt {
            TokenTree::Ident(id) => {
                let s = id.to_string();
                let kind = if CONTROL_FLOW_KEYWORDS.contains(&s.as_str()) {
                    PrevTokenKind::Keyword
                } else {
                    PrevTokenKind::Ident
                };
                maybe_space(format, *prev_kind, kind);
                format.push_str(&s.replace('%', "%%"));
                *prev_kind = kind;
            }
            TokenTree::Punct(p) => {
                let ch = p.as_char();
                let new_kind = PrevTokenKind::Punct(ch, p.spacing());
                maybe_space(format, *prev_kind, new_kind);
                if ch == '%' {
                    format.push_str("%%");
                } else {
                    format.push(ch);
                }
                *prev_kind = new_kind;
            }
            TokenTree::Literal(lit) => {
                maybe_space(format, *prev_kind, PrevTokenKind::Literal);
                let s = lit.to_string();
                format.push_str(&s.replace('%', "%%"));
                *prev_kind = PrevTokenKind::Literal;
            }
            TokenTree::Group(g) => {
                let (open, close) = match g.delimiter() {
                    Delimiter::Parenthesis => ("(", ")"),
                    Delimiter::Bracket => ("[", "]"),
                    Delimiter::Brace => ("{", "}"),
                    Delimiter::None => ("", ""),
                };
                let new_kind = PrevTokenKind::GroupOpen;
                maybe_space(format, *prev_kind, new_kind);
                format.push_str(open);
                *prev_kind = PrevTokenKind::GroupOpen;

                let inner: Vec<TokenTree> = g.stream().into_iter().collect();
                tokens_to_format_inner(&inner, format, args, prev_kind)?;

                format.push_str(close);
                *prev_kind = PrevTokenKind::Literal;
            }
        }
        pos += 1;
    }

    Ok(())
}

/// Split `$join(sep, iter)` arguments on the first top-level comma.
pub(super) fn split_join_args(
    group: &proc_macro2::Group,
) -> Result<(TokenStream, TokenStream), CompileError> {
    let tokens: Vec<TokenTree> = group.stream().into_iter().collect();
    let mut split_pos = None;

    for (i, tt) in tokens.iter().enumerate() {
        if let TokenTree::Punct(p) = tt
            && p.as_char() == ','
        {
            split_pos = Some(i);
            break;
        }
    }

    let split_pos = match split_pos {
        Some(p) => p,
        None => {
            return Err(CompileError::new(
                group.span(),
                "$join requires two arguments separated by comma: $join(sep, iter)",
            ));
        }
    };

    let sep_tokens: TokenStream = tokens[..split_pos].iter().cloned().collect();
    let iter_tokens: TokenStream = tokens[split_pos + 1..].iter().cloned().collect();

    if sep_tokens.is_empty() {
        return Err(CompileError::new(
            group.span(),
            "$join separator expression cannot be empty",
        ));
    }
    if iter_tokens.is_empty() {
        return Err(CompileError::new(
            group.span(),
            "$join iterable expression cannot be empty",
        ));
    }

    Ok((sep_tokens, iter_tokens))
}

/// Insert a space between the previous and current tokens if needed.
pub(super) fn maybe_space(format: &mut String, prev: PrevTokenKind, current: PrevTokenKind) {
    if prev == PrevTokenKind::None || prev == PrevTokenKind::GroupOpen {
        return;
    }

    // No space before certain punctuation.
    if let PrevTokenKind::Punct(ch, _) = current {
        match ch {
            ',' | ';' | ')' | ']' | '.' => return,
            ':' => {
                return;
            }
            _ => {}
        }
    }

    // No space between joint punctuation (===, !==, ->, ::, etc.).
    if let PrevTokenKind::Punct(_, Spacing::Joint) = prev {
        return;
    }

    // No space after opening punctuation.
    if let PrevTokenKind::Punct('(' | '[' | '.' | '!' | '~' | '@' | '#', _) = prev {
        return;
    }

    // No space before `(` when preceded by ident (function call), specifier, or literal.
    // Keywords get a space: `if (...)`, `while (...)`, `for (...)`.
    if let PrevTokenKind::GroupOpen = current
        && matches!(
            prev,
            PrevTokenKind::Ident | PrevTokenKind::Specifier | PrevTokenKind::Literal
        )
    {
        return;
    }

    // Default: add a space between tokens.
    format.push(' ');
}
