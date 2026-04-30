use proc_macro2::{Span, TokenStream};

/// A parsed `sigil_quote!` invocation.
pub(crate) struct ParsedInput {
    /// The language type tokens (e.g., `TypeScript`). Parsed for backwards
    /// compatibility but no longer used in code generation since types are
    /// no longer parameterized by language.
    #[allow(dead_code)]
    pub lang_type: TokenStream,
    /// The parsed body statements.
    pub statements: Vec<Statement>,
}

/// A single statement or directive in the macro body.
#[allow(clippy::enum_variant_names)]
pub(crate) enum Statement {
    /// `add_statement(format, args)` — line ending with `;`.
    Statement { format: String, args: Vec<TypedArg> },
    /// `add(format, args)` + newline — line without `;`.
    Line { format: String, args: Vec<TypedArg> },
    /// `add_line()` — blank line.
    BlankLine,
    /// `add_comment(text)` — `$comment("text")`.
    Comment(String),
    /// Control flow: `begin_control_flow` / `next_control_flow` / `end_control_flow`.
    ControlFlow { branches: Vec<Branch> },
    /// `add("%>", ())` — increase indent.
    Indent,
    /// `add("%<", ())` — decrease indent.
    Dedent,
    /// `$C_each(expr)` — splice each code block from an iterable sequentially.
    SpliceEach { expr: TokenStream },
    /// `$if(cond) { ... } $else_if(cond) { ... } $else { ... }` — meta-conditional.
    MetaIf { branches: Vec<MetaBranch> },
}

/// A single branch in a control flow chain.
pub(crate) struct Branch {
    /// Format string for the condition (e.g., `"if (x > 0)"`).
    pub condition_format: String,
    /// Interpolation args for the condition.
    pub condition_args: Vec<TypedArg>,
    /// Body statements inside the braces.
    pub body: Vec<Statement>,
    /// Custom block opener override from `$open("...")`, only on the first branch.
    pub block_open_override: Option<String>,
}

/// A branch in a `$if` / `$else_if` / `$else` meta-conditional.
pub(crate) struct MetaBranch {
    /// Condition expression (None for `$else`).
    pub condition: Option<TokenStream>,
    /// Body statements inside the braces.
    pub body: Vec<Statement>,
}

/// An interpolation argument with its kind for proper wrapping in codegen.
pub(crate) struct TypedArg {
    pub kind: InterpolationKind,
    pub expr: TokenStream,
}

/// The kind of an interpolation marker.
#[derive(Clone, Copy)]
pub(crate) enum InterpolationKind {
    /// `$T(expr)` — type reference.
    Type,
    /// `$N(expr)` — name identifier.
    Name,
    /// `$S(expr)` — string literal.
    StringLit,
    /// `$L(expr)` — literal value.
    Literal,
    /// `$C(expr)` — nested code block.
    Code,
}

/// A compile error with span information.
pub(crate) struct CompileError {
    span: Span,
    message: String,
}

impl CompileError {
    pub fn new(span: Span, message: impl Into<String>) -> Self {
        CompileError {
            span,
            message: message.into(),
        }
    }

    /// Convert to a `compile_error!()` token stream.
    pub fn into_compile_error(self) -> TokenStream {
        let msg = &self.message;
        let mut ts = TokenStream::new();
        let err = quote::quote_spanned! { self.span =>
            ::core::compile_error!(#msg)
        };
        ts.extend(err);
        ts
    }
}
