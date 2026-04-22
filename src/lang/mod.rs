/// Bash shell language support.
pub mod bash;
/// C language support.
pub mod c_lang;
/// Shared configuration types (quote style, optional-field rendering).
pub mod config;
/// C++ language support.
pub mod cpp_lang;
/// Dart language support.
pub mod dart;
/// Go language support.
pub mod go_lang;
/// Java language support.
pub mod java_lang;
/// JavaScript language support.
pub mod javascript;
/// Kotlin language support.
pub mod kotlin;
/// Python language support.
pub mod python;
/// Rust language support.
pub mod rust_lang;
/// Swift language support.
pub mod swift;
/// TypeScript language support.
pub mod typescript;
/// Zsh shell language support.
pub mod zsh;

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

    // --- Type presentation (data-driven, no BoxDoc) ---

    /// How `TypeName::Array(T)` renders.
    ///
    /// Default: `Postfix { suffix: "[]" }` (TypeScript `T[]`).
    fn present_array(&self) -> crate::type_name::TypePresentation<'_> {
        crate::type_name::TypePresentation::Postfix { suffix: "[]" }
    }

    /// How `TypeName::ReadonlyArray(T)` renders.
    ///
    /// Default: `None` — renders as `readonly ` + the array presentation.
    /// Override to return `Some(...)` for languages with distinct readonly array syntax.
    fn present_readonly_array(&self) -> Option<crate::type_name::TypePresentation<'_>> {
        None
    }

    /// How `TypeName::Optional(T)` renders.
    ///
    /// Default: `Infix { sep: " | " }` — renders `T | null` (TypeScript style).
    /// When using `Infix`, the absent literal from `optional_absent_literal()` is
    /// automatically appended as the second member.
    fn present_optional(&self) -> crate::type_name::TypePresentation<'_> {
        crate::type_name::TypePresentation::Infix { sep: " | " }
    }

    /// The literal used for the "absent" case in Optional when using `Infix` presentation.
    ///
    /// Default: `"null"`. Python: `"None"`.
    fn optional_absent_literal(&self) -> &str {
        "null"
    }

    /// How `TypeName::Map { K, V }` renders.
    ///
    /// Default: `GenericWrap { name: "Map" }`.
    fn present_map(&self) -> crate::type_name::TypePresentation<'_> {
        crate::type_name::TypePresentation::GenericWrap { name: "Map" }
    }

    /// How `TypeName::Union(members)` renders.
    ///
    /// Default: `Infix { sep: " | " }`.
    fn present_union(&self) -> crate::type_name::TypePresentation<'_> {
        crate::type_name::TypePresentation::Infix { sep: " | " }
    }

    /// How `TypeName::Intersection(members)` renders.
    ///
    /// Default: `Infix { sep: " & " }`.
    fn present_intersection(&self) -> crate::type_name::TypePresentation<'_> {
        crate::type_name::TypePresentation::Infix { sep: " & " }
    }

    /// How `TypeName::Pointer(T)` renders.
    ///
    /// Default: `Prefix { prefix: "*" }`.
    fn present_pointer(&self) -> crate::type_name::TypePresentation<'_> {
        crate::type_name::TypePresentation::Prefix { prefix: "*" }
    }

    /// How `TypeName::Slice(T)` renders.
    ///
    /// Default: `Prefix { prefix: "[]" }`.
    fn present_slice(&self) -> crate::type_name::TypePresentation<'_> {
        crate::type_name::TypePresentation::Prefix { prefix: "[]" }
    }

    /// How `TypeName::Function { params, return_type }` renders.
    ///
    /// Default: TypeScript `(A, B) => R`.
    fn present_function(&self) -> crate::type_name::FunctionPresentation<'_> {
        crate::type_name::FunctionPresentation {
            keyword: "",
            params_open: "(",
            params_sep: ", ",
            params_close: ")",
            arrow: " => ",
            return_first: false,
            curried: false,
            wrapper_open: "",
            wrapper_close: "",
        }
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

    /// Whether doc comments should be emitted before annotations/attributes.
    ///
    /// Default: `true`. Most languages (Rust, Go, TypeScript) put doc comments
    /// above annotations. Java overrides to `false` (`@Override` before Javadoc).
    fn doc_before_annotations(&self) -> bool {
        true
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

    /// Keyword emitted for readonly/immutable fields.
    ///
    /// In type-before-name languages (C/C++/Java): appears before the type.
    /// In name-before-type languages (TS/Kotlin): appears before the name.
    ///
    /// Default: `"const "` (C/C++). Java overrides to `"final "`,
    /// TypeScript to `"readonly "`, Kotlin to `"val "`.
    fn readonly_keyword(&self) -> &str {
        "const "
    }

    /// Keyword emitted for mutable fields in name-before-type languages.
    ///
    /// Default: `""` (most languages have no mutable keyword).
    /// Kotlin overrides to `"var "`.
    fn mutable_field_keyword(&self) -> &str {
        ""
    }

    /// How this language expresses that a field is optional (key may be absent).
    ///
    /// Consulted by `FieldSpec::emit()` when `is_optional` is set. Languages
    /// that can't express optional fields return `OptionalFieldStyle::Ignored`
    /// and the field is rendered as if it were required.
    ///
    /// Default: `OptionalFieldStyle::Ignored`.
    fn optional_field_style(&self) -> crate::lang::config::OptionalFieldStyle {
        crate::lang::config::OptionalFieldStyle::Ignored
    }

    // --- Phase 3: enum variant support ---

    /// Prefix before each enum variant name.
    ///
    /// Default: `""`. Swift overrides to `"case "`.
    fn enum_variant_prefix(&self) -> &str {
        ""
    }

    /// Separator after each enum variant (e.g., `","` for most languages).
    ///
    /// Default: `","`. Python and Swift override to `""`.
    fn enum_variant_separator(&self) -> &str {
        ","
    }

    /// Whether the separator appears after the last variant too (trailing comma).
    ///
    /// Default: `false`. Rust and TypeScript override to `true`.
    fn enum_variant_trailing_separator(&self) -> bool {
        false
    }

    // --- Phase 3: annotation support ---

    /// Prefix and suffix wrapping an annotation name.
    ///
    /// Default: `("@", "")` → `@Name(args)`.
    /// Rust: `("#[", "]")` → `#[name(args)]`.
    /// C++: `("[[", "]]")` → `[[name(args)]]`.
    /// C: `("__attribute__((", "))")` → `__attribute__((name(args)))`.
    fn render_annotation_prefix(&self) -> (&str, &str) {
        ("@", "")
    }

    // --- Phase 3: constructor support ---

    /// Function keyword used for constructors.
    ///
    /// When `FunSpec::is_constructor()` is set, this keyword is used instead of
    /// `function_keyword()`. Default: `""` (no keyword prefix — works for TS/JS,
    /// Java, C++, Dart, Swift, Kotlin where constructors have no function keyword).
    ///
    /// Python overrides to `"def"` (`def __init__`).
    /// Rust overrides to `"fn"` (`fn new`).
    fn constructor_keyword(&self) -> &str {
        ""
    }

    /// Where a constructor delegation call (`super(...)` / `this(...)`) is placed.
    ///
    /// Default: `Body` — the delegation call is emitted as the first statement
    /// in the constructor body (TS, JS, Java, Dart, Swift, Python, C++).
    ///
    /// Kotlin overrides to `Signature` — the delegation call appears between the
    /// parameter list and the body: `constructor(x: Int) : this(x, 0) { ... }`.
    fn constructor_delegation_style(&self) -> crate::spec::modifiers::ConstructorDelegationStyle {
        crate::spec::modifiers::ConstructorDelegationStyle::Body
    }

    /// Whether this language supports primary constructors on type declarations.
    ///
    /// When true, `TypeSpec` will render primary constructor parameters after the
    /// type name: `class Foo(val x: Int, val y: String)`.
    ///
    /// Default: `false`. Kotlin overrides to `true`.
    fn supports_primary_constructor(&self) -> bool {
        false
    }

    // --- Phase 3: property support ---

    /// How `PropertySpec` renders: accessor methods or inline field body.
    ///
    /// Default: `Accessor` — emits `get name()` / `set name(v)` methods (TS/JS).
    /// Swift and Kotlin override to `Field` — emits a field with inline `get`/`set` blocks.
    fn property_style(&self) -> crate::spec::modifiers::PropertyStyle {
        crate::spec::modifiers::PropertyStyle::Accessor
    }

    /// The keyword for a property getter in field-style rendering.
    ///
    /// Default: `"get"` (Swift: `get { ... }`).
    /// Kotlin overrides to `"get()"`.
    fn property_getter_keyword(&self) -> &str {
        "get"
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
