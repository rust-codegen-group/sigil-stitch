use proc_macro2::{Span, TokenStream};

/// Language identity for macro-level tokenizer decisions.
///
/// Allows `annotate_tokens` to apply language-specific spacing heuristics
/// (e.g., shell `.` as argument vs. member access).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum MacroLang {
    Unaware,
    Bash,
    C,
    Cpp,
    CSharp,
    Dart,
    Go,
    Haskell,
    Kotlin,
    OCaml,
    Php,
    Ruby,
    Swift,
    TypeScript,
    Zsh,
}

impl MacroLang {
    pub fn is_shell(self) -> bool {
        matches!(self, Self::Bash | Self::Zsh)
    }

    /// Whether `:` defaults to space-before in type position.
    /// C-like languages use `name: Type` (no space before `:`);
    /// OCaml and Shell conventionally use `(x : int)` / `echo :` with space.
    pub fn default_colon_is_space_before(self) -> bool {
        matches!(self, Self::OCaml | Self::Bash | Self::Zsh)
    }

    /// Whether the language uses `<>` angle brackets for generics,
    /// so that `$T(Type)<params>` should mark `<` as GenericOpen.
    /// True for `Unaware` because most unrecognized languages (C#, Java,
    /// TypeScript, C++, etc.) use C-style angle-bracket generics.
    pub fn has_angle_generics(self) -> bool {
        !matches!(
            self,
            Self::Ruby
                | Self::Bash
                | Self::C
                | Self::Zsh
                | Self::OCaml
                | Self::Php
                | Self::Go
                | Self::Haskell
        )
    }

    /// Whether `?Ident` (nullable prefix like `?User`, `?string`) is valid syntax.
    pub fn nullable_prefix_is_valid(self) -> bool {
        matches!(self, Self::Php | Self::OCaml)
    }

    /// Whether `*` adjacent to a preceding ident means a postfix pointer type
    /// (C/C++ `Config*`, C# `int*` in unsafe context).
    pub fn has_postfix_star(self) -> bool {
        matches!(self, Self::C | Self::Cpp | Self::CSharp)
    }

    /// Whether `&` adjacent to a preceding ident means a postfix reference type
    /// (C++ `auto&`, `int&`).
    pub fn has_postfix_ampersand(self) -> bool {
        matches!(self, Self::Cpp)
    }

    /// Whether `?` adjacent to a preceding ident means a postfix nullable type
    /// (C# `int?`, TS `string?`, Swift `Int?`, Kotlin `Int?`, Dart `int?`).
    pub fn has_postfix_question_type(self) -> bool {
        matches!(
            self,
            Self::CSharp | Self::Dart | Self::Kotlin | Self::Swift | Self::TypeScript
        )
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
    /// `add_comment(text)` — `$comment("text")` or `$comment(expr)`.
    Comment(TokenStream),
    /// `add_attribute(text)` — `$attr(expr)`. Language-aware attribute/annotation
    /// that renders with the target language's prefix/suffix (Rust: #[...],
    /// Java/Python: @..., C++: [[...]]).
    Attr(TokenStream),
    /// Control flow: `begin_control_flow` / `next_control_flow` / `end_control_flow`.
    /// Only for actual control flow (`if`/`for`/`while`/`class`/`function`).
    /// Expression braces are routed through `Statement::Statement` + `ParsedBlock`.
    ControlFlow {
        branches: Vec<Branch>,
        /// Emit a `;` after the closing brace (for expression-level control
        /// flow like `match` in PHP/Rust).
        trailing_semicolon: bool,
    },
    /// `add("%>", ())` — increase indent.
    Indent,
    /// `add("%<", ())` — decrease indent.
    Dedent,
    /// `$C_each(expr)` — splice each code block from an iterable sequentially.
    SpliceEach { expr: TokenStream },
    /// `$if(cond) { ... } $else_if(cond) { ... } $else { ... }` — meta-conditional.
    MetaIf { branches: Vec<MetaBranch> },
    /// `$for(pat in expr[; separator = expr[, trailing = bool]]) { ... }` —
    /// meta-loop that emits body once per iteration.
    MetaFor {
        pat: TokenStream,
        iter_expr: TokenStream,
        separator: Option<TokenStream>,
        trailing: Option<TokenStream>,
        body: Vec<Statement>,
    },
    /// Inline `$for(...) { ... }` splice that emits fragment bodies without
    /// statement newlines.
    InlineFor {
        pat: TokenStream,
        iter_expr: TokenStream,
        separator: Option<TokenStream>,
        trailing: Option<TokenStream>,
        body_format: String,
        body_args: Vec<TypedArg>,
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
    /// Pre-parsed body for `InterpolationKind::ParsedBlock`.
    pub parsed_body: Option<Vec<Statement>>,
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
    /// `$comment(expr)` — inline comment (when used inside a statement).
    Comment,
    /// Parsed brace group containing statement markers
    /// (`$C_each`, `$for`, `$if`, `$let`). Emitted as a nested
    /// `CodeBlock` via `%L`.
    ParsedBlock,
    /// Parsed inline splice (`$for`/`$if` inside expressions).
    /// Emitted as `%L` + nested CodeBlock WITHOUT synthetic block delimiters
    /// (no `begin_control_flow` / `end_control_flow_no_newline` wrapping).
    ParsedSplice,
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

    #[cfg(test)]
    pub(crate) fn message(&self) -> &str {
        &self.message
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
