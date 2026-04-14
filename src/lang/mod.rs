pub mod c_lang;
pub mod cpp_lang;
pub mod go_lang;
pub mod javascript;
pub mod python;
pub mod rust_lang;
pub mod typescript;

use crate::import::ImportGroup;

/// Trait defining language-specific behavior for code generation.
///
/// All types in sigil-stitch are parameterized by `L: CodeLang`, allowing
/// the same CodeBlock and TypeName structures to generate output for any
/// supported language.
///
/// Methods that may vary per configuration (indent_unit, uses_semicolons,
/// render_string_literal) take `&self`. Methods that are truly invariant
/// per language (file_extension, reserved_words) also take `&self` for
/// consistency and future flexibility.
pub trait CodeLang: Sized + Clone + 'static {
    /// File extension for this language (e.g., "ts", "go", "rs").
    fn file_extension(&self) -> &str;

    /// Reserved words that need escaping.
    fn reserved_words(&self) -> &[&str];

    /// Render an import group to a string.
    /// `imports` is deduplicated and grouped by module.
    fn render_imports(&self, imports: &ImportGroup) -> String;

    /// Render a string literal with language-appropriate quoting and escaping.
    fn render_string_literal(&self, s: &str) -> String;

    /// Render a doc comment block.
    fn render_doc_comment(&self, lines: &[&str]) -> String;

    /// Single-line comment prefix (e.g., "//", "#").
    fn line_comment_prefix(&self) -> &str;

    /// Indentation unit (e.g., "  " for 2-space, "\t" for tabs).
    fn indent_unit(&self) -> &str;

    /// Whether this language uses semicolons to terminate statements.
    fn uses_semicolons(&self) -> bool;

    /// Escape a name if it collides with a reserved word.
    /// Default: append underscore.
    fn escape_reserved(&self, name: &str) -> String {
        if self.reserved_words().contains(&name) {
            format!("{name}_")
        } else {
            name.to_string()
        }
    }

    // --- Phase 2: structural spec support ---

    /// Render a visibility modifier for the given declaration context.
    fn render_visibility(
        &self,
        vis: crate::spec::modifiers::Visibility,
        ctx: crate::spec::modifiers::DeclarationContext,
    ) -> &str;

    /// The keyword used to declare a function (e.g., "fn", "function").
    fn function_keyword(&self, ctx: crate::spec::modifiers::DeclarationContext) -> &str;

    /// Separator between a function and its return type (e.g., " -> ", ": ").
    fn return_type_separator(&self) -> &str;

    /// The keyword for a type declaration (e.g., "struct", "class").
    fn type_keyword(&self, kind: crate::spec::modifiers::TypeKind) -> &str;

    /// Terminator after a field declaration (e.g., "," for Rust, ";" for TS).
    fn field_terminator(&self) -> &str;

    /// Whether methods are declared inside the type body (true for TS class, Rust trait)
    /// vs in a separate impl block (Rust struct/enum).
    fn methods_inside_type_body(&self, kind: crate::spec::modifiers::TypeKind) -> bool;

    /// Keyword introducing a generic constraint (e.g., ": " for Rust, " extends " for TS).
    fn generic_constraint_keyword(&self) -> &str;

    /// Separator between multiple generic bounds (e.g., " + " for Rust, " & " for TS).
    fn generic_constraint_separator(&self) -> &str;

    /// Keyword for super type / base class (e.g., "" for Rust, " extends " for TS).
    fn super_type_keyword(&self) -> &str;

    /// Keyword for interface implementation (e.g., "" for Rust, " implements " for TS).
    fn implements_keyword(&self) -> &str;

    /// Separator between name and type annotation (e.g., ": ").
    fn type_annotation_separator(&self) -> &str {
        ": "
    }

    /// The async keyword with trailing space (e.g., "async ").
    fn async_keyword(&self) -> &str {
        "async "
    }

    /// Opening delimiter for generic type parameters (e.g., "<" for Rust/TS, "[" for Go).
    fn generic_open(&self) -> &str {
        "<"
    }

    /// Closing delimiter for generic type parameters (e.g., ">" for Rust/TS, "]" for Go).
    fn generic_close(&self) -> &str {
        ">"
    }

    /// Qualify an import name for rendering in code.
    ///
    /// Default: return the resolved name as-is (TS/Rust import individual symbols).
    /// Go overrides this to prefix the package name (e.g., `"http.Server"`).
    fn qualify_import_name(&self, _module: &str, resolved_name: &str) -> String {
        resolved_name.to_string()
    }

    /// Optional kind suffix after the type name (e.g., Go's `type Foo struct`).
    ///
    /// Default: empty (TS/Rust put the kind keyword before the name).
    fn type_kind_suffix(&self, _kind: crate::spec::modifiers::TypeKind) -> &str {
        ""
    }

    /// Opening block delimiter appended after a function signature or type header.
    ///
    /// Default: `" {"` (brace languages). Python overrides to `":"`.
    fn block_open(&self) -> &str {
        " {"
    }

    /// Closing block delimiter emitted after a dedent at the end of a block.
    ///
    /// Default: `"}"` (brace languages). Python overrides to `""` (indent-only).
    fn block_close(&self) -> &str {
        "}"
    }

    /// Whether doc comments should be rendered inside the body (after block open)
    /// rather than above the declaration.
    ///
    /// Default: `false`. Python overrides to `true` (docstrings go inside the body).
    fn doc_comment_inside_body(&self) -> bool {
        false
    }

    /// Closing delimiter for base class / implements list.
    ///
    /// Default: `""`. Python overrides to `")"` to close `class Foo(Base):`.
    fn bases_close(&self) -> &str {
        ""
    }

    /// Content to emit for an abstract method with no body.
    ///
    /// Default: `""` (no body emitted). Python overrides to `"..."` (Ellipsis).
    fn empty_body(&self) -> &str {
        ""
    }

    /// Whether type annotations use type-before-name order (e.g., C: `int count`)
    /// rather than name-before-type (e.g., Rust: `count: i32`).
    ///
    /// Default: `false`. C overrides to `true`.
    fn type_before_name(&self) -> bool {
        false
    }

    /// Whether the return type appears before the function name (e.g., C: `int add(...)`)
    /// rather than after the parameters (e.g., Rust: `fn add(...) -> int`).
    ///
    /// Default: `false`. C overrides to `true`.
    fn return_type_is_prefix(&self) -> bool {
        false
    }

    /// Terminator appended after a type declaration's closing brace.
    ///
    /// Default: `""`. C overrides to `";"` for `struct Config { ... };`.
    fn type_close_terminator(&self) -> &str {
        ""
    }

    /// The keyword emitted when `is_abstract` is set on a function.
    ///
    /// Default: `"abstract "` (TypeScript). C++ overrides to `"virtual "`.
    fn abstract_keyword(&self) -> &str {
        "abstract "
    }

    /// Separator between super types in an inheritance list.
    ///
    /// Default: `", "`. C++ overrides to `", public "` for `class D : public B1, public B2`.
    fn super_type_separator(&self) -> &str {
        ", "
    }
}
