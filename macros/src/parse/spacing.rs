use proc_macro2::{Delimiter, Group, Spacing, TokenTree};

use super::MacroLang;
use super::annotate::TokenAnnotation;
use crate::ir::LayoutSpec;

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
    /// Inline `$for`/`$if` parsed splice emitted as `%L`.
    ParsedSplice,
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
    /// Explicit space-before-colon for languages where `:` is not a type annotation
    /// (OCaml, Shell). Behaves like `Ternary`/`ForRange` — allows space before `:`.
    SpaceBefore,
}

/// Spacing/layout policy selected by the macro language.
///
/// The shared parser carries this value by copy. Python grammar context stays
/// private to the Python variant; another language can add a policy without
/// adding fields to `SpacingState` or threading a language-specific scope.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum MacroSpacingPolicy {
    Generic,
    Python(PythonContext),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct PythonContext {
    role: PythonGroupRole,
    layout: bool,
    comprehension_target: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PythonGroupRole {
    Statement,
    CallArguments,
    FunctionParameters,
    LambdaParameters,
    TypeArguments,
    List,
    Tuple,
    Dict,
    Set,
    Subscript,
    Comprehension,
    Decorator,
    ControlHeader,
    Parenthesized,
}

impl MacroSpacingPolicy {
    pub(super) fn for_language(lang: MacroLang) -> Self {
        if lang.is_python() {
            Self::Python(PythonContext {
                role: PythonGroupRole::Statement,
                layout: false,
                comprehension_target: false,
            })
        } else {
            Self::Generic
        }
    }

    pub(super) fn with_layout(self) -> Self {
        match self {
            Self::Generic => Self::Generic,
            Self::Python(mut context) => {
                context.layout = true;
                Self::Python(context)
            }
        }
    }

    pub(super) fn enter_group(
        self,
        tokens: &[TokenTree],
        pos: usize,
        group: &Group,
        annotation: TokenAnnotation,
        inner: &[TokenTree],
    ) -> Self {
        let Self::Python(parent) = self else {
            return Self::Generic;
        };
        let role = if group.delimiter() == Delimiter::None {
            PythonGroupRole::Parenthesized
        } else if python_contains_comprehension_for(inner)
            && !python_control_header_before(tokens, pos)
        {
            PythonGroupRole::Comprehension
        } else {
            match group.delimiter() {
                Delimiter::Brace => {
                    if python_contains_punct(inner, ':') {
                        PythonGroupRole::Dict
                    } else {
                        PythonGroupRole::Set
                    }
                }
                Delimiter::Bracket => {
                    if annotation == TokenAnnotation::CallOpen {
                        if python_type_like_before(tokens, pos) {
                            PythonGroupRole::TypeArguments
                        } else {
                            PythonGroupRole::Subscript
                        }
                    } else {
                        PythonGroupRole::List
                    }
                }
                Delimiter::Parenthesis => {
                    if python_decorator_before(tokens, pos) {
                        PythonGroupRole::Decorator
                    } else if python_control_header_before(tokens, pos) {
                        PythonGroupRole::ControlHeader
                    } else if python_function_parameter_group(tokens, pos) {
                        PythonGroupRole::FunctionParameters
                    } else if annotation == TokenAnnotation::CallOpen {
                        PythonGroupRole::CallArguments
                    } else if python_contains_punct(inner, ',') {
                        PythonGroupRole::Tuple
                    } else {
                        PythonGroupRole::Parenthesized
                    }
                }
                Delimiter::None => PythonGroupRole::Parenthesized,
            }
        };
        Self::Python(PythonContext {
            role,
            layout: parent.layout,
            comprehension_target: false,
        })
    }

    pub(super) fn annotate_punctuation(
        self,
        tokens: &[TokenTree],
        pos: usize,
        annotation: TokenAnnotation,
    ) -> TokenAnnotation {
        let Self::Python(context) = self else {
            return annotation;
        };
        let Some(TokenTree::Punct(punctuation)) = tokens.get(pos) else {
            return annotation;
        };
        let mut annotation = annotation;
        if punctuation.as_char() == ':' && context.role == PythonGroupRole::Subscript {
            annotation = TokenAnnotation::SliceColon;
        }
        if punctuation.as_char() == '.'
            && pos > 0
            && matches!(&tokens[pos - 1], TokenTree::Ident(id) if id == "from")
        {
            annotation = TokenAnnotation::DotArg;
        }
        if punctuation.as_char() == '=' && context.is_assign_adjacent(tokens, pos) {
            annotation = TokenAnnotation::AssignAdjacent;
        }
        annotation
    }

    pub(super) fn after_ident(self, ident: &str) -> Self {
        match self {
            Self::Python(context)
                if ident == "lambda" && context.role == PythonGroupRole::Statement =>
            {
                Self::Python(PythonContext {
                    role: PythonGroupRole::LambdaParameters,
                    layout: context.layout,
                    comprehension_target: context.comprehension_target,
                })
            }
            Self::Python(context) if context.role == PythonGroupRole::Comprehension => {
                let mut context = context;
                if ident == "for" {
                    context.comprehension_target = true;
                } else if ident == "in" {
                    context.comprehension_target = false;
                }
                Self::Python(context)
            }
            _ => self,
        }
    }

    pub(super) fn after_punctuation(self, punctuation: char) -> Self {
        match self {
            Self::Python(context)
                if punctuation == ':' && context.role == PythonGroupRole::LambdaParameters =>
            {
                Self::Python(PythonContext {
                    role: PythonGroupRole::Statement,
                    layout: context.layout,
                    comprehension_target: context.comprehension_target,
                })
            }
            _ => self,
        }
    }

    pub(super) fn break_after_comma(self) -> bool {
        matches!(
            self,
            Self::Python(PythonContext {
                layout: true,
                comprehension_target: false,
                ..
            })
        )
    }

    pub(super) fn is_dict_group(self) -> bool {
        matches!(
            self,
            Self::Python(PythonContext {
                role: PythonGroupRole::Dict,
                ..
            })
        )
    }

    pub(super) fn break_before_comprehension_clause(self, ident: &str) -> bool {
        matches!(
            self,
            Self::Python(PythonContext {
                role: PythonGroupRole::Comprehension,
                layout: true,
                ..
            }) if matches!(ident, "for" | "if")
        )
    }

    pub(super) fn needs_trailing_comma(self) -> bool {
        matches!(
            self,
            Self::Python(PythonContext {
                role: PythonGroupRole::CallArguments
                    | PythonGroupRole::FunctionParameters
                    | PythonGroupRole::LambdaParameters
                    | PythonGroupRole::TypeArguments
                    | PythonGroupRole::List
                    | PythonGroupRole::Tuple
                    | PythonGroupRole::Dict
                    | PythonGroupRole::Set
                    | PythonGroupRole::Subscript
                    | PythonGroupRole::Decorator,
                ..
            })
        )
    }

    pub(super) fn layout_spec(
        self,
        group: &Group,
        inner: &[TokenTree],
        close: &str,
    ) -> Option<LayoutSpec> {
        if !matches!(self, Self::Python(_))
            || group.delimiter() == Delimiter::None
            || python_group_has_explicit_newline(group, inner)
        {
            return None;
        }
        Some(LayoutSpec {
            consistent: true,
            empty_flat_break: false,
            edge_breaks: true,
            trailing_comma: self.needs_trailing_comma() && python_contains_punct(inner, ','),
            flat_prefix: String::new(),
            flat_suffix: close.to_string(),
            broken_prefix: String::new(),
            broken_suffix: close.to_string(),
        })
    }
}

impl PythonContext {
    fn is_assign_adjacent(self, tokens: &[TokenTree], pos: usize) -> bool {
        match self.role {
            PythonGroupRole::CallArguments => python_keyword_equals(tokens, pos),
            PythonGroupRole::FunctionParameters | PythonGroupRole::LambdaParameters => {
                python_default_equals(tokens, pos)
            }
            _ => false,
        }
    }
}

fn python_group_has_explicit_newline(group: &Group, inner: &[TokenTree]) -> bool {
    let Some(first) = inner.first() else {
        return false;
    };
    let Some(last) = inner.last() else {
        return false;
    };
    first.span().start().line > group.span().start().line
        || group.span().end().line > last.span().end().line
}

fn python_function_parameter_group(tokens: &[TokenTree], pos: usize) -> bool {
    if pos < 2 {
        return false;
    }
    let is_def = |index: usize| matches!(&tokens[index], TokenTree::Ident(id) if id == "def");
    matches!(&tokens[pos - 1], TokenTree::Ident(_)) && is_def(pos - 2)
}

fn python_contains_comprehension_for(tokens: &[TokenTree]) -> bool {
    tokens.iter().enumerate().any(|(index, token)| {
        matches!(token, TokenTree::Ident(value) if value == "for")
            && !matches!(
                index.checked_sub(1).and_then(|previous| tokens.get(previous)),
                Some(TokenTree::Punct(dollar)) if dollar.as_char() == '$'
            )
    })
}

fn python_contains_punct(tokens: &[TokenTree], punctuation: char) -> bool {
    tokens
        .iter()
        .any(|token| matches!(token, TokenTree::Punct(value) if value.as_char() == punctuation))
}

fn python_control_header_before(tokens: &[TokenTree], pos: usize) -> bool {
    pos > 0
        && matches!(
            &tokens[pos - 1],
            TokenTree::Ident(keyword)
                if matches!(keyword.to_string().as_str(), "if" | "for" | "while" | "with" | "match")
        )
}

fn python_decorator_before(tokens: &[TokenTree], pos: usize) -> bool {
    pos >= 2 && matches!(&tokens[pos - 2], TokenTree::Punct(at) if at.as_char() == '@')
}

fn python_type_like_before(tokens: &[TokenTree], pos: usize) -> bool {
    pos > 0
        && match &tokens[pos - 1] {
            TokenTree::Ident(identifier) => identifier
                .to_string()
                .chars()
                .next()
                .is_some_and(char::is_uppercase),
            TokenTree::Group(_) => true,
            TokenTree::Punct(punctuation) => punctuation.as_char() == '>',
            _ => false,
        }
}

pub(super) fn python_keyword_equals(tokens: &[TokenTree], pos: usize) -> bool {
    let Some(TokenTree::Punct(current)) = tokens.get(pos) else {
        return false;
    };
    if current.as_char() != '=' {
        return false;
    }
    if pos + 1 < tokens.len()
        && matches!(&tokens[pos + 1], TokenTree::Punct(next) if next.as_char() == '=' || next.as_char() == '>')
        && current.spacing() == Spacing::Joint
    {
        return false;
    }
    if pos > 0 {
        match &tokens[pos - 1] {
            TokenTree::Punct(previous)
                if matches!(
                    previous.as_char(),
                    '!' | '<' | '>' | ':' | '+' | '-' | '*' | '/' | '%' | '&' | '|' | '^' | '@'
                ) =>
            {
                return false;
            }
            _ => {}
        }
    }
    match tokens.get(pos.wrapping_sub(1)) {
        Some(TokenTree::Ident(_)) => true,
        Some(TokenTree::Group(_)) if pos >= 3 => {
            matches!(&tokens[pos - 2], TokenTree::Ident(kind) if kind == "N")
                && matches!(&tokens[pos - 3], TokenTree::Punct(dollar) if dollar.as_char() == '$')
        }
        _ => false,
    }
}

pub(super) fn python_default_equals(tokens: &[TokenTree], pos: usize) -> bool {
    if !python_keyword_equals(tokens, pos) {
        return false;
    }
    let mut index = pos;
    while index > 0 {
        index -= 1;
        match &tokens[index] {
            TokenTree::Punct(p) if p.as_char() == ',' || p.as_char() == '(' => break,
            TokenTree::Punct(p) if p.as_char() == ':' => return false,
            _ => {}
        }
    }
    true
}

/// Accumulated token-local state threaded through the format-string builder.
pub(super) struct SpacingState {
    pub prev: PrevTokenKind,
    pub colon_ctx: ColonContext,
    /// End position (line, column) of the last specifier's closing group,
    /// used to detect adjacent specifiers like `$L("a")$L("b")`.
    pub prev_specifier_end: Option<(usize, usize)>,
}

impl SpacingState {
    pub fn new(lang: MacroLang) -> Self {
        Self {
            prev: PrevTokenKind::None,
            colon_ctx: if lang.default_colon_is_space_before() {
                ColonContext::SpaceBefore
            } else {
                ColonContext::TypeAnnotation
            },
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
        | TokenAnnotation::PostfixAmpersand
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
                    ColonContext::Ternary
                    | ColonContext::WalrusAssign
                    | ColonContext::ForRange
                    | ColonContext::SpaceBefore => {}
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
        && matches!(prev, PrevTokenKind::Specifier | PrevTokenKind::ParsedSplice)
    {
        return;
    }

    // Default: add a space between tokens.
    format.push(' ');
}

#[cfg(test)]
mod tests {
    use proc_macro2::{Delimiter, Group, TokenStream, TokenTree};
    use std::str::FromStr;

    use super::*;

    fn tokens(source: &str) -> Vec<TokenTree> {
        TokenStream::from_str(source)
            .expect("test token stream parses")
            .into_iter()
            .collect()
    }

    fn group_role(source: &str, position: usize, annotation: TokenAnnotation) -> PythonGroupRole {
        let tokens = tokens(source);
        let TokenTree::Group(group) = &tokens[position] else {
            panic!("expected group at token position {position}");
        };
        let inner = group.stream().into_iter().collect::<Vec<_>>();
        let policy = MacroSpacingPolicy::for_language(MacroLang::Python)
            .enter_group(&tokens, position, group, annotation, &inner);
        let MacroSpacingPolicy::Python(context) = policy else {
            panic!("expected Python policy");
        };
        context.role
    }

    #[test]
    fn generic_policy_has_no_language_layout_state() {
        let generic = MacroSpacingPolicy::Generic;
        let group = Group::new(Delimiter::Parenthesis, TokenStream::new());
        assert!(!generic.break_after_comma());
        assert!(
            generic
                .with_layout()
                .layout_spec(&group, &[], ")")
                .is_none()
        );
    }

    #[test]
    fn python_layout_is_policy_owned_and_role_preserving() {
        let python = MacroSpacingPolicy::for_language(MacroLang::Python);
        assert!(!python.break_after_comma());
        assert!(python.with_layout().break_after_comma());
    }

    #[test]
    fn python_group_roles_cover_declarations_and_collections() {
        assert_eq!(
            group_role("def build(value)", 2, TokenAnnotation::Normal),
            PythonGroupRole::FunctionParameters
        );
        assert_eq!(
            group_role("Foo[Value]", 1, TokenAnnotation::CallOpen),
            PythonGroupRole::TypeArguments
        );
        assert_eq!(
            group_role("items[0]", 1, TokenAnnotation::CallOpen),
            PythonGroupRole::Subscript
        );
        assert_eq!(
            group_role("[value]", 0, TokenAnnotation::Normal),
            PythonGroupRole::List
        );
        assert_eq!(
            group_role("(first, second)", 0, TokenAnnotation::Normal),
            PythonGroupRole::Tuple
        );
        assert_eq!(
            group_role("{key: value}", 0, TokenAnnotation::Normal),
            PythonGroupRole::Dict
        );
        assert_eq!(
            group_role("{value}", 0, TokenAnnotation::Normal),
            PythonGroupRole::Set
        );
        assert_eq!(
            group_role("[value for value in values]", 0, TokenAnnotation::Normal),
            PythonGroupRole::Comprehension
        );
        assert_eq!(
            group_role("@decorator(value)", 2, TokenAnnotation::CallOpen),
            PythonGroupRole::Decorator
        );
        assert_eq!(
            group_role("if (value)", 1, TokenAnnotation::Normal),
            PythonGroupRole::ControlHeader
        );
        assert_eq!(
            group_role("(value)", 0, TokenAnnotation::Normal),
            PythonGroupRole::Parenthesized
        );
    }

    #[test]
    fn python_group_roles_ignore_macro_for_markers() {
        assert_eq!(
            group_role("call($for(value))", 1, TokenAnnotation::CallOpen),
            PythonGroupRole::CallArguments
        );
    }

    #[test]
    fn python_policy_punctuation_helpers_cover_context_boundaries() {
        let python = MacroSpacingPolicy::for_language(MacroLang::Python);
        let ident = tokens("identifier");
        assert_eq!(
            python.annotate_punctuation(&ident, 0, TokenAnnotation::Normal),
            TokenAnnotation::Normal
        );
        let from = tokens("from .");
        assert_eq!(
            python.annotate_punctuation(&from, 1, TokenAnnotation::Normal),
            TokenAnnotation::DotArg
        );

        let call_tokens = tokens("call(value)");
        let TokenTree::Group(call_group) = &call_tokens[1] else {
            panic!("expected call group");
        };
        let call_inner = tokens("value");
        let call_policy = python.enter_group(
            &call_tokens,
            1,
            call_group,
            TokenAnnotation::CallOpen,
            &call_inner,
        );
        let equals = tokens("value=1");
        assert_eq!(
            call_policy.annotate_punctuation(&equals, 1, TokenAnnotation::Normal),
            TokenAnnotation::AssignAdjacent
        );

        assert!(!python_keyword_equals(&tokens("x += 1"), 2));
        assert!(!python_keyword_equals(&tokens("x == 1"), 1));
        assert!(!python_keyword_equals(&tokens("= 1"), 0));
        assert!(!python_default_equals(&tokens("x: int = 1"), 3));

        let group_before = tokens("(x)(y)");
        assert!(python_type_like_before(&group_before, 1));
        let punctuation_before = tokens(">(x)");
        assert!(python_type_like_before(&punctuation_before, 1));
    }
}
