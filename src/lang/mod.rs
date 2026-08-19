/// Bash shell language support.
pub mod bash;
/// C language support.
pub mod c;
/// Shared configuration types (quote style, optional-field rendering).
pub mod config;
/// C++ language support.
pub mod cpp;
/// C# language support.
pub mod csharp;
/// Dart language support.
pub mod dart;
/// Go language support.
pub mod go;
/// Haskell language support.
pub mod haskell;
/// Java language support.
pub mod java;
/// JavaScript language support.
pub mod javascript;
/// Kotlin language support.
pub mod kotlin;
/// Lua language support.
pub mod lua;
/// OCaml language support.
pub mod ocaml;
/// PHP language support.
pub mod php;
/// Python language support.
pub mod python;
/// Ruby language support.
pub mod ruby;
/// Rust language support.
pub mod rust;
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

use crate::code_block::{Arg, CodeBlock};
use crate::code_node::BlockIntent;
use crate::error::SigilStitchError;
use crate::import::ImportGroup;
use crate::spec::modifiers::TypeKind;
use crate::spec::where_spec::{TypeParamSpec, render_type_params};
use crate::type_name::TypeName;

/// Narrow trait for `CodeRenderer` and `TypeName` rendering.
///
/// Implementors must provide:
/// - [`file_extension`](Self::file_extension)
/// - [`line_comment_prefix`](Self::line_comment_prefix)
/// - [`reserved_words`](Self::reserved_words) (default `&[]` — useless for real languages)
/// - [`type_presentation`](Self::type_presentation) (rules for compound type rendering)
/// - [`block_syntax`](Self::block_syntax) (delimiters, indentation, statement terminator)
///
/// The remaining methods have defaults suitable for most brace/double-quote
/// languages. Override only when your language differs.
pub trait RendererLang: std::fmt::Debug + 'static {
    /// File extension for this language (e.g., "ts", "go", "rs").
    fn file_extension(&self) -> &str;

    /// Render a string literal with language-appropriate quoting and escaping.
    ///
    /// Default: double-quote with C-style escaping (`\\`, `\"`, `\n`, `\t`,
    /// `\r`, `\0`). Override for languages with different quoting
    /// conventions (shell, JS/TS, Python, Kotlin, Dart, Go, Haskell,
    /// Rust, OCaml, Lua).
    fn render_string_literal(&self, s: &str) -> String {
        format!(
            "\"{}\"",
            s.replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', "\\n")
                .replace('\t', "\\t")
                .replace('\r', "\\r")
                .replace('\0', "\\0")
        )
    }

    /// Render a verbatim string literal with minimal escaping.
    ///
    /// Only escapes characters that would structurally break the string delimiter,
    /// preserving interpolation sigils (`$`, `` ` ``, `{`, etc.) as-is.
    /// For languages without string interpolation, falls back to full escaping.
    ///
    /// Default: delegates to [`render_string_literal`](Self::render_string_literal).
    fn render_verbatim_string(&self, s: &str) -> String {
        self.render_string_literal(s)
    }

    /// Single-line comment prefix (e.g., "//", "#").
    fn line_comment_prefix(&self) -> &str;

    /// Suffix appended after a single-line comment.
    ///
    /// Default: `""` (no suffix — most languages use line comments like `//`).
    /// OCaml overrides to `" *)"` to close `(* comment *)` block comments.
    fn line_comment_suffix(&self) -> &str {
        ""
    }

    /// Render an attribute / annotation with language-specific syntax.
    ///
    /// Default: `"@{text}"` (Java/Python/Kotlin/TypeScript decorator style).
    /// Override for Rust (`#[text]`), C++ (`[[text]]`), C# (`[text]`), etc.
    fn render_attribute(&self, text: &str) -> String {
        format!("@{text}")
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
    /// Legacy path used by old serialized/external string-only block nodes.
    /// New nodes use [`RendererLang::block_open_for_intent`].
    #[deprecated(note = "use block_open_for_intent")]
    fn block_open_for(&self, _condition: &str) -> Option<&str> {
        None
    }

    /// Map a control-flow condition to its block-closing delimiter.
    ///
    /// Legacy path used by old serialized/external string-only block nodes.
    /// New nodes use [`RendererLang::block_close_for_intent`].
    #[deprecated(note = "use block_close_for_intent")]
    fn block_close_for(&self, _condition: &str) -> Option<&str> {
        None
    }

    /// Map a control-flow block intent and condition to its block-opening
    /// delimiter.
    ///
    /// Called at render time for
    /// [`crate::code_node::CodeNode::BlockOpenIntent`] nodes. Return
    /// `Some("...")` to override the default `block_syntax().block_open`.
    ///
    /// The default delegates to the legacy [`RendererLang::block_open_for`]
    /// so existing external adapters keep working for both node forms.
    #[allow(deprecated)]
    fn block_open_for_intent(&self, _intent: BlockIntent, condition: &str) -> Option<&str> {
        self.block_open_for(condition)
    }

    /// Map a control-flow block intent and condition to its block-closing
    /// delimiter.
    ///
    /// Called at render time for
    /// [`crate::code_node::CodeNode::BlockCloseIntent`] and
    /// [`crate::code_node::CodeNode::BranchCloseIntent`] nodes. Return
    /// `Some("...")` to override the default `block_syntax().block_close`.
    ///
    /// The default delegates to the legacy [`RendererLang::block_close_for`]
    /// so existing external adapters keep working for both node forms.
    #[allow(deprecated)]
    fn block_close_for_intent(&self, _intent: BlockIntent, condition: &str) -> Option<&str> {
        self.block_close_for(condition)
    }

    /// Rewrite the node tree before rendering. Called automatically by the
    /// renderer. Default is no-op.
    fn rewrite_nodes(&self, _nodes: &mut Vec<crate::code_node::CodeNode>) {}

    /// Qualify an import name for rendering in code.
    ///
    /// Default: return the resolved name as-is.
    /// Go overrides this to prefix the package name (e.g., `"http.Server"`).
    /// Haskell uses the original name to render aliases as qualified references.
    fn qualify_import_name(&self, _module: &str, _name: &str, resolved_name: &str) -> String {
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
///
/// # Implementing structured type hooks
///
/// ```
/// use sigil_stitch::code_block::{Arg, CodeBlock};
/// use sigil_stitch::error::SigilStitchError;
/// use sigil_stitch::lang::{CodeLang, RendererLang};
/// use sigil_stitch::spec::file_spec::FileSpec;
/// use sigil_stitch::spec::modifiers::TypeKind;
/// use sigil_stitch::spec::type_spec::TypeSpec;
/// use sigil_stitch::spec::where_spec::{TypeParamSpec, render_type_params};
/// use sigil_stitch::type_name::TypeName;
///
/// #[derive(Debug)]
/// struct ExampleLang;
///
/// impl RendererLang for ExampleLang {
///     fn file_extension(&self) -> &str { "example" }
///     fn line_comment_prefix(&self) -> &str { "//" }
/// }
///
/// impl CodeLang for ExampleLang {
///     fn emit_newtype_decl(
///         &self,
///         visibility: &str,
///         name: &str,
///         type_params: &[TypeParamSpec],
///         inner: &TypeName,
///     ) -> Result<CodeBlock, SigilStitchError> {
///         let mut args = Vec::new();
///         let params = render_type_params(type_params, self, &mut args);
///         args.push(Arg::TypeName(inner.clone()));
///         CodeBlock::of(&format!("{visibility}type {name}{params} = %T"), args)
///     }
///
///     fn emit_type_context(
///         &self,
///         _type_params: &[TypeParamSpec],
///     ) -> Result<Option<CodeBlock>, SigilStitchError> {
///         Ok(None)
///     }
///
///     fn emit_type_close_suffix(
///         &self,
///         _kind: TypeKind,
///         _impl_types: &[TypeName],
///     ) -> Result<Option<CodeBlock>, SigilStitchError> {
///         Ok(None)
///     }
/// }
///
/// let wrapped = TypeSpec::builder("Wrapped", TypeKind::Newtype)
///     .extends(TypeName::primitive("String"))
///     .build()?;
/// let output = FileSpec::builder_with("wrapped.example", ExampleLang)
///     .add_type(wrapped)
///     .build()?
///     .render(80)?;
/// assert!(output.contains("type Wrapped = String"));
/// # Ok::<(), SigilStitchError>(())
/// ```
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

    /// Prefix applied to variable names (parameters, fields, properties,
    /// receiver names). Returns `""` by default. PHP returns `"$"`.
    fn variable_prefix(&self) -> &str {
        ""
    }

    /// Optional kind suffix after the type name (e.g., Go's `type Foo struct`).
    ///
    /// Default: empty (TS/Rust put the kind keyword before the name).
    fn type_kind_suffix(&self, _kind: crate::spec::modifiers::TypeKind) -> &str {
        ""
    }

    /// Emit a newtype declaration while preserving semantic type references.
    ///
    /// Default: Rust tuple-struct `{visibility}struct {name}<T>({inner});`.
    fn emit_newtype_decl(
        &self,
        visibility: &str,
        name: &str,
        type_params: &[TypeParamSpec],
        inner: &TypeName,
    ) -> Result<CodeBlock, SigilStitchError> {
        let mut args = Vec::new();
        let type_params = render_type_params(type_params, self, &mut args);
        args.push(Arg::TypeName(inner.clone()));
        CodeBlock::of(
            &format!("{visibility}struct {name}{type_params}(%T);"),
            args,
        )
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

    /// Emit a type context / constraint prefix for split function signatures.
    ///
    /// Default: no context.
    fn emit_type_context(
        &self,
        _type_params: &[TypeParamSpec],
    ) -> Result<Option<CodeBlock>, SigilStitchError> {
        Ok(None)
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

    /// Emit a suffix after the type's closing delimiter (e.g., Haskell `deriving`).
    ///
    /// Default: no suffix.
    fn emit_type_close_suffix(
        &self,
        _kind: TypeKind,
        _impl_types: &[TypeName],
    ) -> Result<Option<CodeBlock>, SigilStitchError> {
        Ok(None)
    }

    /// Render a type parameter's kind annotation (for higher-kinded types).
    ///
    /// Default: empty string.
    fn render_type_param_kind(&self, _kind: &crate::spec::where_spec::TypeParamKind) -> String {
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
        "rs" => Some(Box::new(rust::Rust::default())),
        "go" => Some(Box::new(go::Go::default())),
        "py" | "pyi" => Some(Box::new(python::Python::default())),
        "java" => Some(Box::new(java::Java::default())),
        "kt" | "kts" => Some(Box::new(kotlin::Kotlin::default())),
        "swift" => Some(Box::new(swift::Swift::default())),
        "dart" => Some(Box::new(dart::Dart::default())),
        "scala" | "sc" => Some(Box::new(scala::Scala::default())),
        "hs" => Some(Box::new(haskell::Haskell::default())),
        "ml" | "mli" => Some(Box::new(ocaml::OCaml::default())),
        "c" | "h" => Some(Box::new(c::C::default())),
        "cpp" | "cxx" | "cc" | "hpp" | "hxx" => Some(Box::new(cpp::Cpp::default())),
        "cs" => Some(Box::new(csharp::CSharp::default())),
        "lua" => Some(Box::new(lua::Lua::default())),
        "sh" | "bash" => Some(Box::new(bash::Bash::default())),
        "zsh" => Some(Box::new(zsh::Zsh::default())),
        "php" => Some(Box::new(php::Php::default())),
        "rb" => Some(Box::new(ruby::Ruby::default())),
        _ => None,
    }
}
