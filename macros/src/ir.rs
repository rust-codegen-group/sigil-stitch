use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Expr, Local, Pat};

/// A parsed `sigil_quote!` invocation.
pub(crate) struct ParsedInput {
    pub(crate) statements: Vec<Statement>,
}

/// A target-language format string coupled to its Rust-side arguments.
///
/// The fields stay private so a format specifier cannot drift away from the
/// argument variant that supplies it.
pub(crate) struct FormattedCode {
    format: String,
    args: Vec<QuoteArg>,
}

impl FormattedCode {
    pub(crate) fn new() -> Self {
        Self {
            format: String::new(),
            args: Vec::new(),
        }
    }

    pub(crate) fn push_argument(&mut self, arg: QuoteArg) {
        self.format.push_str(arg.specifier());
        self.args.push(arg);
    }

    pub(crate) fn push_layout(&mut self, code: FormattedCode, spec: LayoutPlan) {
        self.push_argument(QuoteArg::Layout {
            code: Box::new(code),
            spec,
        });
    }

    pub(crate) fn append(&mut self, mut fragment: Self) {
        self.format.push_str(&fragment.format);
        self.args.append(&mut fragment.args);
    }

    pub(crate) fn push_marker(&mut self, marker: NoArgMarker) {
        self.format.push_str(marker.specifier());
    }

    pub(crate) fn format(&self) -> &str {
        &self.format
    }

    pub(crate) fn format_mut(&mut self) -> &mut String {
        &mut self.format
    }

    pub(crate) fn args(&self) -> &[QuoteArg] {
        &self.args
    }

    pub(crate) fn is_single_code_argument(&self) -> bool {
        self.format == "%L" && matches!(self.args.as_slice(), [QuoteArg::Code(_)])
    }

    #[cfg(test)]
    pub(crate) fn into_parts(self) -> (String, Vec<QuoteArg>) {
        (self.format, self.args)
    }
}

pub(crate) enum NoArgMarker {
    Indent,
    Dedent,
    SoftBreak,
}

impl NoArgMarker {
    fn specifier(&self) -> &'static str {
        match self {
            Self::Indent => "%>",
            Self::Dedent => "%<",
            Self::SoftBreak => "%W",
        }
    }
}

/// A dynamic string expression or the decoded contents of a direct literal.
pub(crate) enum StringValue {
    Dynamic(Expr),
    Literal(String),
    Interpolated {
        format_string: String,
        expressions: Vec<Expr>,
        // Source-literal location with call-site name resolution.
        span: Span,
    },
}

/// Runtime layout metadata for one macro-owned delimiter group.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TrailingCommaPolicy {
    Never,
    IfBroken,
    Always,
}

impl TrailingCommaPolicy {
    pub(crate) fn runtime_value(self) -> proc_macro2::TokenStream {
        let value = match self {
            Self::Never => "Never",
            Self::IfBroken => "IfBroken",
            Self::Always => "Always",
        };
        let ident = proc_macro2::Ident::new(value, proc_macro2::Span::call_site());
        quote!(::sigil_stitch::code_node::TrailingCommaPolicy::#ident)
    }
}

/// Mutually exclusive layout plans selected by the target-owned parser.
pub(crate) enum LayoutPlan {
    /// The fragment continues at the containing delimiter's indentation.
    Inline,
    /// A complete delimiter group with an explicit edge policy.
    Delimited(DelimitedLayout),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LayoutEdgesPlan {
    /// Delimiter edges wrap a separately laid-out item group.
    WithBreaks,
    /// Delimiter edges and items break together.
    Coupled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LayoutForceBreak {
    Automatic,
    Forced,
}

/// Complete grammar-neutral recipe for one delimiter layout.
pub(crate) struct DelimitedLayout {
    pub(crate) edges: LayoutEdgesPlan,
    pub(crate) trailing_comma: TrailingCommaPolicy,
    pub(crate) force_break: LayoutForceBreak,
    pub(crate) delimited_loop: Option<DelimitedLoopLayout>,
    pub(crate) flat_prefix: String,
    pub(crate) flat_suffix: String,
    pub(crate) broken_prefix: String,
    pub(crate) broken_suffix: String,
}

/// A typed argument accepted by a target-language format string.
pub(crate) enum QuoteArg {
    Type(Expr),
    Name(Expr),
    StringLit(Expr),
    VerbatimStr(StringValue),
    Literal(StringValue),
    Code(Expr),
    Join {
        separator: Expr,
        iter: Expr,
    },
    TypeJoin {
        separator: Expr,
        iter: Expr,
    },
    Comment(StringValue),
    ParsedBlock(Vec<Statement>),
    ParsedSplice(Vec<Statement>),
    Layout {
        code: Box<FormattedCode>,
        spec: LayoutPlan,
    },
}

impl QuoteArg {
    fn specifier(&self) -> &'static str {
        match self {
            Self::Type(_) => "%T",
            Self::Name(_) => "%N",
            Self::StringLit(_) => "%S",
            Self::VerbatimStr(_) => "%V",
            Self::Comment(_) => "%R",
            Self::Literal(_)
            | Self::Code(_)
            | Self::Join { .. }
            | Self::TypeJoin { .. }
            | Self::ParsedBlock(_)
            | Self::ParsedSplice(_)
            | Self::Layout { .. } => "%L",
        }
    }
}

/// A single statement or directive in the macro body.
pub(crate) enum Statement {
    Terminated(FormattedCode),
    Line(FormattedCode),
    BlankLine,
    Comment(StringValue),
    Attr(StringValue),
    ControlFlow {
        branches: Vec<Branch>,
        trailing_semicolon: bool,
    },
    Indent,
    Dedent,
    SpliceEach {
        expr: Expr,
    },
    MetaIf(MetaIf),
    MetaFor {
        pat: Pat,
        iter_expr: Expr,
        separator: Option<LoopSeparator>,
        body: Vec<Statement>,
    },
    InlineFor {
        pat: Pat,
        iter_expr: Expr,
        separator: Option<LoopSeparator>,
        body: FormattedCode,
    },
    MetaLet {
        local: Local,
        marker_span: proc_macro2::Span,
    },
    ParenBlock {
        header: FormattedCode,
        body: Vec<Statement>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SingletonCommaPolicy {
    KeepInline,
    ForceBroken,
}

/// Delimiter mechanics for a complete loop of structured comma-separated items.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DelimitedLoopLayout {
    pub(crate) edges: LayoutEdgesPlan,
    pub(crate) singleton_comma: SingletonCommaPolicy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SeparatorIntent {
    Verbatim,
    CommaSpace,
}

pub(crate) struct LoopSeparator {
    pub(crate) expr: Expr,
    pub(crate) trailing: Option<Expr>,
    pub(crate) intent: SeparatorIntent,
}

/// Structural block role carried from parse time to builder lowering.
///
/// This intentionally mirrors `sigil_stitch::code_node::BlockIntent`. The
/// proc-macro crate cannot import the runtime crate, so equivalence tests keep
/// the two enums in sync.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BranchIntent {
    Generic,
    If,
    ElseIf,
    Else,
    For,
    While,
    Until,
    Case,
    Match,
    Try,
    Class,
    Instance,
    Module,
    ModuleType,
    Do,
    Function,
    Lambda,
}

impl BranchIntent {
    pub(crate) fn runtime_path(self) -> TokenStream {
        let variant = match self {
            Self::Generic => "Generic",
            Self::If => "If",
            Self::ElseIf => "ElseIf",
            Self::Else => "Else",
            Self::For => "For",
            Self::While => "While",
            Self::Until => "Until",
            Self::Case => "Case",
            Self::Match => "Match",
            Self::Try => "Try",
            Self::Class => "Class",
            Self::Instance => "Instance",
            Self::Module => "Module",
            Self::ModuleType => "ModuleType",
            Self::Do => "Do",
            Self::Function => "Function",
            Self::Lambda => "Lambda",
        };
        let ident = proc_macro2::Ident::new(variant, proc_macro2::Span::call_site());
        quote!(::sigil_stitch::code_node::BlockIntent::#ident)
    }
}

pub(crate) struct Branch {
    pub(crate) condition: FormattedCode,
    pub(crate) body: Vec<Statement>,
    pub(crate) intent: BranchIntent,
}

pub(crate) struct MetaIf {
    pub(crate) first: ConditionalBranch,
    pub(crate) else_if: Vec<ConditionalBranch>,
    pub(crate) otherwise: Option<Vec<Statement>>,
}

pub(crate) struct ConditionalBranch {
    pub(crate) condition: Expr,
    pub(crate) body: Vec<Statement>,
}
