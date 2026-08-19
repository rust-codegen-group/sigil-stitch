use proc_macro2::TokenStream;
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
    },
}

/// A typed argument accepted by a target-language format string.
pub(crate) enum QuoteArg {
    Type(Expr),
    Name(Expr),
    StringLit(Expr),
    VerbatimStr(StringValue),
    Literal(StringValue),
    Code(Expr),
    Join { separator: Expr, iter: Expr },
    TypeJoin { separator: Expr, iter: Expr },
    Comment(StringValue),
    ParsedBlock(Vec<Statement>),
    ParsedSplice(Vec<Statement>),
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
            | Self::ParsedSplice(_) => "%L",
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

pub(crate) struct LoopSeparator {
    pub(crate) expr: Expr,
    pub(crate) trailing: Option<Expr>,
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
