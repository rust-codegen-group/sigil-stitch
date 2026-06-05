use proc_macro2::{Delimiter, Spacing, TokenTree};

use super::types::MacroLang;
use super::util::is_ident;

/// Annotations computed by pre-scanning the token stream.
/// Indexed by token position — each token gets one annotation.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub(super) enum TokenAnnotation {
    #[default]
    Normal,
    /// Second `:` of `::` — suppress space after it.
    PathSepComplete,
    /// `<` used as generic opener (matched via stack).
    GenericOpen,
    /// `>` used as generic closer (matched via stack).
    GenericClose,
    /// `<` used as Ruby inheritance operator (`class Dog < Animal`).
    InheritanceAngle,
    /// `&` or `*` used as prefix operator (not binary).
    PrefixOp,
    /// `!` used as macro-call bang (after ident).
    MacroBang,
    /// `?` in `?.` safe-call — suppress space before it.
    SafeCallQ,
    /// First `+` of `++` or first `-` of `--` used as postfix — suppress space before.
    PostfixIncDec,
    /// `*` used as postfix pointer marker (e.g. `Config*`) — suppress space before.
    PostfixStar,
    /// `&` used as postfix reference marker (e.g. `auto&`, `int&`) — suppress space before.
    PostfixAmpersand,
    /// `?` used as postfix type marker (e.g. `Int?`, `String?`) — suppress space before.
    PostfixQuestion,
    /// `?` used as nullable prefix (e.g. `?User`, `?string`) — suppress space around.
    NullablePrefix,
    /// `-` starting `->` when adjacent to preceding token (member access, not type arrow).
    ArrowOp,
    /// First `:` of `::` used as operator (not path separator) — space before it.
    DoubleColonOp,
    /// Group open (paren/bracket) that is span-adjacent to preceding token —
    /// suppress space (function call, array index).
    CallOpen,
    /// `=` span-adjacent to preceding token (shell-style `NAME=val`) — suppress space.
    AssignAdjacent,
    /// `:` span-adjacent to both neighbors (Lua method call `obj:method()`).
    MethodCallColon,
    /// `:` used as Ruby symbol prefix (`:name`, `:foo`).
    SymbolColon,
    /// `-` that acts as flag prefix (like `-q`, `-f`). Does NOT suppress space
    /// before (so `declare -q` keeps the space). Only suppresses space after via
    /// `PrevTokenKind::PrefixOp`.
    DashFlag,
    /// `-` span-adjacent to both neighbors where prev is ident/literal
    /// (hyphenated word like `from-oci-layout`).
    DashSep,
    /// `/` span-adjacent to both neighbors (path separator like `linux/amd64`).
    SlashSep,
    /// `.` used as standalone argument in shell (e.g. `find .`), not member access.
    DotArg,
}

#[rustfmt::skip]
pub(super) const CONTROL_FLOW_KEYWORDS: &[&str] = &[
    "if", "else", "for", "while", "do", "switch", "catch",
    "synchronized", "when", "guard", "unless", "until",
    "elif", "elsif", "match", "case", "try", "with",
    "return", "throw", "yield", "await", "typeof", "instanceof",
    "in", "as", "is",
];

#[rustfmt::skip]
pub(super) const DECLARATION_KEYWORDS: &[&str] = &[
    "const", "let", "var", "val", "type", "fun", "def",
    "pub", "public", "private", "protected", "internal", "static", "final",
    "abstract", "async", "export", "import", "mut", "ref", "override",
    "virtual", "sealed", "lazy", "unsafe", "inline",
    "suspend", "defer", "go",
    "declare", "typeset", "local", "read", "readonly", "unset",
];

/// Pre-scan a token slice to classify each token for spacing decisions.
///
/// Skips `$`-prefixed interpolation markers (their contents are Rust
/// expressions, not target-language tokens).
pub(super) fn annotate_tokens(tokens: &[TokenTree], lang: MacroLang) -> Vec<TokenAnnotation> {
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
            if let TokenTree::Ident(id) = next {
                let is_type_interp = *id == "T";
                i += 1;
                if i < tokens.len() && matches!(&tokens[i], TokenTree::Group(_)) {
                    i += 1;
                }
                // $T(...) always produces a type — mark following `<` as generic
                // but NOT if it's `<<` (shift operator), and only for
                // languages that actually use `<>` angle-bracket generics.
                if is_type_interp
                    && lang.has_angle_generics()
                    && i < tokens.len()
                    && let TokenTree::Punct(p) = &tokens[i]
                    && p.as_char() == '<'
                {
                    // Check if this is `<<` (shift) by looking at spacing
                    let is_shift = p.spacing() == Spacing::Joint
                        && i + 1 < tokens.len()
                        && matches!(&tokens[i + 1], TokenTree::Punct(np) if np.as_char() == '<');
                    if !is_shift {
                        annotations[i] = TokenAnnotation::GenericOpen;
                        generic_stack.push(i);
                    }
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
                        // PathSepComplete: first `:` is Joint, next is `:`, and
                        // the `::` is span-adjacent to the preceding token
                        // (no whitespace before `::` → path separator like `std::fmt`).
                        // When user writes `fmap :: Type` with space, it's an operator.
                        if p.spacing() == Spacing::Joint
                            && i + 1 < tokens.len()
                            && let TokenTree::Punct(next_p) = &tokens[i + 1]
                            && next_p.as_char() == ':'
                        {
                            let is_path_sep = i > 0 && {
                                let prev_end = tokens[i - 1].span().end();
                                let colon_start = p.span().start();
                                prev_end.line == colon_start.line
                                    && prev_end.column == colon_start.column
                            };
                            if is_path_sep {
                                annotations[i + 1] = TokenAnnotation::PathSepComplete;
                            } else {
                                annotations[i] = TokenAnnotation::DoubleColonOp;
                            }
                        }
                        // MethodCallColon: single `:` span-adjacent to both neighbors
                        // (Lua `obj:method()`)
                        else if p.spacing() == Spacing::Alone && i > 0 && i + 1 < tokens.len() {
                            let prev_end = tokens[i - 1].span().end();
                            let colon_start = p.span().start();
                            let colon_end = p.span().end();
                            let next_start = tokens[i + 1].span().start();
                            if prev_end.line == colon_start.line
                                && prev_end.column == colon_start.column
                                && colon_end.line == next_start.line
                                && colon_end.column == next_start.column
                            {
                                annotations[i] = TokenAnnotation::MethodCallColon;
                            }
                        }

                        // Ruby symbol prefix: `:foo` — space before, no space after.
                        if lang == MacroLang::Ruby
                            && p.spacing() == Spacing::Alone
                            && i + 1 < tokens.len()
                            && matches!(&tokens[i + 1], TokenTree::Ident(_))
                        {
                            let colon_start = p.span().start();
                            let colon_end = p.span().end();
                            let next_start = tokens[i + 1].span().start();
                            let prev_adjacent = i > 0 && {
                                let prev_end = tokens[i - 1].span().end();
                                prev_end.line == colon_start.line
                                    && prev_end.column == colon_start.column
                            };
                            if !prev_adjacent
                                && colon_end.line == next_start.line
                                && colon_end.column == next_start.column
                            {
                                annotations[i] = TokenAnnotation::SymbolColon;
                            }
                        }
                    }
                    '!' if p.spacing() == Spacing::Alone
                        && i > 0
                        && matches!(&tokens[i - 1], TokenTree::Ident(_)) =>
                    {
                        annotations[i] = TokenAnnotation::MacroBang;
                    }
                    '+' | '-'
                        if p.spacing() == Spacing::Joint
                            && i + 1 < tokens.len()
                            && matches!(&tokens[i + 1], TokenTree::Punct(np) if np.as_char() == ch) =>
                    {
                        // ++ or -- : check if postfix (preceded by ident, literal, or group close)
                        let is_postfix = if i == 0 {
                            false
                        } else {
                            match &tokens[i - 1] {
                                TokenTree::Ident(id) => {
                                    let s = id.to_string();
                                    !CONTROL_FLOW_KEYWORDS.contains(&s.as_str())
                                }
                                TokenTree::Literal(_) => true,
                                TokenTree::Group(g) => matches!(
                                    g.delimiter(),
                                    Delimiter::Parenthesis | Delimiter::Bracket
                                ),
                                _ => false,
                            }
                        };
                        if is_postfix {
                            // Look-ahead: if the token after `++`/`--` is an operand
                            // (ident, literal, group, or `$` interpolation), this is a
                            // binary operator (e.g. Haskell `++`), not postfix inc/dec.
                            let after_second = i + 2;
                            let followed_by_operand = if after_second < tokens.len() {
                                match &tokens[after_second] {
                                    TokenTree::Ident(_)
                                    | TokenTree::Literal(_)
                                    | TokenTree::Group(_) => true,
                                    TokenTree::Punct(p2) => p2.as_char() == '$',
                                }
                            } else {
                                false
                            };
                            if !followed_by_operand {
                                annotations[i] = TokenAnnotation::PostfixIncDec;
                            }
                        }
                    }
                    '&' | '*' | '-' => {
                        // ArrowOp: `-` that forms `->` and is span-adjacent to
                        // preceding token (member access like `cfg->host`).
                        if ch == '-'
                            && p.spacing() == Spacing::Joint
                            && i + 1 < tokens.len()
                            && matches!(&tokens[i + 1], TokenTree::Punct(np) if np.as_char() == '>')
                            && i > 0
                        {
                            let prev_end = tokens[i - 1].span().end();
                            let cur_start = p.span().start();
                            if prev_end.line == cur_start.line
                                && prev_end.column == cur_start.column
                            {
                                annotations[i] = TokenAnnotation::ArrowOp;
                                annotations[i + 1] = TokenAnnotation::ArrowOp;
                                i += 2;
                                continue;
                            }
                        }

                        // `<-` compound operator: when `-` follows Joint `<` that
                        // isn't GenericOpen, the pair forms a binary operator (Haskell
                        // bind, Go channel). Don't mark as PrefixOp.
                        if ch == '-'
                            && i > 0
                            && let TokenTree::Punct(prev_p) = &tokens[i - 1]
                            && prev_p.as_char() == '<'
                            && prev_p.spacing() == Spacing::Joint
                            && annotations[i - 1] != TokenAnnotation::GenericOpen
                        {
                            // Go: `<-ch` is prefix receive — suppress space after
                            // if span-adjacent to next token. `ch <- 42` (send) has
                            // whitespace after `-`, so it keeps the space.
                            if lang == MacroLang::Go && i + 1 < tokens.len() {
                                let dash_end = p.span().end();
                                let next_start = tokens[i + 1].span().start();
                                if dash_end.line == next_start.line
                                    && dash_end.column == next_start.column
                                {
                                    annotations[i] = TokenAnnotation::PrefixOp;
                                }
                            }
                            i += 1;
                            continue;
                        }

                        // DashSep: `-` between two ident/literals with no whitespace
                        // on either side (hyphenated word like `from-oci-layout`).
                        // Requires prev to be Ident/Literal to avoid false positives
                        // (e.g. `-- file` where prev `-` is Punct).
                        if ch == '-'
                            && p.spacing() == Spacing::Alone
                            && i > 0
                            && i + 1 < tokens.len()
                            && matches!(&tokens[i - 1], TokenTree::Ident(_) | TokenTree::Literal(_))
                            && matches!(&tokens[i + 1], TokenTree::Ident(_) | TokenTree::Literal(_))
                        {
                            let prev_end = tokens[i - 1].span().end();
                            let dash_start = p.span().start();
                            let dash_end = p.span().end();
                            let next_start = tokens[i + 1].span().start();
                            if prev_end.line == dash_start.line
                                && prev_end.column == dash_start.column
                                && dash_end.line == next_start.line
                                && dash_end.column == next_start.column
                            {
                                annotations[i] = TokenAnnotation::DashSep;
                                i += 1;
                                continue;
                            }
                        }

                        // PostfixStar: `*` span-adjacent to preceding ident (pointer type like `Config*`).
                        // Only for languages with postfix pointer syntax (C, C++, C#).
                        if ch == '*'
                            && lang.has_postfix_star()
                            && i > 0
                            && matches!(&tokens[i - 1], TokenTree::Ident(_))
                        {
                            let prev_end = tokens[i - 1].span().end();
                            let star_start = p.span().start();
                            if prev_end.line == star_start.line
                                && prev_end.column == star_start.column
                            {
                                annotations[i] = TokenAnnotation::PostfixStar;
                                i += 1;
                                continue;
                            }
                        }

                        // PostfixAmpersand: `&` span-adjacent to preceding ident/keyword
                        // (reference type like `auto&`, `int&`). C++ only.
                        if ch == '&'
                            && lang.has_postfix_ampersand()
                            && i > 0
                            && matches!(&tokens[i - 1], TokenTree::Ident(_))
                        {
                            let prev_end = tokens[i - 1].span().end();
                            let amp_start = p.span().start();
                            if prev_end.line == amp_start.line
                                && prev_end.column == amp_start.column
                            {
                                annotations[i] = TokenAnnotation::PostfixAmpersand;
                                i += 1;
                                continue;
                            }
                        }

                        // PrefixOp: NOT preceded by non-keyword ident, literal, `)`, or `]`
                        // After keywords like `return`, `-` is prefix (unary minus)
                        let is_prefix = if i == 0 {
                            true
                        } else {
                            let prev = &tokens[i - 1];
                            match prev {
                                TokenTree::Ident(id) => {
                                    // After keyword → prefix; after variable → binary
                                    let s = id.to_string();
                                    CONTROL_FLOW_KEYWORDS.contains(&s.as_str())
                                        || DECLARATION_KEYWORDS.contains(&s.as_str())
                                }
                                TokenTree::Literal(_) => false,
                                TokenTree::Group(g) => !matches!(
                                    g.delimiter(),
                                    Delimiter::Parenthesis | Delimiter::Bracket
                                ),
                                TokenTree::Punct(pp) => !matches!(pp.as_char(), ')' | ']'),
                            }
                        };
                        if is_prefix {
                            annotations[i] = TokenAnnotation::PrefixOp;
                        }

                        // Shell: `-- file` separator. The second `-` of `--` gets
                        // PrefixOp above (prev is Punct '-'). In shell mode, if
                        // NOT span-adjacent to next ident, downgrade to Normal so
                        // the space is preserved (separator, not flag prefix).
                        if lang.is_shell()
                            && ch == '-'
                            && annotations[i] == TokenAnnotation::PrefixOp
                            && i > 0
                            && matches!(&tokens[i - 1], TokenTree::Punct(pp) if pp.as_char() == '-')
                            && i + 1 < tokens.len()
                            && matches!(&tokens[i + 1], TokenTree::Ident(_) | TokenTree::Literal(_))
                        {
                            let dash_end = p.span().end();
                            let next_start = tokens[i + 1].span().start();
                            if !(dash_end.line == next_start.line
                                && dash_end.column == next_start.column)
                            {
                                annotations[i] = TokenAnnotation::Normal;
                            }
                        }

                        // DashFlag: standalone `-` span-adjacent to following ident/literal.
                        // Overrides PrefixOp or Normal with DashFlag to suppress space after.
                        // Guards: must not be preceded by another `-` (avoids `-- file`
                        // false positive where proc_macro2 spans are unreliable).
                        if ch == '-'
                            && p.spacing() == Spacing::Alone
                            && i + 1 < tokens.len()
                            && matches!(&tokens[i + 1], TokenTree::Ident(_) | TokenTree::Literal(_))
                            && !(i > 0
                                && matches!(
                                    &tokens[i - 1],
                                    TokenTree::Punct(pp) if pp.as_char() == '-'
                                ))
                        {
                            let dash_end = p.span().end();
                            let next_start = tokens[i + 1].span().start();
                            if dash_end.line == next_start.line
                                && dash_end.column == next_start.column
                            {
                                annotations[i] = TokenAnnotation::DashFlag;
                                i += 1;
                                continue;
                            }
                        }
                    }
                    '/' if p.spacing() == Spacing::Alone && i + 1 < tokens.len() => {
                        let slash_end = p.span().end();
                        let next_start = tokens[i + 1].span().start();
                        let next_adj = slash_end.line == next_start.line
                            && slash_end.column == next_start.column;

                        if i > 0 {
                            // SlashSep: `/` span-adjacent to both neighbors (path like `linux/amd64`).
                            let prev_end = tokens[i - 1].span().end();
                            let slash_start = p.span().start();
                            if prev_end.line == slash_start.line
                                && prev_end.column == slash_start.column
                                && next_adj
                            {
                                annotations[i] = TokenAnnotation::SlashSep;
                            }
                        }
                        // Shell: leading `/` or non-adjacent prev — treat as path prefix
                        if lang.is_shell()
                            && annotations[i] != TokenAnnotation::SlashSep
                            && next_adj
                        {
                            annotations[i] = TokenAnnotation::SlashSep;
                        }
                    }
                    '.' if lang.is_shell() => {
                        // Shell: `.` not span-adjacent to prev is a standalone argument
                        // (e.g. `find .`, `cd ..`), not member access. Handles both
                        // Alone (single `.`) and Joint (first `.` of `..`).
                        // Guard: if the dot IS span-adjacent to the next non-dot token,
                        // it's a dotfile prefix (`.gitignore`) — keep as Normal.
                        let not_adj_to_prev = if i > 0 {
                            let prev_end = tokens[i - 1].span().end();
                            let dot_start = p.span().start();
                            !(prev_end.line == dot_start.line
                                && prev_end.column == dot_start.column)
                        } else {
                            true
                        };

                        if not_adj_to_prev {
                            // Check if this dot (or `..` sequence) is adjacent to
                            // the following non-dot token — if so, it's a dotfile.
                            let seq_end = if p.spacing() == Spacing::Joint
                                && i + 1 < tokens.len()
                                && matches!(&tokens[i + 1], TokenTree::Punct(p2) if p2.as_char() == '.')
                            {
                                i + 2 // `..` — check token after second dot
                            } else {
                                i + 1 // single `.` — check next token
                            };

                            let adj_to_next = if seq_end < tokens.len() {
                                let dot_seq_end = tokens[seq_end - 1].span().end();
                                let next_start = tokens[seq_end].span().start();
                                dot_seq_end.line == next_start.line
                                    && dot_seq_end.column == next_start.column
                            } else {
                                false
                            };

                            if !adj_to_next {
                                annotations[i] = TokenAnnotation::DotArg;
                                // If Joint (first of `..`), also mark the second dot
                                if seq_end == i + 2 {
                                    annotations[i + 1] = TokenAnnotation::DotArg;
                                }
                            }
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
                        // PostfixQuestion: `?` span-adjacent to preceding ident or group-close
                        // (e.g. `Int?`, `String?`, `(Int)?`)
                        // Only for languages with postfix nullable type syntax
                        // (C#, TS, Swift, Kotlin, Dart).
                        else if lang.has_postfix_question_type() && i > 0 {
                            let prev_end = tokens[i - 1].span().end();
                            let q_start = p.span().start();
                            let is_adjacent =
                                prev_end.line == q_start.line && prev_end.column == q_start.column;
                            if is_adjacent {
                                let is_type_context = match &tokens[i - 1] {
                                    TokenTree::Ident(_) => true,
                                    TokenTree::Group(g) => matches!(
                                        g.delimiter(),
                                        Delimiter::Parenthesis | Delimiter::Bracket
                                    ),
                                    _ => false,
                                };
                                if is_type_context {
                                    annotations[i] = TokenAnnotation::PostfixQuestion;
                                }
                            }
                        }
                        // NullablePrefix: `?` span-adjacent to a following ident —
                        // nullable type prefix like `?User` or `?string`.
                        // Only valid in languages that use `?` as nullable prefix.
                        if lang.nullable_prefix_is_valid()
                            && annotations[i] == TokenAnnotation::Normal
                            && i + 1 < tokens.len()
                            && matches!(&tokens[i + 1], TokenTree::Ident(_))
                        {
                            let q_end = p.span().end();
                            let next_start = tokens[i + 1].span().start();
                            if q_end.line == next_start.line && q_end.column == next_start.column {
                                annotations[i] = TokenAnnotation::NullablePrefix;
                            }
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
                                    // declaration keyword (e.g. `public <T>`), OR
                                    // any ident preceded by PathSepComplete
                                    // (e.g., `std::map<` — lowercase but qualified), OR
                                    // span-adjacent lowercase ident (`identity<T>`)
                                    s.starts_with(|c: char| c.is_uppercase())
                                        || DECLARATION_KEYWORDS.contains(&s.as_str())
                                        || (i >= 2
                                            && annotations[i - 2]
                                                == TokenAnnotation::PathSepComplete)
                                        || {
                                            let prev_end = id.span().end();
                                            let lt_start = p.span().start();
                                            prev_end.line == lt_start.line
                                                && prev_end.column == lt_start.column
                                        }
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
                            if lang == MacroLang::Ruby {
                                annotations[i] = TokenAnnotation::InheritanceAngle;
                            } else {
                                annotations[i] = TokenAnnotation::GenericOpen;
                                generic_stack.push(i);
                            }
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
                    '=' if lang.is_shell() && i > 0 => {
                        let prev_is_assignable = matches!(
                            &tokens[i - 1],
                            TokenTree::Ident(_) | TokenTree::Literal(_) | TokenTree::Group(_)
                        );
                        if prev_is_assignable {
                            let prev_end = tokens[i - 1].span().end();
                            let eq_start = p.span().start();
                            if prev_end.line == eq_start.line && prev_end.column == eq_start.column
                            {
                                // Don't mark as AssignAdjacent if this is part of
                                // ==, =>, <=, >= (Joint followed by = or >)
                                let is_compound = p.spacing() == Spacing::Joint
                                    && i + 1 < tokens.len()
                                    && matches!(&tokens[i + 1], TokenTree::Punct(np) if np.as_char() == '=' || np.as_char() == '>');
                                if !is_compound {
                                    annotations[i] = TokenAnnotation::AssignAdjacent;
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            TokenTree::Group(g)
                if i > 0
                    && matches!(g.delimiter(), Delimiter::Parenthesis | Delimiter::Bracket) =>
            {
                // Mark groups that are span-adjacent to the preceding ident/literal/>
                // as CallOpen (function call / array index). Non-adjacent groups
                // or groups after operators/keywords get default spacing.
                let prev_is_callable = match &tokens[i - 1] {
                    TokenTree::Ident(id) => {
                        let s = id.to_string();
                        let is_keyword = CONTROL_FLOW_KEYWORDS.contains(&s.as_str())
                            || DECLARATION_KEYWORDS.contains(&s.as_str());
                        if !is_keyword {
                            true
                        } else if i >= 2 {
                            matches!(&tokens[i - 2], TokenTree::Punct(p) if p.as_char() == '.')
                        } else {
                            false
                        }
                    }
                    TokenTree::Literal(_) | TokenTree::Group(_) => true,
                    TokenTree::Punct(p) => p.as_char() == '>',
                };
                if prev_is_callable {
                    let prev_end = tokens[i - 1].span().end();
                    let group_start = g.span().start();
                    if prev_end.line == group_start.line && prev_end.column == group_start.column {
                        annotations[i] = TokenAnnotation::CallOpen;
                    }
                }
            }
            _ => {}
        }
        i += 1;
    }

    annotations
}

#[cfg(test)]
#[path = "annotate_tests.rs"]
mod tests;
