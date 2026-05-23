use proc_macro2::{Delimiter, Spacing, TokenStream, TokenTree};

use super::annotate::{
    CONTROL_FLOW_KEYWORDS, DECLARATION_KEYWORDS, TokenAnnotation, annotate_tokens,
};
use super::spacing::{ColonContext, PrevTokenKind, SpacingState, maybe_space};
use super::types::{CompileError, InterpolationKind, MacroLang, TypedArg};
use super::util::is_ident;

/// Convert a sequence of tokens into a format string and typed argument list.
///
/// Handles interpolation markers (`$T(expr)`, `$W`, `$$`, etc.) and
/// escapes `%` to `%%` in literal text. Recursively handles groups.
pub(crate) fn tokens_to_format(
    tokens: &[TokenTree],
    lang: MacroLang,
) -> Result<(String, Vec<TypedArg>), CompileError> {
    let mut format = String::new();
    let mut args: Vec<TypedArg> = Vec::new();
    let mut state = SpacingState::new();
    let annotations = annotate_tokens(tokens, lang);

    tokens_to_format_inner(
        tokens,
        &annotations,
        &mut format,
        &mut args,
        &mut state,
        lang,
    )?;

    Ok((format, args))
}

fn tokens_to_format_inner(
    tokens: &[TokenTree],
    annotations: &[TokenAnnotation],
    format: &mut String,
    args: &mut Vec<TypedArg>,
    state: &mut SpacingState,
    lang: MacroLang,
) -> Result<(), CompileError> {
    let mut pos = 0;

    while pos < tokens.len() {
        let tt = &tokens[pos];

        // Check for `$` interpolation.
        if let TokenTree::Punct(p) = tt
            && p.as_char() == '$'
        {
            // Check if this `$` is immediately adjacent to the previous
            // specifier's closing group (e.g. `$L("a")$L("b")` with no
            // whitespace). Used to suppress unwanted space insertion.
            let dollar_start = p.span().start();
            let adjacent_to_prev_specifier = state
                .prev_specifier_end
                .is_some_and(|(line, col)| dollar_start.line == line && dollar_start.column == col);

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
                if !adjacent_to_prev_specifier {
                    maybe_space(
                        format,
                        state,
                        PrevTokenKind::DollarLiteral,
                        TokenAnnotation::Normal,
                    );
                }
                format.push('$');
                // Haskell: `$` is an infix operator that needs space after it.
                // Other languages (shell): `$` glues to the next token (`$VAR`).
                state.prev = if lang == MacroLang::Haskell {
                    PrevTokenKind::Punct('$', Spacing::Alone)
                } else {
                    PrevTokenKind::DollarLiteral
                };
                state.prev_specifier_end = None;
                pos += 1;
                continue;
            }

            // `$>` -> `%>` (indent)
            if let TokenTree::Punct(p2) = next
                && p2.as_char() == '>'
            {
                format.push_str("%>");
                state.prev = PrevTokenKind::Specifier;
                state.prev_specifier_end = None;
                pos += 1;
                continue;
            }

            // `$<` -> `%<` (dedent)
            if let TokenTree::Punct(p2) = next
                && p2.as_char() == '<'
            {
                format.push_str("%<");
                state.prev = PrevTokenKind::Specifier;
                state.prev_specifier_end = None;
                pos += 1;
                continue;
            }

            // `$+` — line continuation marker (no-op, consumed by parser).
            if let TokenTree::Punct(p2) = next
                && p2.as_char() == '+'
            {
                state.prev_specifier_end = None;
                pos += 1;
                continue;
            }

            // `$W` -> `%W` (no arg, no parens)
            if is_ident(next, "W") {
                format.push_str("%W");
                state.prev = PrevTokenKind::SoftBreak;
                state.prev_specifier_end = None;
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

            // `$if`/`$else_if`/`$else`/`$for`/`$let` should have been caught earlier (statement-level).
            if is_ident(next, "if")
                || is_ident(next, "else_if")
                || is_ident(next, "else")
                || is_ident(next, "for")
                || is_ident(next, "let")
            {
                return Err(CompileError::new(
                    next.span(),
                    "$if/$else_if/$else/$for/$let must appear at the start of a line",
                ));
            }

            // `$T_join(sep, iter)` — inline type join; each item tracked via %T.
            if is_ident(next, "T_join") {
                pos += 1;
                if pos >= tokens.len() {
                    return Err(CompileError::new(
                        next.span(),
                        "$T_join requires parenthesized arguments: $T_join(sep, iter)",
                    ));
                }
                let group = match &tokens[pos] {
                    TokenTree::Group(g) if g.delimiter() == Delimiter::Parenthesis => g,
                    _ => {
                        return Err(CompileError::new(
                            tokens[pos].span(),
                            "$T_join requires parenthesized arguments: $T_join(sep, iter)",
                        ));
                    }
                };
                let (sep_expr, iter_expr) = split_join_args(group)?;

                if !adjacent_to_prev_specifier {
                    maybe_space(
                        format,
                        state,
                        PrevTokenKind::Specifier,
                        TokenAnnotation::Normal,
                    );
                }
                format.push_str("%L");
                state.prev = PrevTokenKind::Specifier;
                let group_end = group.span().end();
                state.prev_specifier_end = Some((group_end.line, group_end.column));

                let join_expr: TokenStream = quote::quote! {
                    {
                        let mut __sigil_cb =
                            ::sigil_stitch::code_block::CodeBlock::builder();
                        for (__sigil_idx, __sigil_i) in (#iter_expr).into_iter().enumerate() {
                            if __sigil_idx > 0 {
                                __sigil_cb.add("%L", (#sep_expr).to_string());
                            }
                            __sigil_cb.add("%T", __sigil_i.clone());
                        }
                        __sigil_cb.build().unwrap()
                    }
                };

                args.push(TypedArg {
                    kind: InterpolationKind::TypeJoin,
                    expr: join_expr,
                });

                pos += 1;
                continue;
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

                if !adjacent_to_prev_specifier {
                    maybe_space(
                        format,
                        state,
                        PrevTokenKind::Specifier,
                        TokenAnnotation::Normal,
                    );
                }
                format.push_str("%L");
                state.prev = PrevTokenKind::Specifier;
                let group_end = group.span().end();
                state.prev_specifier_end = Some((group_end.line, group_end.column));

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

            // `$T(expr)`, `$N(expr)`, `$S(expr)`, `$V(expr)`, `$L(expr)`, `$C(expr)`
            if let TokenTree::Ident(id) = next {
                let kind_str = id.to_string();
                let kind = match kind_str.as_str() {
                    "T" => InterpolationKind::Type,
                    "N" => InterpolationKind::Name,
                    "S" => InterpolationKind::StringLit,
                    "V" => InterpolationKind::VerbatimStr,
                    "L" => InterpolationKind::Literal,
                    "C" => InterpolationKind::Code,
                    _ => {
                        return Err(CompileError::new(
                            id.span(),
                            format!(
                                "unknown interpolation kind `${kind_str}`. \
                                     Expected $T, $N, $S, $V, $L, $C, $W, $T_join, $join, or $C_each"
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
                    InterpolationKind::VerbatimStr => "%V",
                    InterpolationKind::Literal
                    | InterpolationKind::Code
                    | InterpolationKind::TypeJoin => "%L",
                };

                if !adjacent_to_prev_specifier {
                    maybe_space(
                        format,
                        state,
                        PrevTokenKind::Specifier,
                        TokenAnnotation::Normal,
                    );
                }
                format.push_str(specifier);
                state.prev = PrevTokenKind::Specifier;
                let group_end = group.span().end();
                state.prev_specifier_end = Some((group_end.line, group_end.column));

                args.push(TypedArg {
                    kind,
                    expr: group.stream(),
                });

                pos += 1;
                continue;
            }

            return Err(CompileError::new(
                next.span(),
                "expected interpolation kind after `$`: $T, $N, $S, $V, $L, $C, $W, $T_join, $join, $C_each, $for, or $$",
            ));
        }

        let annotation = annotations[pos];

        // Regular (non-interpolation) token — clear specifier adjacency tracking.
        state.prev_specifier_end = None;

        // Regular tokens.
        match tt {
            TokenTree::Ident(id) => {
                let s = id.to_string();
                let kind = if CONTROL_FLOW_KEYWORDS.contains(&s.as_str())
                    || DECLARATION_KEYWORDS.contains(&s.as_str())
                {
                    PrevTokenKind::Keyword
                } else if s.starts_with(|c: char| c.is_uppercase()) {
                    PrevTokenKind::TypeIdent
                } else {
                    PrevTokenKind::Ident
                };
                maybe_space(format, state, kind, annotation);
                format.push_str(&s.replace('%', "%%"));
                state.prev = kind;
            }
            TokenTree::Punct(p) => {
                let ch = p.as_char();
                let new_kind = PrevTokenKind::Punct(ch, p.spacing());

                // Set colon context before spacing decision so `maybe_space`
                // can use it for the current `:` token.
                if ch == ':'
                    && p.spacing() == Spacing::Joint
                    && pos + 1 < tokens.len()
                    && let TokenTree::Punct(next_p) = &tokens[pos + 1]
                {
                    match next_p.as_char() {
                        '=' => state.colon_ctx = ColonContext::WalrusAssign,
                        ':' if annotations[pos + 1] == TokenAnnotation::PathSepComplete => {
                            state.colon_ctx = ColonContext::PathSeparator;
                        }
                        _ => {}
                    }
                }

                maybe_space(format, state, new_kind, annotation);
                if ch == '%' {
                    format.push_str("%%");
                } else {
                    format.push(ch);
                }
                // Context transitions after emitting the token.
                match (ch, p.spacing()) {
                    ('?', Spacing::Alone) => state.colon_ctx = ColonContext::Ternary,
                    (':', _) => state.colon_ctx = ColonContext::TypeAnnotation,
                    (';', _) => state.colon_ctx = ColonContext::TypeAnnotation,
                    _ => {}
                }
                // Set prev based on annotation.
                state.prev = match annotation {
                    TokenAnnotation::PathSepComplete => PrevTokenKind::PathSep,
                    TokenAnnotation::GenericOpen => PrevTokenKind::GenericOpen,
                    TokenAnnotation::PrefixOp => PrevTokenKind::PrefixOp(ch),
                    TokenAnnotation::DashFlag => PrevTokenKind::PrefixOp(ch),
                    TokenAnnotation::ArrowOp
                    | TokenAnnotation::AssignAdjacent
                    | TokenAnnotation::MethodCallColon
                    | TokenAnnotation::DashSep
                    | TokenAnnotation::SlashSep => PrevTokenKind::PathSep,
                    TokenAnnotation::DotArg => {
                        if p.spacing() == Spacing::Joint {
                            new_kind // Punct('.', Joint) — keeps `..` glued
                        } else {
                            PrevTokenKind::Literal // standalone `.` — allow space after
                        }
                    }
                    _ => new_kind,
                };
            }
            TokenTree::Literal(lit) => {
                maybe_space(format, state, PrevTokenKind::Literal, annotation);
                let s = lit.to_string();
                format.push_str(&s.replace('%', "%%"));
                state.prev = PrevTokenKind::Literal;
            }
            TokenTree::Group(g) => {
                let (open, close) = match g.delimiter() {
                    Delimiter::Parenthesis => ("(", ")"),
                    Delimiter::Bracket => ("[", "]"),
                    Delimiter::Brace => ("{", "}"),
                    Delimiter::None => ("", ""),
                };
                let shell_bracket = lang.is_shell()
                    && g.delimiter() == Delimiter::Bracket
                    && annotation != TokenAnnotation::CallOpen;
                // For `[[ ... ]]`, proc_macro2 nests as Bracket(Bracket(...)).
                // The outer bracket should NOT add inner spaces — only the
                // innermost bracket (which contains the actual tokens) should.
                let is_double_bracket_outer = shell_bracket
                    && {
                        let inner_tokens: Vec<TokenTree> = g.stream().into_iter().collect();
                        inner_tokens.len() == 1
                            && matches!(&inner_tokens[0], TokenTree::Group(ig) if ig.delimiter() == Delimiter::Bracket)
                    };
                let add_bracket_spaces = shell_bracket && !is_double_bracket_outer;
                let new_kind = PrevTokenKind::GroupOpen;
                maybe_space(format, state, new_kind, annotation);
                format.push_str(open);

                let saved_ctx = state.colon_ctx;
                if g.delimiter() == Delimiter::Brace {
                    state.colon_ctx = ColonContext::MapEntry;
                } else if g.delimiter() == Delimiter::Parenthesis
                    && pos > 0
                    && let TokenTree::Ident(id) = &tokens[pos - 1]
                    && *id == "for"
                {
                    state.colon_ctx = ColonContext::ForRange;
                }
                if add_bracket_spaces {
                    state.prev = PrevTokenKind::Literal;
                } else {
                    state.prev = PrevTokenKind::GroupOpen;
                }

                let inner: Vec<TokenTree> = g.stream().into_iter().collect();
                let inner_annotations = annotate_tokens(&inner, lang);
                tokens_to_format_inner(&inner, &inner_annotations, format, args, state, lang)?;

                state.colon_ctx = saved_ctx;
                if add_bracket_spaces {
                    format.push(' ');
                }
                format.push_str(close);

                // After a bracket group, check if the next token is span-adjacent.
                // If so, suppress space (e.g., `[]byte` in Go — the ident is directly
                // after `]`). Also handles `)(` when non-adjacent getting a space.
                let group_end = g.span().end();
                let next_adjacent = if pos + 1 < tokens.len() {
                    let next_start = tokens[pos + 1].span().start();
                    group_end.line == next_start.line && group_end.column == next_start.column
                } else {
                    false
                };
                if next_adjacent {
                    state.prev = PrevTokenKind::GroupOpen;
                } else {
                    state.prev = PrevTokenKind::Literal;
                }
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

#[cfg(test)]
#[path = "format_tests.rs"]
mod tests;
