use proc_macro2::{Delimiter, Spacing, TokenStream, TokenTree};

use super::types::{CompileError, InterpolationKind, MacroLang, TypedArg};
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
    /// First `+` of `++` or first `-` of `--` used as postfix — suppress space before.
    PostfixIncDec,
    /// `*` used as postfix pointer marker (e.g. `Config*`) — suppress space before.
    PostfixStar,
    /// `?` used as postfix type marker (e.g. `Int?`, `String?`) — suppress space before.
    PostfixQuestion,
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
    /// `%W` soft-break — already provides a space, so suppress `maybe_space`.
    SoftBreak,
    /// `$$` literal dollar — suppress space after it so `$$1` renders as `$1`.
    DollarLiteral,
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
    /// `for (item : collection)` — space before `:`.
    ForRange,
}

/// Accumulated state threaded through the format-string builder.
pub(super) struct SpacingState {
    pub prev: PrevTokenKind,
    pub colon_ctx: ColonContext,
    /// End position (line, column) of the last specifier's closing group,
    /// used to detect adjacent specifiers like `$L("a")$L("b")`.
    pub prev_specifier_end: Option<(usize, usize)>,
}

impl SpacingState {
    pub fn new() -> Self {
        Self {
            prev: PrevTokenKind::None,
            colon_ctx: ColonContext::TypeAnnotation,
            prev_specifier_end: None,
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

#[rustfmt::skip]
const DECLARATION_KEYWORDS: &[&str] = &[
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
fn annotate_tokens(tokens: &[TokenTree], lang: MacroLang) -> Vec<TokenAnnotation> {
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
                // but NOT if it's `<<` (shift operator)
                if is_type_interp
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
                        if ch == '*' && i > 0 && matches!(&tokens[i - 1], TokenTree::Ident(_)) {
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
                        // (reference type like `auto&`, `int&`).
                        if ch == '&' && i > 0 && matches!(&tokens[i - 1], TokenTree::Ident(_)) {
                            let prev_end = tokens[i - 1].span().end();
                            let amp_start = p.span().start();
                            if prev_end.line == amp_start.line
                                && prev_end.column == amp_start.column
                            {
                                annotations[i] = TokenAnnotation::PostfixStar;
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
                        else if i > 0 {
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
                    '=' if i > 0 => {
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
                state.prev = PrevTokenKind::DollarLiteral;
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
                "expected interpolation kind after `$`: $T, $N, $S, $L, $C, $W, $join, $C_each, $for, or $$",
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
                state.prev = PrevTokenKind::GroupOpen;

                let inner: Vec<TokenTree> = g.stream().into_iter().collect();
                let inner_annotations = annotate_tokens(&inner, lang);
                tokens_to_format_inner(&inner, &inner_annotations, format, args, state, lang)?;

                state.colon_ctx = saved_ctx;
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

    // %W already provides a space (or newline), so don't add another.
    if prev == PrevTokenKind::SoftBreak {
        return;
    }

    // Annotation-based suppression (replaces old suppress_space flag).
    match annotation {
        TokenAnnotation::MacroBang
        | TokenAnnotation::GenericClose
        | TokenAnnotation::SafeCallQ
        | TokenAnnotation::PostfixIncDec
        | TokenAnnotation::PostfixStar
        | TokenAnnotation::PostfixQuestion
        | TokenAnnotation::ArrowOp
        | TokenAnnotation::AssignAdjacent
        | TokenAnnotation::MethodCallColon
        | TokenAnnotation::DashSep
        | TokenAnnotation::SlashSep => return,
        TokenAnnotation::GenericOpen if prev != PrevTokenKind::Keyword => return,
        _ => {}
    }

    // No space after prefix operators, path separators, or generic openers.
    if matches!(
        prev,
        PrevTokenKind::PrefixOp(_)
            | PrevTokenKind::PathSep
            | PrevTokenKind::GenericOpen
            | PrevTokenKind::DollarLiteral
    ) {
        return;
    }

    // No space before certain punctuation.
    if let PrevTokenKind::Punct(ch, _) = current {
        match ch {
            ',' | ';' | ')' | ']' => return,
            '.' if annotation != TokenAnnotation::DotArg => return,
            ':' if annotation != TokenAnnotation::DoubleColonOp => match state.colon_ctx {
                ColonContext::Ternary | ColonContext::WalrusAssign | ColonContext::ForRange => {}
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

    // No space before `(` or `[` when span-adjacent to preceding token
    // (function call, array index). Non-adjacent groups get default spacing.
    if annotation == TokenAnnotation::CallOpen {
        return;
    }

    // No space before a group when preceded by a specifier ($T, $N, etc.)
    // — handles `$T(x){}` struct literals and `$T(x)()` calls.
    if let PrevTokenKind::GroupOpen = current
        && prev == PrevTokenKind::Specifier
    {
        return;
    }

    // Default: add a space between tokens.
    format.push(' ');
}
