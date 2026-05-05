use proc_macro2::{Delimiter, Spacing, TokenStream, TokenTree};

use super::types::{CompileError, InterpolationKind, TypedArg};
use super::util::is_ident;

/// Annotations computed by pre-scanning the token stream.
/// Indexed by token position — each token gets one annotation.
#[derive(Clone, Copy, PartialEq, Default)]
pub(super) enum TokenAnnotation {
    #[default]
    Normal,
    /// Second `:` of `::` — suppress space after it.
    PathSepComplete,
    /// `<` used as generic opener (matched via stack).
    GenericOpen,
    /// `>` used as generic closer (matched via stack).
    GenericClose,
    /// `&` or `*` used as prefix operator (not binary).
    PrefixOp,
    /// `!` used as macro-call bang (after ident).
    MacroBang,
    /// `?` in `?.` safe-call — suppress space before it.
    SafeCallQ,
}

/// What kind of token was just emitted (for spacing decisions).
#[derive(Clone, Copy, PartialEq)]
pub(super) enum PrevTokenKind {
    None,
    Ident,
    TypeIdent,
    Keyword,
    Punct(char, Spacing),
    PrefixOp(char),
    PathSep,
    GenericOpen,
    Literal,
    GroupOpen,
    Specifier,
}

/// Context for how `:` should be spaced.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum ColonContext {
    /// `name: Type`, `param: Type` — no space before `:`.
    TypeAnnotation,
    /// `key: value` in map/object literals — no space before `:`.
    MapEntry,
    /// `cond ? a : b` — space before `:`.
    Ternary,
    /// `std::mem` — no space before `:`.
    PathSeparator,
    /// `x := 42` — space before `:`.
    WalrusAssign,
}

/// Accumulated state threaded through the format-string builder.
pub(super) struct SpacingState {
    pub prev: PrevTokenKind,
    pub colon_ctx: ColonContext,
}

impl SpacingState {
    pub fn new() -> Self {
        Self {
            prev: PrevTokenKind::None,
            colon_ctx: ColonContext::TypeAnnotation,
        }
    }
}

#[rustfmt::skip]
pub(super) const CONTROL_FLOW_KEYWORDS: &[&str] = &[
    "if", "else", "for", "while", "do", "switch", "catch",
    "synchronized", "when", "guard", "unless", "until",
    "elif", "elsif", "match", "case", "try", "with",
    "return", "throw", "yield", "await", "typeof", "instanceof",
    "in", "as", "is",
];

/// Pre-scan a token slice to classify each token for spacing decisions.
///
/// Skips `$`-prefixed interpolation markers (their contents are Rust
/// expressions, not target-language tokens).
fn annotate_tokens(tokens: &[TokenTree]) -> Vec<TokenAnnotation> {
    let mut annotations = vec![TokenAnnotation::Normal; tokens.len()];
    let mut generic_stack: Vec<usize> = Vec::new();
    let mut i = 0;

    while i < tokens.len() {
        let tt = &tokens[i];

        // Skip $ interpolation markers — mirrors the main pass logic.
        if let TokenTree::Punct(p) = tt
            && p.as_char() == '$'
        {
            i += 1;
            if i >= tokens.len() {
                break;
            }
            let next = &tokens[i];
            // $$ or $> or $< or $+ — skip one more token
            if let TokenTree::Punct(p2) = next
                && matches!(p2.as_char(), '$' | '>' | '<' | '+')
            {
                i += 1;
                continue;
            }
            // $W — skip one ident
            if is_ident(next, "W") {
                i += 1;
                continue;
            }
            // $join(expr) or $T(expr) etc — skip ident + group
            if let TokenTree::Ident(_) = next {
                i += 1;
                if i < tokens.len() && matches!(&tokens[i], TokenTree::Group(_)) {
                    i += 1;
                }
                continue;
            }
            continue;
        }

        match tt {
            TokenTree::Punct(p) => {
                let ch = p.as_char();
                match ch {
                    ':' => {
                        // PathSepComplete: first `:` is Joint and next is `:`
                        if p.spacing() == Spacing::Joint
                            && i + 1 < tokens.len()
                            && let TokenTree::Punct(next_p) = &tokens[i + 1]
                            && next_p.as_char() == ':'
                        {
                            annotations[i + 1] = TokenAnnotation::PathSepComplete;
                        }
                    }
                    '!' if p.spacing() == Spacing::Alone
                        && i > 0
                        && matches!(&tokens[i - 1], TokenTree::Ident(_)) =>
                    {
                        annotations[i] = TokenAnnotation::MacroBang;
                    }
                    '&' | '*' => {
                        // PrefixOp: NOT preceded by ident, literal, `)`, or `]`
                        let is_prefix = if i == 0 {
                            true
                        } else {
                            let prev = &tokens[i - 1];
                            match prev {
                                TokenTree::Ident(_) | TokenTree::Literal(_) => false,
                                TokenTree::Group(g) => !matches!(
                                    g.delimiter(),
                                    Delimiter::Parenthesis | Delimiter::Bracket
                                ),
                                TokenTree::Punct(pp) => {
                                    // After `)` or `]` from previous group close
                                    // tokens won't be Punct ')'/']' because those
                                    // are inside groups. After other punct → prefix.
                                    // But also: after `>` could be binary (rare),
                                    // let's be conservative and treat it as prefix.
                                    !matches!(pp.as_char(), ')' | ']')
                                }
                            }
                        };
                        if is_prefix {
                            annotations[i] = TokenAnnotation::PrefixOp;
                        }
                    }
                    '?' => {
                        // SafeCallQ: Joint `?` followed by `.`
                        if p.spacing() == Spacing::Joint
                            && i + 1 < tokens.len()
                            && let TokenTree::Punct(next_p) = &tokens[i + 1]
                            && next_p.as_char() == '.'
                        {
                            annotations[i] = TokenAnnotation::SafeCallQ;
                        }
                    }
                    '<' => {
                        // GenericOpen: preceded by uppercase Ident, PathSepComplete,
                        // or Joint `:` (turbofish).
                        // Joint `<` followed by another `<` is `<<` (shift), not generic.
                        let is_shift = p.spacing() == Spacing::Joint
                            && i + 1 < tokens.len()
                            && matches!(&tokens[i + 1], TokenTree::Punct(np) if np.as_char() == '<' || np.as_char() == '=');
                        let is_generic = if is_shift || i == 0 {
                            false
                        } else {
                            let prev = &tokens[i - 1];
                            match prev {
                                TokenTree::Ident(id) => {
                                    let s = id.to_string();
                                    // Uppercase ident (type name heuristic), OR
                                    // any ident preceded by PathSepComplete
                                    // (e.g., `std::map<` — lowercase but qualified)
                                    s.starts_with(|c: char| c.is_uppercase())
                                        || (i >= 2
                                            && annotations[i - 2]
                                                == TokenAnnotation::PathSepComplete)
                                }
                                TokenTree::Punct(pp) => {
                                    // After PathSepComplete or Joint `:` (turbofish)
                                    (pp.as_char() == ':'
                                        && annotations[i - 1] == TokenAnnotation::PathSepComplete)
                                        || (pp.as_char() == ':' && pp.spacing() == Spacing::Joint)
                                }
                                _ => false,
                            }
                        };
                        if is_generic {
                            annotations[i] = TokenAnnotation::GenericOpen;
                            generic_stack.push(i);
                        }
                    }
                    '>' if !generic_stack.is_empty() => {
                        generic_stack.pop();
                        annotations[i] = TokenAnnotation::GenericClose;
                    }
                    ';' => {
                        // Reset generic stack at statement boundaries
                        generic_stack.clear();
                    }
                    _ => {}
                }
            }
            TokenTree::Group(_) => {
                // Groups are handled recursively in the main pass —
                // they get their own annotation vector.
                // Don't clear generic_stack: generics can contain groups
                // (e.g., `Vec<(A, B)>`).
            }
            _ => {}
        }
        i += 1;
    }

    annotations
}

/// Convert a sequence of tokens into a format string and typed argument list.
///
/// Handles interpolation markers (`$T(expr)`, `$W`, `$$`, etc.) and
/// escapes `%` to `%%` in literal text. Recursively handles groups.
pub(crate) fn tokens_to_format(
    tokens: &[TokenTree],
) -> Result<(String, Vec<TypedArg>), CompileError> {
    let mut format = String::new();
    let mut args: Vec<TypedArg> = Vec::new();
    let mut state = SpacingState::new();
    let annotations = annotate_tokens(tokens);

    tokens_to_format_inner(tokens, &annotations, &mut format, &mut args, &mut state)?;

    Ok((format, args))
}

fn tokens_to_format_inner(
    tokens: &[TokenTree],
    annotations: &[TokenAnnotation],
    format: &mut String,
    args: &mut Vec<TypedArg>,
    state: &mut SpacingState,
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
                maybe_space(
                    format,
                    state,
                    PrevTokenKind::Literal,
                    TokenAnnotation::Normal,
                );
                format.push('$');
                state.prev = PrevTokenKind::Literal;
                pos += 1;
                continue;
            }

            // `$>` -> `%>` (indent)
            if let TokenTree::Punct(p2) = next
                && p2.as_char() == '>'
            {
                format.push_str("%>");
                state.prev = PrevTokenKind::Specifier;
                pos += 1;
                continue;
            }

            // `$<` -> `%<` (dedent)
            if let TokenTree::Punct(p2) = next
                && p2.as_char() == '<'
            {
                format.push_str("%<");
                state.prev = PrevTokenKind::Specifier;
                pos += 1;
                continue;
            }

            // `$+` — line continuation marker (no-op, consumed by parser).
            if let TokenTree::Punct(p2) = next
                && p2.as_char() == '+'
            {
                pos += 1;
                continue;
            }

            // `$W` -> `%W` (no arg, no parens)
            if is_ident(next, "W") {
                format.push_str("%W");
                state.prev = PrevTokenKind::Specifier;
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

                maybe_space(
                    format,
                    state,
                    PrevTokenKind::Specifier,
                    TokenAnnotation::Normal,
                );
                format.push_str("%L");
                state.prev = PrevTokenKind::Specifier;

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

                maybe_space(
                    format,
                    state,
                    PrevTokenKind::Specifier,
                    TokenAnnotation::Normal,
                );
                format.push_str(specifier);
                state.prev = PrevTokenKind::Specifier;

                args.push(TypedArg {
                    kind,
                    expr: group.stream(),
                });

                pos += 1;
                continue;
            }

            return Err(CompileError::new(
                next.span(),
                "expected interpolation kind after `$`: $T, $N, $S, $L, $C, $W, $join, $C_each, $for, or $$",
            ));
        }

        let annotation = annotations[pos];

        // Regular tokens.
        match tt {
            TokenTree::Ident(id) => {
                let s = id.to_string();
                let kind = if CONTROL_FLOW_KEYWORDS.contains(&s.as_str()) {
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
                        ':' => state.colon_ctx = ColonContext::PathSeparator,
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
                let new_kind = PrevTokenKind::GroupOpen;
                maybe_space(format, state, new_kind, annotation);
                format.push_str(open);

                let saved_ctx = state.colon_ctx;
                if g.delimiter() == Delimiter::Brace {
                    state.colon_ctx = ColonContext::MapEntry;
                }
                state.prev = PrevTokenKind::GroupOpen;

                let inner: Vec<TokenTree> = g.stream().into_iter().collect();
                let inner_annotations = annotate_tokens(&inner);
                tokens_to_format_inner(&inner, &inner_annotations, format, args, state)?;

                state.colon_ctx = saved_ctx;
                format.push_str(close);
                state.prev = PrevTokenKind::Literal;
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
pub(super) fn maybe_space(
    format: &mut String,
    state: &SpacingState,
    current: PrevTokenKind,
    annotation: TokenAnnotation,
) {
    let prev = state.prev;

    if prev == PrevTokenKind::None || prev == PrevTokenKind::GroupOpen {
        return;
    }

    // Annotation-based suppression (replaces old suppress_space flag).
    match annotation {
        TokenAnnotation::MacroBang
        | TokenAnnotation::GenericOpen
        | TokenAnnotation::GenericClose
        | TokenAnnotation::SafeCallQ => return,
        _ => {}
    }

    // No space after prefix operators, path separators, or generic openers.
    if matches!(
        prev,
        PrevTokenKind::PrefixOp(_) | PrevTokenKind::PathSep | PrevTokenKind::GenericOpen
    ) {
        return;
    }

    // No space before certain punctuation.
    if let PrevTokenKind::Punct(ch, _) = current {
        match ch {
            ',' | ';' | ')' | ']' | '.' => return,
            ':' => match state.colon_ctx {
                ColonContext::Ternary | ColonContext::WalrusAssign => {}
                ColonContext::TypeAnnotation
                | ColonContext::MapEntry
                | ColonContext::PathSeparator => return,
            },
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

    // No space before `(` when preceded by ident/type-ident (function call),
    // specifier, literal, or `>` (generic close then call, e.g. `size_of::<u32>()`).
    if let PrevTokenKind::GroupOpen = current
        && matches!(
            prev,
            PrevTokenKind::Ident
                | PrevTokenKind::TypeIdent
                | PrevTokenKind::Specifier
                | PrevTokenKind::Literal
                | PrevTokenKind::Punct('>', _)
        )
    {
        return;
    }

    // Default: add a space between tokens.
    format.push(' ');
}
