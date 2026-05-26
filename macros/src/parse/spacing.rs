use proc_macro2::Spacing;

use super::annotate::TokenAnnotation;

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
        | TokenAnnotation::NullablePrefix
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
            ':' if annotation != TokenAnnotation::DoubleColonOp
                && annotation != TokenAnnotation::SymbolColon =>
            {
                match state.colon_ctx {
                    ColonContext::Ternary | ColonContext::WalrusAssign | ColonContext::ForRange => {
                    }
                    ColonContext::TypeAnnotation
                    | ColonContext::MapEntry
                    | ColonContext::PathSeparator => return,
                }
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
