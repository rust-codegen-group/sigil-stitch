use proc_macro2::{Span, TokenStream};

/// Language identity for macro-level tokenizer decisions.
///
/// Allows `annotate_tokens` to apply language-specific spacing heuristics
/// (e.g., shell `.` as argument vs. member access).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum MacroLang {
    Unaware,
    Bash,
    Zsh,
    GoLang,
    Haskell,
    OCaml,
}

impl MacroLang {
    pub fn is_shell(self) -> bool {
        matches!(self, Self::Bash | Self::Zsh)
    }
}

/// A parsed `sigil_quote!` invocation.
pub(crate) struct ParsedInput {
    /// The resolved language for macro-level tokenizer decisions.
    /// Used during parsing (threaded to annotate_tokens); not read by codegen.
    #[allow(dead_code)]
    pub lang: MacroLang,
    /// The language type tokens (e.g., `TypeScript`). Kept for codegen
    /// (the generated code references this type).
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
    /// `add_attribute(text)` — `$attr("text")`. Language-aware attribute/annotation
    /// that renders with the target language's prefix/suffix (Rust: #[...],
    /// Java/Python: @..., C++: [[...]]).
    Attr(String),
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
    /// `$for(pat in expr) { ... }` — meta-loop that emits body once per iteration.
    MetaFor {
        pat: TokenStream,
        iter_expr: TokenStream,
        body: Vec<Statement>,
    },
    /// `$let(binding);` — Rust-level `let` binding inside macro body.
    MetaLet { binding: TokenStream },
    /// A parenthesized declaration block (Go: `const (...)`, `var (...)`,
    /// `import (...)`, `type (...)`). The body is recursively parsed so
    /// `$for`, `$if`, etc. expand inside.
    ParenBlock {
        header_format: String,
        header_args: Vec<TypedArg>,
        body: Vec<Statement>,
    },
}

/// A single branch in a control flow chain.
pub(crate) struct Branch {
    /// Format string for the condition (e.g., `"if (x > 0)"`).
    pub condition_format: String,
    /// Interpolation args for the condition.
    pub condition_args: Vec<TypedArg>,
    /// Body statements inside the braces.
    pub body: Vec<Statement>,
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
    /// `$V(expr)` — verbatim string literal (minimal escaping).
    VerbatimStr,
    /// `$L(expr)` — literal value.
    Literal,
    /// `$C(expr)` — nested code block.
    Code,
    /// `$T_join(sep, iter)` — join TypeName items with a separator,
    /// tracking imports for each item via `%T` slots.
    TypeJoin,
}

/// A compile error with span information.
pub(crate) struct CompileError {
    span: Span,
    message: String,
}

impl std::fmt::Debug for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CompileError({:?})", self.message)
    }
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
