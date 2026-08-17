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

pub(crate) struct Branch {
    pub(crate) condition: FormattedCode,
    pub(crate) body: Vec<Statement>,
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
