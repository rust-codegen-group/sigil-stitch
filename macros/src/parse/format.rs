use proc_macro2::{Delimiter, Spacing, TokenStream, TokenTree};

use super::MacroLang;
use super::annotate::{
    CONTROL_FLOW_KEYWORDS, DECLARATION_KEYWORDS, TokenAnnotation, annotate_tokens,
};
use super::directives::{parse_for_raw_components, parse_if_components};
use super::recovery::combine;
use super::rust_interpolation::{parse_expr, parse_string_expr};
use super::spacing::{ColonContext, PrevTokenKind, SpacingState, maybe_space};
use super::util::is_ident;
use crate::ir::{FormattedCode, NoArgMarker, QuoteArg, Statement};

/// Convert a sequence of tokens into a format string and typed argument list.
///
/// Handles interpolation markers (`$T(expr)`, `$W`, `$$`, etc.) and
/// escapes `%` to `%%` in literal text. Recursively handles groups.
pub(crate) fn tokens_to_format(
    tokens: &[TokenTree],
    lang: MacroLang,
) -> syn::Result<FormattedCode> {
    let mut formatted = FormattedCode::new();
    let mut state = SpacingState::new(lang);
    let annotations = annotate_tokens(tokens, lang);

    tokens_to_format_inner(tokens, &annotations, &mut formatted, &mut state, lang)?;

    Ok(formatted)
}

fn tokens_to_format_inner(
    tokens: &[TokenTree],
    annotations: &[TokenAnnotation],
    formatted: &mut FormattedCode,
    state: &mut SpacingState,
    lang: MacroLang,
) -> syn::Result<()> {
    let mut pos = 0;
    let mut errors = None;
    let mut prev_end_line: Option<usize> = None;
    let base_column = tokens
        .first()
        .map(|tt| tt.span().start().column)
        .unwrap_or(0);

    while pos < tokens.len() {
        let tt = &tokens[pos];

        // Detect blank lines via span-location gaps between tokens.
        // Only emit for blank lines (gap >= 2), not consecutive lines.
        // Consecutive line breaks are handled by the statement parser.
        let tt_start = tt.span().start();
        let tt_line = tt_start.line;
        if let Some(prev_line) = prev_end_line {
            let gap = tt_line.saturating_sub(prev_line).saturating_sub(1);
            for _ in 0..gap {
                formatted.format_mut().push('\n');
                state.prev = PrevTokenKind::None;
            }
            if gap == 0 && tt_line > prev_line && preserves_newline_before_inline_meta(tokens, pos)
            {
                formatted.format_mut().push('\n');
                for _ in 0..tt_start.column.saturating_sub(base_column) {
                    formatted.format_mut().push(' ');
                }
                state.prev = PrevTokenKind::None;
            }
        }
        prev_end_line = Some(tt.span().end().line);
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
                return Err(syn::Error::new(p.span(), "unexpected `$` at end of input"));
            }

            let next = &tokens[pos];

            // `$$` -> literal `$`
            if let TokenTree::Punct(p2) = next
                && p2.as_char() == '$'
            {
                if !adjacent_to_prev_specifier {
                    maybe_space(
                        formatted.format_mut(),
                        state,
                        PrevTokenKind::DollarLiteral,
                        TokenAnnotation::Normal,
                    );
                }
                formatted.format_mut().push('$');
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
                formatted.push_marker(NoArgMarker::Indent);
                state.prev = PrevTokenKind::Specifier;
                state.prev_specifier_end = None;
                pos += 1;
                continue;
            }

            // `$<` -> `%<` (dedent)
            if let TokenTree::Punct(p2) = next
                && p2.as_char() == '<'
            {
                formatted.push_marker(NoArgMarker::Dedent);
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
                formatted.push_marker(NoArgMarker::SoftBreak);
                state.prev = PrevTokenKind::SoftBreak;
                state.prev_specifier_end = None;
                pos += 1;
                continue;
            }

            // `$comment(...)` — inline comment. Falls through to the
            // interpolation handler below.

            // `$C_each(...)` should have been caught earlier (statement-level).
            if is_ident(next, "C_each") {
                return Err(syn::Error::new(
                    next.span(),
                    "$C_each() must appear at the start of a line",
                ));
            }

            // `$for(pat in expr) { body }` — inline meta-for loop.
            if is_ident(next, "for") {
                pos += 1;
                if pos >= tokens.len() {
                    return Err(syn::Error::new(
                        next.span(),
                        "$for requires a parenthesized pattern: $for(pat in expr) { ... }",
                    ));
                }
                let parts = parse_for_raw_components(tokens, pos)?;
                let after_for = parts.next_pos;
                let body = tokens_to_format(&parts.body_tokens, lang);
                let (pat, iter_expr, separator, body) = match (parts.header, body) {
                    (Ok((pat, iter_expr, separator)), Ok(body)) => {
                        (pat, iter_expr, separator, body)
                    }
                    (header, body) => {
                        if let Err(error) = header {
                            combine(&mut errors, error);
                        }
                        if let Err(error) = body {
                            combine(&mut errors, error);
                        }
                        pos = after_for;
                        continue;
                    }
                };

                if !adjacent_to_prev_specifier {
                    maybe_space(
                        formatted.format_mut(),
                        state,
                        PrevTokenKind::Specifier,
                        TokenAnnotation::Normal,
                    );
                }
                state.prev = PrevTokenKind::ParsedSplice;
                let end = tokens[after_for - 1].span().end();
                state.prev_specifier_end = Some((end.line, end.column));

                formatted.push_argument(QuoteArg::ParsedSplice(vec![Statement::InlineFor {
                    pat,
                    iter_expr,
                    separator,
                    body,
                }]));
                pos = after_for;
                continue;
            }

            // `$if(cond) { body } [$else_if(cond) { body }]* [$else { body }]`
            // — inline meta-conditional.
            if is_ident(next, "if") {
                pos += 1;
                if pos >= tokens.len() {
                    return Err(syn::Error::new(
                        next.span(),
                        "$if requires a parenthesized condition: $if(condition) { ... }",
                    ));
                }
                let parts = parse_if_components(tokens, pos, lang)?;
                let after_if = parts.next_pos;
                let meta_if = match parts.value {
                    Ok(meta_if) => meta_if,
                    Err(error) => {
                        combine(&mut errors, error);
                        pos = after_if;
                        continue;
                    }
                };

                if !adjacent_to_prev_specifier {
                    maybe_space(
                        formatted.format_mut(),
                        state,
                        PrevTokenKind::Specifier,
                        TokenAnnotation::Normal,
                    );
                }
                state.prev = PrevTokenKind::ParsedSplice;
                let end = tokens[after_if - 1].span().end();
                state.prev_specifier_end = Some((end.line, end.column));

                formatted.push_argument(QuoteArg::ParsedSplice(vec![Statement::MetaIf(meta_if)]));
                pos = after_if;
                continue;
            }

            // `$else_if` / `$else` only valid as continuation of `$if`.
            if is_ident(next, "else_if") || is_ident(next, "else") {
                return Err(syn::Error::new(
                    next.span(),
                    "$else_if/$else must immediately follow a $if(condition) { ... } branch",
                ));
            }

            // `$let` is a Rust-level binding, only meaningful at statement level.
            if is_ident(next, "let") {
                return Err(syn::Error::new(
                    next.span(),
                    "$let must appear at the start of a line",
                ));
            }

            // `$T_join(sep, iter)` — inline type join; each item tracked via %T.
            if is_ident(next, "T_join") {
                pos += 1;
                if pos >= tokens.len() {
                    return Err(syn::Error::new(
                        next.span(),
                        "$T_join requires parenthesized arguments: $T_join(sep, iter)",
                    ));
                }
                let group = match &tokens[pos] {
                    TokenTree::Group(g) if g.delimiter() == Delimiter::Parenthesis => g,
                    _ => {
                        return Err(syn::Error::new(
                            tokens[pos].span(),
                            "$T_join requires parenthesized arguments: $T_join(sep, iter)",
                        ));
                    }
                };
                let (sep_expr, iter_expr) = match split_join_args(group) {
                    Ok(args) => args,
                    Err(error) => {
                        combine(&mut errors, error);
                        pos += 1;
                        continue;
                    }
                };

                if !adjacent_to_prev_specifier {
                    maybe_space(
                        formatted.format_mut(),
                        state,
                        PrevTokenKind::Specifier,
                        TokenAnnotation::Normal,
                    );
                }
                state.prev = PrevTokenKind::Specifier;
                let group_end = group.span().end();
                state.prev_specifier_end = Some((group_end.line, group_end.column));

                formatted.push_argument(QuoteArg::TypeJoin {
                    separator: sep_expr,
                    iter: iter_expr,
                });

                pos += 1;
                continue;
            }

            // `$join(sep, iter)` — inline join expression, emits as %L.
            if is_ident(next, "join") {
                pos += 1;
                if pos >= tokens.len() {
                    return Err(syn::Error::new(
                        next.span(),
                        "$join requires parenthesized arguments: $join(sep, iter)",
                    ));
                }
                let group = match &tokens[pos] {
                    TokenTree::Group(g) if g.delimiter() == Delimiter::Parenthesis => g,
                    _ => {
                        return Err(syn::Error::new(
                            tokens[pos].span(),
                            "$join requires parenthesized arguments: $join(sep, iter)",
                        ));
                    }
                };

                let (sep_expr, iter_expr) = match split_join_args(group) {
                    Ok(args) => args,
                    Err(error) => {
                        combine(&mut errors, error);
                        pos += 1;
                        continue;
                    }
                };

                if !adjacent_to_prev_specifier {
                    maybe_space(
                        formatted.format_mut(),
                        state,
                        PrevTokenKind::Specifier,
                        TokenAnnotation::Normal,
                    );
                }
                state.prev = PrevTokenKind::Specifier;
                let group_end = group.span().end();
                state.prev_specifier_end = Some((group_end.line, group_end.column));

                formatted.push_argument(QuoteArg::Join {
                    separator: sep_expr,
                    iter: iter_expr,
                });

                pos += 1;
                continue;
            }

            // `$T(expr)`, `$N(expr)`, `$S(expr)`, `$V(expr)`, `$L(expr)`, `$C(expr)`
            if let TokenTree::Ident(id) = next {
                let kind_str = id.to_string();
                if !matches!(
                    kind_str.as_str(),
                    "T" | "N" | "S" | "V" | "L" | "C" | "comment"
                ) {
                    let error = syn::Error::new(
                        id.span(),
                        format!(
                            "unknown interpolation kind `${kind_str}`. \
                                 Expected $T, $N, $S, $V, $L, $C, $W, $T_join, $join, $comment, $for, $if, or $C_each"
                        ),
                    );
                    if let Some(TokenTree::Group(group)) = tokens.get(pos + 1)
                        && group.delimiter() == Delimiter::Parenthesis
                    {
                        combine(&mut errors, error);
                        let group_end = group.span().end();
                        prev_end_line = Some(group_end.line);
                        state.prev = PrevTokenKind::None;
                        state.prev_specifier_end = None;
                        pos += 2;
                        continue;
                    }

                    combine(&mut errors, error);
                    return Err(errors.expect("the unknown marker error was just recorded"));
                }

                pos += 1;
                if pos >= tokens.len() {
                    return Err(syn::Error::new(
                        id.span(),
                        format!(
                            "${kind_str} requires a parenthesized expression: ${kind_str}(expr)"
                        ),
                    ));
                }

                let group = match &tokens[pos] {
                    TokenTree::Group(g) if g.delimiter() == Delimiter::Parenthesis => g,
                    _ => {
                        return Err(syn::Error::new(
                            tokens[pos].span(),
                            format!(
                                "${kind_str} requires a parenthesized expression: ${kind_str}(expr)"
                            ),
                        ));
                    }
                };

                let expr_tokens = group.stream();
                let expr_span = group.span();
                let arg = match kind_str.as_str() {
                    "T" => parse_expr(expr_tokens, "$T", expr_span).map(QuoteArg::Type),
                    "N" => parse_expr(expr_tokens, "$N", expr_span).map(QuoteArg::Name),
                    "S" => parse_expr(expr_tokens, "$S", expr_span).map(QuoteArg::StringLit),
                    "V" => {
                        parse_string_expr(expr_tokens, "$V", expr_span).map(QuoteArg::VerbatimStr)
                    }
                    "L" => parse_string_expr(expr_tokens, "$L", expr_span).map(QuoteArg::Literal),
                    "C" => parse_expr(expr_tokens, "$C", expr_span).map(QuoteArg::Code),
                    "comment" => {
                        parse_string_expr(expr_tokens, "$comment", expr_span).map(QuoteArg::Comment)
                    }
                    _ => Err(syn::Error::new(
                        id.span(),
                        "internal interpolation classification error",
                    )),
                };
                let arg = match arg {
                    Ok(arg) => arg,
                    Err(error) => {
                        combine(&mut errors, error);
                        pos += 1;
                        continue;
                    }
                };

                if !adjacent_to_prev_specifier {
                    maybe_space(
                        formatted.format_mut(),
                        state,
                        PrevTokenKind::Specifier,
                        TokenAnnotation::Normal,
                    );
                }
                state.prev = PrevTokenKind::Specifier;
                let group_end = group.span().end();
                state.prev_specifier_end = Some((group_end.line, group_end.column));

                formatted.push_argument(arg);

                pos += 1;
                continue;
            }

            return Err(syn::Error::new(
                next.span(),
                "expected interpolation kind after `$`: $T, $N, $S, $V, $L, $C, $W, $T_join, $join, $for, $if, or $$",
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
                maybe_space(formatted.format_mut(), state, kind, annotation);
                formatted.format_mut().push_str(&s.replace('%', "%%"));
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

                maybe_space(formatted.format_mut(), state, new_kind, annotation);
                if ch == '%' {
                    formatted.format_mut().push_str("%%");
                } else {
                    formatted.format_mut().push(ch);
                }
                // Context transitions after emitting the token.
                match (ch, p.spacing()) {
                    ('?', Spacing::Alone) => state.colon_ctx = ColonContext::Ternary,
                    (':', _)
                        if !matches!(
                            state.colon_ctx,
                            ColonContext::MapEntry | ColonContext::ForRange
                        ) =>
                    {
                        state.colon_ctx = if lang.default_colon_is_space_before() {
                            ColonContext::SpaceBefore
                        } else {
                            ColonContext::TypeAnnotation
                        };
                    }
                    // MapEntry (inside braces) or ForRange — preserve group-level context.
                    (':', _) => {}
                    (';', _) => {
                        state.colon_ctx = if lang.default_colon_is_space_before() {
                            ColonContext::SpaceBefore
                        } else {
                            ColonContext::TypeAnnotation
                        };
                    }
                    _ => {}
                }
                // Set prev based on annotation.
                state.prev = match annotation {
                    TokenAnnotation::PathSepComplete => PrevTokenKind::PathSep,
                    TokenAnnotation::GenericOpen => PrevTokenKind::GenericOpen,
                    TokenAnnotation::PrefixOp | TokenAnnotation::SymbolColon => {
                        PrevTokenKind::PrefixOp(ch)
                    }
                    TokenAnnotation::NullablePrefix => PrevTokenKind::PrefixOp('?'),
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
                maybe_space(
                    formatted.format_mut(),
                    state,
                    PrevTokenKind::Literal,
                    annotation,
                );
                let s = lit.to_string();
                formatted.format_mut().push_str(&s.replace('%', "%%"));
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
                maybe_space(formatted.format_mut(), state, new_kind, annotation);
                formatted.format_mut().push_str(open);

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
                // For parenthesized groups: if the first inner token is on a
                // different line from `(`, emit a newline so content starts on
                // a new line. This matches Go's paren-block behavior.
                // Braces and brackets already have language-specific newline
                // handling via begin_control_flow / block indentation.
                if g.delimiter() == Delimiter::Parenthesis
                    && let Some(first) = inner.first()
                {
                    let open_line = g.span().start().line;
                    let first_line = first.span().start().line;
                    if first_line > open_line {
                        formatted.format_mut().push('\n');
                        state.prev = PrevTokenKind::None;
                    }
                }
                let inner_annotations = annotate_tokens(&inner, lang);
                if let Err(error) =
                    tokens_to_format_inner(&inner, &inner_annotations, formatted, state, lang)
                {
                    combine(&mut errors, error);
                }

                if g.delimiter() == Delimiter::Parenthesis
                    && let Some(last) = inner.last()
                {
                    let last_line = last.span().end().line;
                    let close_line = g.span().end().line;
                    if close_line > last_line
                        && state.prev == PrevTokenKind::ParsedSplice
                        && !formatted.format().ends_with('\n')
                    {
                        formatted.format_mut().push('\n');
                        state.prev = PrevTokenKind::None;
                    }
                }

                state.colon_ctx = saved_ctx;
                if add_bracket_spaces {
                    formatted.format_mut().push(' ');
                }
                formatted.format_mut().push_str(close);

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

    match errors {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

fn preserves_newline_before_inline_meta(tokens: &[TokenTree], pos: usize) -> bool {
    if pos + 1 >= tokens.len() || !matches!(&tokens[pos], TokenTree::Punct(p) if p.as_char() == '$')
    {
        return false;
    }

    if is_ident(&tokens[pos + 1], "for") {
        return true;
    }

    if is_ident(&tokens[pos + 1], "if") {
        return !matches!(
            pos.checked_sub(1).and_then(|prev| tokens.get(prev)),
            Some(TokenTree::Punct(p)) if p.as_char() == '='
        );
    }

    false
}

/// Split `$join(sep, iter)` arguments on the first top-level comma.
pub(super) fn split_join_args(group: &proc_macro2::Group) -> syn::Result<(syn::Expr, syn::Expr)> {
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
            return Err(syn::Error::new(
                group.span(),
                "$join requires two arguments separated by comma: $join(sep, iter)",
            ));
        }
    };

    let sep_tokens: TokenStream = tokens[..split_pos].iter().cloned().collect();
    let iter_tokens: TokenStream = tokens[split_pos + 1..].iter().cloned().collect();

    if sep_tokens.is_empty() {
        return Err(syn::Error::new(
            group.span(),
            "$join separator expression cannot be empty",
        ));
    }
    if iter_tokens.is_empty() {
        return Err(syn::Error::new(
            group.span(),
            "$join iterable expression cannot be empty",
        ));
    }

    let separator = parse_expr(sep_tokens, "$join separator", group.span());
    let iterable = parse_expr(iter_tokens, "$join iterable", group.span());
    match (separator, iterable) {
        (Ok(separator), Ok(iterable)) => Ok((separator, iterable)),
        (Err(mut first), Err(second)) => {
            first.combine(second);
            Err(first)
        }
        (Err(error), Ok(_)) | (Ok(_), Err(error)) => Err(error),
    }
}

#[cfg(test)]
#[path = "format_tests.rs"]
mod tests;
