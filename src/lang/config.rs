//! Shared configuration types used across language implementations.
//!
//! These types live here (rather than inside individual `src/lang/*.rs` files)
//! because they represent cross-cutting concepts — quote style, optional-field
//! semantics, type presentation — that multiple languages express in similar ways.

use crate::spec::fun_spec::{FunctionSignatureStyle, ParamListStyle, WhereClauseStyle};
use crate::spec::modifiers::ConstructorDelegationStyle;
use crate::type_name::{
    AssociatedTypeStyle, BoundsPresentation, FunctionPresentation, GenericApplicationStyle,
    TypePresentation, WildcardPresentation,
};

/// Quote style for rendering string literals.
///
/// Used by `TypeScript`, `JavaScript`, and `Python` where either single or
/// double quotes are valid and the choice is a project style decision.
/// Languages with a fixed quote style (Rust, Java, Go, etc.) don't consult
/// this enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum QuoteStyle {
    /// Single quotes (`'hello'`).
    #[default]
    Single,
    /// Double quotes (`"hello"`).
    Double,
}

impl QuoteStyle {
    /// The quote character for this style.
    pub fn char(self) -> char {
        match self {
            QuoteStyle::Single => '\'',
            QuoteStyle::Double => '"',
        }
    }
}

/// How a language expresses that a field is optional (key may be absent).
///
/// This is distinct from nullability (value may be `null`), which is handled
/// by `TypeName::Optional`. A `FieldSpec` marked `is_optional` is rendered
/// using this style.
///
/// Examples:
///
/// | Language | Style | Rendered output |
/// |----------|-------|-----------------|
/// | TypeScript | `NameSuffix("?")` | `name?: T` |
/// | Rust | `TypeWrap { open: "Option<", close: ">" }` | `name: Option<T>` |
/// | Go | `TypePrefix("*")` | `name *T` |
/// | Python | `UnionWithNone(" \| ")` | `name: T \| None` |
/// | Kotlin, Swift, Dart | `TypeSuffix("?")` | `name: T?` or `T? name` |
/// | Java | `TypeWrap { open: "Optional<", close: ">" }` | `Optional<T> name` |
/// | C++ | `TypeWrap { open: "std::optional<", close: ">" }` | `std::optional<T> name` |
/// | C | `TypePrefix("*")` | `T *name` |
/// | JavaScript, Bash, Zsh | `Ignored` | field rendered without any optionality marker |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionalFieldStyle {
    /// Append a suffix to the field name. TypeScript: `name?: T`.
    NameSuffix(&'static str),
    /// Append a suffix to the type. Kotlin/Swift/Dart: `T?`.
    TypeSuffix(&'static str),
    /// Wrap the type in `open...close`. Rust `Option<T>`, Java `Optional<T>`,
    /// C++ `std::optional<T>`.
    TypeWrap {
        /// Opening wrapper, e.g. `"Option<"`.
        open: &'static str,
        /// Closing wrapper, e.g. `">"`.
        close: &'static str,
    },
    /// Prepend a prefix to the type. Go: `name *T`, C: `T *name`.
    TypePrefix(&'static str),
    /// Render as a union with `None`. Python: `T | None` (separator is
    /// language-configurable for future flexibility).
    UnionWithNone(&'static str),
    /// Optional fields are not expressible in this language's type system.
    /// The field is rendered without any marker.
    Ignored,
}

// ── Config structs ─────────────────────────────────────────────────

/// How each compound `TypeName` variant renders.
#[derive(Debug, Clone, Copy)]
pub struct TypePresentationConfig<'a> {
    /// `TypeName::Array(T)` — e.g. `T[]`, `Vec<T>`.
    pub array: TypePresentation<'a>,
    /// `TypeName::ReadonlyArray(T)` — `None` falls back to `readonly` + `array`.
    pub readonly_array: Option<TypePresentation<'a>>,
    /// `TypeName::Optional(T)` — e.g. `T | null`, `T?`.
    pub optional: TypePresentation<'a>,
    /// The absent literal for `Optional` with `Infix` presentation (e.g. `"null"`, `"None"`).
    pub optional_absent_literal: &'a str,
    /// `TypeName::Map { K, V }` — e.g. `Map<K, V>`, `dict[K, V]`.
    pub map: TypePresentation<'a>,
    /// `TypeName::Union(members)` — e.g. `A | B`.
    pub union: TypePresentation<'a>,
    /// `TypeName::Intersection(members)` — e.g. `A & B`.
    pub intersection: TypePresentation<'a>,
    /// `TypeName::Pointer(T)` — e.g. `*T`, `*const T`.
    pub pointer: TypePresentation<'a>,
    /// `TypeName::Slice(T)` — e.g. `[]T`, `[T]`.
    pub slice: TypePresentation<'a>,
    /// `TypeName::Tuple(elements)` — e.g. `(A, B)`.
    pub tuple: TypePresentation<'a>,
    /// `TypeName::Reference { mutable: false }` — e.g. `&T`.
    pub reference: TypePresentation<'a>,
    /// `TypeName::Reference { mutable: true }` — e.g. `&mut T`.
    pub reference_mut: TypePresentation<'a>,
    /// `TypeName::Function { params, return_type }` — e.g. `(A, B) => R`.
    pub function: FunctionPresentation<'a>,
    /// `TypeName::AssociatedType` — e.g. `<T as Trait>::Assoc`.
    pub associated_type: AssociatedTypeStyle<'a>,
    /// `TypeName::ImplTrait` — e.g. `impl Trait`.
    pub impl_trait: BoundsPresentation<'a>,
    /// `TypeName::DynTrait` — e.g. `dyn Trait`.
    pub dyn_trait: BoundsPresentation<'a>,
    /// `TypeName::Wildcard` — e.g. `?`, `? extends T`.
    pub wildcard: WildcardPresentation<'a>,
}

impl Default for TypePresentationConfig<'_> {
    fn default() -> Self {
        Self {
            array: TypePresentation::Postfix { suffix: "[]" },
            readonly_array: None,
            optional: TypePresentation::Infix { sep: " | " },
            optional_absent_literal: "null",
            map: TypePresentation::GenericWrap { name: "Map" },
            union: TypePresentation::Infix { sep: " | " },
            intersection: TypePresentation::Infix { sep: " & " },
            pointer: TypePresentation::Prefix { prefix: "*" },
            slice: TypePresentation::Prefix { prefix: "[]" },
            tuple: TypePresentation::Delimited {
                open: "(",
                sep: ", ",
                close: ")",
            },
            reference: TypePresentation::Prefix { prefix: "" },
            reference_mut: TypePresentation::Prefix { prefix: "" },
            function: FunctionPresentation {
                keyword: "",
                params_open: "(",
                params_sep: ", ",
                params_close: ")",
                arrow: " => ",
                return_first: false,
                curried: false,
                wrapper_open: "",
                wrapper_close: "",
            },
            associated_type: AssociatedTypeStyle::QualifiedPath {
                open: "<",
                as_kw: " as ",
                close_sep: ">::",
                simple_sep: "::",
            },
            impl_trait: BoundsPresentation {
                keyword: "impl ",
                separator: " + ",
            },
            dyn_trait: BoundsPresentation {
                keyword: "dyn ",
                separator: " + ",
            },
            wildcard: WildcardPresentation {
                unbounded: "?",
                upper_keyword: "? extends ",
                lower_keyword: "? super ",
            },
        }
    }
}

/// Generic type parameter delimiters and constraints.
#[derive(Debug, Clone, Copy)]
pub struct GenericSyntaxConfig<'a> {
    /// Opening delimiter (e.g. `"<"`, `"["`).
    pub open: &'a str,
    /// Closing delimiter (e.g. `">"`, `"]"`).
    pub close: &'a str,
    /// How `TypeName::Generic` application renders.
    pub application_style: GenericApplicationStyle,
    /// Keyword introducing a bound (e.g. `": "`, `" extends "`).
    pub constraint_keyword: &'a str,
    /// Separator between multiple bounds (e.g. `" + "`, `" & "`).
    pub constraint_separator: &'a str,
    /// Keyword for context bounds (e.g. Scala `" : "`). Defaults to `constraint_keyword`.
    pub context_bound_keyword: &'a str,
}

impl Default for GenericSyntaxConfig<'_> {
    fn default() -> Self {
        Self {
            open: "<",
            close: ">",
            application_style: GenericApplicationStyle::Delimited,
            constraint_keyword: ": ",
            constraint_separator: " + ",
            context_bound_keyword: ": ",
        }
    }
}

/// Block delimiters, indentation, and statement termination.
#[derive(Debug, Clone, Copy)]
pub struct BlockSyntaxConfig<'a> {
    /// Opening block delimiter after a signature (e.g. `" {"`, `":"`).
    pub block_open: &'a str,
    /// Closing block delimiter (e.g. `"}"`, `""`).
    pub block_close: &'a str,
    /// Indentation unit (e.g. `"  "`, `"\t"`).
    pub indent_unit: &'a str,
    /// Whether statements end with semicolons.
    pub uses_semicolons: bool,
    /// Terminator after a field declaration (e.g. `","`, `";"`).
    pub field_terminator: &'a str,
    /// Terminator after a type's closing brace (e.g. `";"` for C structs).
    pub type_close_terminator: &'a str,
    /// Closing delimiter for base class / implements list (e.g. `")"` for Python).
    pub bases_close: &'a str,
}

impl Default for BlockSyntaxConfig<'_> {
    fn default() -> Self {
        Self {
            block_open: " {",
            block_close: "}",
            indent_unit: "  ",
            uses_semicolons: true,
            field_terminator: ",",
            type_close_terminator: "",
            bases_close: "",
        }
    }
}

/// Function signature syntax.
#[derive(Debug, Clone, Copy)]
pub struct FunctionSyntaxConfig<'a> {
    /// Separator between parameters and return type (e.g. `" -> "`, `": "`).
    pub return_type_separator: &'a str,
    /// Keyword for async functions (e.g. `"async "`).
    pub async_keyword: &'a str,
    /// Keyword for abstract methods (e.g. `"abstract "`, `"virtual "`).
    pub abstract_keyword: &'a str,
    /// How parameter lists are formatted (tupled vs curried).
    pub param_list_style: ParamListStyle,
    /// How signatures are rendered (merged vs split type signature).
    pub function_signature_style: FunctionSignatureStyle,
    /// Function keyword for constructors (e.g. `""`, `"def"`, `"fn"`).
    pub constructor_keyword: &'a str,
    /// Where constructor delegation calls are placed (body vs signature).
    pub constructor_delegation_style: ConstructorDelegationStyle,
    /// How where-clause constraints are rendered (inline vs block).
    pub where_clause_style: WhereClauseStyle,
    /// Content emitted for abstract methods with no body (e.g. `""`, `"..."`).
    pub empty_body: &'a str,
}

impl Default for FunctionSyntaxConfig<'_> {
    fn default() -> Self {
        Self {
            return_type_separator: ": ",
            async_keyword: "async ",
            abstract_keyword: "abstract ",
            param_list_style: ParamListStyle::Tupled,
            function_signature_style: FunctionSignatureStyle::Merged,
            constructor_keyword: "",
            constructor_delegation_style: ConstructorDelegationStyle::Body,
            where_clause_style: WhereClauseStyle::Inline,
            empty_body: "",
        }
    }
}

/// Type declaration syntax (inheritance, annotation, field order).
#[derive(Debug, Clone, Copy)]
pub struct TypeDeclSyntaxConfig<'a> {
    /// Whether type annotations use type-before-name order (e.g. C: `int count`).
    pub type_before_name: bool,
    /// Whether the return type appears before the function name (e.g. C: `int add(...)`).
    pub return_type_is_prefix: bool,
    /// Separator between name and type annotation (e.g. `": "`).
    pub type_annotation_separator: &'a str,
    /// Keyword for super type / base class (e.g. `" extends "`, `": "`).
    pub super_type_keyword: &'a str,
    /// Separator between super types (e.g. `", "`, `", public "`).
    pub super_type_separator: &'a str,
    /// Override separator for 2nd+ super type (e.g. Scala: `Some(" with ")`).
    pub super_type_subsequent_separator: Option<&'a str>,
    /// Keyword for interface implementation (e.g. `" implements "`).
    pub implements_keyword: &'a str,
    /// Whether type alias uses `typedef target name` order (C).
    pub type_alias_target_first: bool,
    /// Whether the language supports primary constructors on type declarations.
    pub supports_primary_constructor: bool,
}

impl Default for TypeDeclSyntaxConfig<'_> {
    fn default() -> Self {
        Self {
            type_before_name: false,
            return_type_is_prefix: false,
            type_annotation_separator: ": ",
            super_type_keyword: "",
            super_type_separator: ", ",
            super_type_subsequent_separator: None,
            implements_keyword: "",
            type_alias_target_first: false,
            supports_primary_constructor: false,
        }
    }
}

/// How an enum variant's value is formatted.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum VariantValueFormat {
    /// `NAME = value` — TypeScript, Rust, C, Swift, etc.
    #[default]
    Assignment,
    /// `NAME(value)` — Java, Kotlin constructor-arg style.
    ConstructorArg,
}

/// Enum variant formatting, annotation syntax, and field mutability keywords.
#[derive(Debug, Clone, Copy)]
pub struct EnumAndAnnotationConfig<'a> {
    /// Prefix before each enum variant (e.g. `""`, `"case "`).
    pub variant_prefix: &'a str,
    /// Prefix before the first variant only. `None` = same as `variant_prefix`.
    pub variant_prefix_first: Option<&'a str>,
    /// Separator after each variant (e.g. `","`).
    pub variant_separator: &'a str,
    /// Whether the separator appears after the last variant too.
    pub variant_trailing_separator: bool,
    /// How the variant value is rendered (assignment `= val` vs constructor `(val)`).
    pub variant_value_format: VariantValueFormat,
    /// Whether enum variants are emitted before fields (Java/Kotlin pattern).
    pub variants_before_fields: bool,
    /// Terminator emitted after the last variant when fields/methods follow (e.g. `";"`).
    pub variant_section_terminator: &'a str,
    /// Prefix wrapping an annotation name (e.g. `"@"`, `"#["`).
    pub annotation_prefix: &'a str,
    /// Suffix closing an annotation (e.g. `""`, `"]"`).
    pub annotation_suffix: &'a str,
    /// Keyword for readonly/immutable fields (e.g. `"const "`, `"readonly "`).
    pub readonly_keyword: &'a str,
    /// Keyword for mutable fields (e.g. `""`, `"var "`).
    pub mutable_field_keyword: &'a str,
}

impl Default for EnumAndAnnotationConfig<'_> {
    fn default() -> Self {
        Self {
            variant_prefix: "",
            variant_prefix_first: None,
            variant_separator: ",",
            variant_trailing_separator: false,
            variant_value_format: VariantValueFormat::Assignment,
            variants_before_fields: false,
            variant_section_terminator: "",
            annotation_prefix: "@",
            annotation_suffix: "",
            readonly_keyword: "const ",
            mutable_field_keyword: "",
        }
    }
}
