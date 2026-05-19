/// Bash shell language support.
pub mod bash;
/// C language support.
pub mod c_lang;
/// Shared configuration types (quote style, optional-field rendering).
pub mod config;
/// C++ language support.
pub mod cpp_lang;
/// C# language support.
pub mod csharp;
/// Dart language support.
pub mod dart;
/// Go language support.
pub mod go_lang;
/// Haskell language support.
pub mod haskell;
/// Java language support.
pub mod java_lang;
/// JavaScript language support.
pub mod javascript;
/// Kotlin language support.
pub mod kotlin;
/// Lua language support.
pub mod lua;
/// OCaml language support.
pub mod ocaml;
/// Python language support.
pub mod python;
/// Rust language support.
pub mod rust_lang;
/// Scala language support.
pub mod scala;
/// Swift language support.
pub mod swift;
/// TypeScript language support.
pub mod typescript;
/// Zsh shell language support.
pub mod zsh;

/// Helpers for implementing language-specific node rewrite passes.
pub mod rewrite;

use crate::import::ImportGroup;

/// Narrow trait containing only the methods needed by `CodeRenderer` and
/// `TypeName` rendering. Implement this for basic `CodeBlock` rendering
/// without needing the full spec-layer surface.
///
/// All 14 methods here are used directly by `code_renderer.rs` or by
/// `TypeName::to_doc_with_lang` (which is called during rendering).
pub trait RendererLang: std::fmt::Debug + 'static {
    /// File extension for this language (e.g., "ts", "go", "rs").
    fn file_extension(&self) -> &str;

    /// Render a string literal with language-appropriate quoting and escaping.
    fn render_string_literal(&self, s: &str) -> String;

    /// Single-line comment prefix (e.g., "//", "#").
    fn line_comment_prefix(&self) -> &str;

    /// Suffix appended after a single-line comment.
    ///
    /// Default: `""` (no suffix — most languages use line comments like `//`).
    /// OCaml overrides to `" *)"` to close `(* comment *)` block comments.
    fn line_comment_suffix(&self) -> &str {
        ""
    }

    /// Reserved words that need escaping.
    fn reserved_words(&self) -> &[&str] {
        &[]
    }

    /// Escape a name if it collides with a reserved word.
    /// Default: append underscore.
    fn escape_reserved(&self, name: &str) -> String {
        if self.reserved_words().contains(&name) {
            format!("{name}_")
        } else {
            name.to_string()
        }
    }

    /// Block delimiters, indentation, and statement termination.
    fn block_syntax(&self) -> config::BlockSyntaxConfig<'_> {
        config::BlockSyntaxConfig::default()
    }

    /// Map a control-flow condition to its block-opening delimiter.
    ///
    /// Called at render time with the condition text from `begin_control_flow`.
    /// Return `Some("...")` to override the default `block_syntax().block_open`.
    fn block_open_for(&self, _condition: &str) -> Option<&str> {
        None
    }

    /// Map a control-flow condition to its block-closing delimiter.
    ///
    /// Called at render time with the condition text from `begin_control_flow`.
    /// Return `Some("...")` to override the default `block_syntax().block_close`.
    fn block_close_for(&self, _condition: &str) -> Option<&str> {
        None
    }

    /// Rewrite the node tree before rendering. Called automatically by the
    /// renderer. Default is no-op.
    fn rewrite_nodes(&self, _nodes: &mut Vec<crate::code_node::CodeNode>) {}

    /// Qualify an import name for rendering in code.
    ///
    /// Default: return the resolved name as-is.
    /// Go overrides this to prefix the package name (e.g., `"http.Server"`).
    fn qualify_import_name(&self, _module: &str, resolved_name: &str) -> String {
        resolved_name.to_string()
    }

    /// The separator between module path and type name for qualified inline
    /// references (e.g., `"::"` for Rust/C++, `"."` for Go/Python/Java).
    fn module_separator(&self) -> Option<&str> {
        None
    }

    /// How each compound `TypeName` variant renders.
    fn type_presentation(&self) -> config::TypePresentationConfig<'_> {
        config::TypePresentationConfig::default()
    }

    /// Generic type parameter delimiters and constraints.
    fn generic_syntax(&self) -> config::GenericSyntaxConfig<'_> {
        config::GenericSyntaxConfig::default()
    }
}

/// Full language trait for spec-level code generation.
///
/// Extends [`RendererLang`] with the additional methods needed by the spec
/// layer (`FunSpec`, `TypeSpec`, `FieldSpec`, etc.) and `FileSpec` (imports).
///
/// Implement this when you need full `FileSpec`-level generation including
/// functions, types, fields, and imports. For basic `CodeBlock` rendering,
/// only [`RendererLang`] is required.
pub trait CodeLang: RendererLang {
    // ── Spec-layer methods — used by FunSpec, TypeSpec, FieldSpec, etc. ───

    /// Render an import group to a string.
    ///
    /// Default: `""` (no import system).
    fn render_imports(&self, _imports: &ImportGroup) -> String {
        String::new()
    }

    /// Render a doc comment block.
    ///
    /// Default: wraps each line with `line_comment_prefix()` and
    /// `line_comment_suffix()`.
    fn render_doc_comment(&self, lines: &[&str]) -> String {
        let prefix = self.line_comment_prefix();
        let suffix = self.line_comment_suffix();
        lines
            .iter()
            .map(|line| {
                if line.is_empty() {
                    format!("{prefix}{suffix}")
                } else {
                    format!("{prefix} {line}{suffix}")
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Render a visibility modifier for the given declaration context.
    ///
    /// Default: `""` (no visibility modifiers).
    fn render_visibility(
        &self,
        _vis: crate::spec::modifiers::Visibility,
        _ctx: crate::spec::modifiers::DeclarationContext,
    ) -> &str {
        ""
    }

    /// The keyword used to declare a function (e.g., "fn", "function").
    ///
    /// Default: `""`.
    fn function_keyword(&self, _ctx: crate::spec::modifiers::DeclarationContext) -> &str {
        ""
    }

    /// The keyword for a type declaration (e.g., "struct", "class").
    ///
    /// Default: `""`.
    fn type_keyword(&self, _kind: crate::spec::modifiers::TypeKind) -> &str {
        ""
    }

    /// Whether methods are declared inside the type body (true for TS class, Rust trait)
    /// vs in a separate impl block (Rust struct/enum).
    ///
    /// Default: `true`.
    fn methods_inside_type_body(&self, _kind: crate::spec::modifiers::TypeKind) -> bool {
        true
    }

    /// Escape a field/property name. Languages where property names never
    /// conflict with reserved words (e.g. TypeScript) can return the name as-is.
    fn escape_field_name(&self, name: &str) -> String {
        self.escape_reserved(name)
    }

    /// Optional kind suffix after the type name (e.g., Go's `type Foo struct`).
    ///
    /// Default: empty (TS/Rust put the kind keyword before the name).
    fn type_kind_suffix(&self, _kind: crate::spec::modifiers::TypeKind) -> &str {
        ""
    }

    /// Render a newtype declaration line from pre-rendered components.
    ///
    /// Default: Rust tuple-struct `{vis}struct {name}({inner});`.
    fn render_newtype_line(&self, vis: &str, name: &str, inner: &str) -> String {
        format!("{vis}struct {name}({inner});")
    }

    /// Opening block delimiter for function bodies specifically.
    ///
    /// Default: `" {"`.
    fn fun_block_open(&self) -> &str {
        " {"
    }

    /// Opening block delimiter for type headers, parameterized by type kind.
    ///
    /// Default: `" {"`.
    fn type_header_block_open(&self, _kind: crate::spec::modifiers::TypeKind) -> &str {
        " {"
    }

    /// Whether doc comments should be rendered inside the body (after block open)
    /// rather than above the declaration.
    ///
    /// Default: `false`. Python overrides to `true` (docstrings go inside the body).
    fn doc_comment_inside_body(&self) -> bool {
        false
    }

    /// Whether doc comments should be emitted before annotations/attributes.
    ///
    /// Default: `true`.
    fn doc_before_annotations(&self) -> bool {
        true
    }

    /// How this language expresses that a field is optional (key may be absent).
    ///
    /// Default: `OptionalFieldStyle::Ignored`.
    fn optional_field_style(&self) -> crate::lang::config::OptionalFieldStyle {
        crate::lang::config::OptionalFieldStyle::Ignored
    }

    /// How `PropertySpec` renders: accessor methods or inline field body.
    ///
    /// Default: `Accessor`.
    fn property_style(&self) -> crate::spec::modifiers::PropertyStyle {
        crate::spec::modifiers::PropertyStyle::Accessor
    }

    /// The keyword for a property getter in field-style rendering.
    ///
    /// Default: `"get"`.
    fn property_getter_keyword(&self) -> &str {
        "get"
    }

    /// Render a type context / constraint prefix for split function signatures.
    ///
    /// Default: `""` (no context).
    fn render_type_context(&self, _type_params: &[crate::spec::fun_spec::TypeParamSpec]) -> String {
        String::new()
    }

    /// Content emitted after `block_open` but before the first field in a type body.
    ///
    /// Default: `""`.
    fn type_body_prefix(&self, _name: &str, _kind: crate::spec::modifiers::TypeKind) -> String {
        String::new()
    }

    /// Content emitted after the last field but before `block_close` in a type body.
    ///
    /// Default: `""`.
    fn type_body_suffix(&self, _name: &str, _kind: crate::spec::modifiers::TypeKind) -> String {
        String::new()
    }

    /// Suffix rendered after the type's closing delimiter (e.g., Haskell `deriving`).
    ///
    /// Default: `""`.
    fn render_type_close_suffix(
        &self,
        _kind: crate::spec::modifiers::TypeKind,
        _impl_types: &[String],
    ) -> String {
        String::new()
    }

    /// Render a type parameter's kind annotation (for higher-kinded types).
    ///
    /// Default: empty string.
    fn render_type_param_kind(&self, _kind: &crate::spec::fun_spec::TypeParamKind) -> String {
        String::new()
    }

    // ── Config struct accessors (spec-only) ───────────────────────────

    /// Function signature syntax.
    fn function_syntax(&self) -> config::FunctionSyntaxConfig<'_> {
        config::FunctionSyntaxConfig::default()
    }

    /// Type declaration syntax (inheritance, field order).
    fn type_decl_syntax(&self) -> config::TypeDeclSyntaxConfig<'_> {
        config::TypeDeclSyntaxConfig::default()
    }

    /// Enum variant formatting, annotation syntax, and field mutability keywords.
    fn enum_and_annotation(&self) -> config::EnumAndAnnotationConfig<'_> {
        config::EnumAndAnnotationConfig::default()
    }
}

/// Derive a PascalCase namespace alias from a module path.
///
/// Used for wildcard imports that need a namespace name
/// (e.g., `import * as Models from "./models"`).
pub(crate) fn module_to_alias(module: &str) -> String {
    let last_segment = module
        .rsplit(['/', ':', '.', '\\'])
        .find(|s| !s.is_empty())
        .unwrap_or(module);

    let mut chars = last_segment.chars();
    match chars.next() {
        None => "Module".to_string(),
        Some(first) => {
            let upper: String = first.to_uppercase().collect();
            format!("{upper}{}", chars.as_str())
        }
    }
}

/// Create a default `CodeLang` implementation from a file extension.
///
/// Returns `None` if the extension is not recognized.
pub fn lang_from_extension(ext: &str) -> Option<Box<dyn CodeLang>> {
    match ext {
        "ts" | "tsx" => Some(Box::new(typescript::TypeScript::default())),
        "js" | "jsx" | "mjs" | "cjs" => Some(Box::new(javascript::JavaScript::default())),
        "rs" => Some(Box::new(rust_lang::RustLang::default())),
        "go" => Some(Box::new(go_lang::GoLang::default())),
        "py" | "pyi" => Some(Box::new(python::Python::default())),
        "java" => Some(Box::new(java_lang::JavaLang::default())),
        "kt" | "kts" => Some(Box::new(kotlin::Kotlin::default())),
        "swift" => Some(Box::new(swift::Swift::default())),
        "dart" => Some(Box::new(dart::DartLang::default())),
        "scala" | "sc" => Some(Box::new(scala::Scala::default())),
        "hs" => Some(Box::new(haskell::Haskell::default())),
        "ml" | "mli" => Some(Box::new(ocaml::OCaml::default())),
        "c" | "h" => Some(Box::new(c_lang::CLang::default())),
        "cpp" | "cxx" | "cc" | "hpp" | "hxx" => Some(Box::new(cpp_lang::CppLang::default())),
        "cs" => Some(Box::new(csharp::CSharp::default())),
        "lua" => Some(Box::new(lua::Lua::default())),
        "sh" | "bash" => Some(Box::new(bash::Bash::default())),
        "zsh" => Some(Box::new(zsh::Zsh::default())),
        _ => None,
    }
}
