use pretty::BoxDoc;

use crate::import::ImportRef;
use crate::lang::RendererLang;
///
/// Each variant describes a structural pattern for assembling already-rendered
/// inner type docs into the output. The rendering engine in `type_name.rs`
/// interprets these patterns — language implementations never build `BoxDoc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypePresentation<'a> {
    /// `name<P1, P2>` — delimiters from `generic_syntax().open`/`.close`.
    GenericWrap {
        /// The wrapper type name (e.g., `"Vec"`, `"Option"`, `"HashMap"`).
        name: &'a str,
    },
    /// `prefix inner` — `*T`, `&T`, `[]T`, `&mut T`.
    Prefix {
        /// The prefix string (e.g., `"*"`, `"[]"`, `"*const "`).
        prefix: &'a str,
    },
    /// `inner suffix` — `T[]`, `T?`, `T*`.
    Postfix {
        /// The suffix string (e.g., `"[]"`, `"?"`).
        suffix: &'a str,
    },
    /// `prefix inner suffix` — `const T&`, `const T*`.
    Surround {
        /// The prefix string (e.g., `"const "`).
        prefix: &'a str,
        /// The suffix string (e.g., `"&"`, `"*"`).
        suffix: &'a str,
    },
    /// `open P1 sep P2 sep ... close` — `(A, B)`, `[T]`, `[K: V]`, `dict[K, V]`.
    Delimited {
        /// Opening delimiter.
        open: &'a str,
        /// Separator between elements.
        sep: &'a str,
        /// Closing delimiter.
        close: &'a str,
    },
    /// `P1 sep P2 sep ... Pn` — `A | B`, `A & B`, `A + B`.
    Infix {
        /// Separator between elements (e.g., `" | "`, `" & "`).
        sep: &'a str,
    },
}

/// Controls how `TypeName::Generic { base, params }` renders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum GenericApplicationStyle {
    /// `Base<P1, P2>` or `Base[P1, P2]` — uses `generic_syntax().open`/`.close`.
    Delimited,
    /// `Base P1 P2` — Haskell-style prefix juxtaposition.
    /// Compound params are parenthesized: `Either String (Maybe Int)`.
    PrefixJuxtaposition,
    /// `P1 Base` (single) or `(P1, P2) Base` (multi) — OCaml-style postfix.
    PostfixJuxtaposition,
}

/// Syntactic pattern for rendering a function type expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FunctionPresentation<'a> {
    /// Keyword before the param list (e.g., `"fn"`, `"func"`, `""`).
    pub keyword: &'a str,
    /// Opening delimiter for the param list (e.g., `"("`, `"Callable[["`).
    pub params_open: &'a str,
    /// Separator between params (e.g., `", "`).
    pub params_sep: &'a str,
    /// Closing delimiter for the param list (e.g., `")"`, `"]]"`).
    pub params_close: &'a str,
    /// Arrow/separator between params and return type (e.g., `" -> "`, `" => "`).
    pub arrow: &'a str,
    /// Whether the return type comes before the params (Dart: `R Function(A, B)`).
    pub return_first: bool,
    /// Whether to render curried (Haskell: `A -> B -> R` instead of `(A, B) -> R`).
    pub curried: bool,
    /// Outer wrapper opening (e.g., C++ `"std::function<"`).
    pub wrapper_open: &'a str,
    /// Outer wrapper closing (e.g., C++ `">"`).
    pub wrapper_close: &'a str,
}

impl Default for FunctionPresentation<'_> {
    fn default() -> Self {
        Self {
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
}

/// How `TypeName::AssociatedType` renders across languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssociatedTypeStyle<'a> {
    /// Rust-style: `<Base as Qual>::Member` or `Base::Member`.
    QualifiedPath {
        /// Opening delimiter for qualified path (e.g., `"<"`).
        open: &'a str,
        /// Keyword between base and qualifier (e.g., `" as "`).
        as_kw: &'a str,
        /// Closing delimiter + separator to member for qualified path (e.g., `">::"`).
        close_sep: &'a str,
        /// Separator to member for simple (non-qualified) path (e.g., `"::"`).
        simple_sep: &'a str,
    },
    /// Dot access: `Base.Member` (Java, Kotlin, Python).
    /// The qualifier is ignored — only `base.member` is emitted.
    DotAccess,
    /// Index access: `Base["Member"]` (TypeScript).
    /// The qualifier is ignored — only `base["member"]` is emitted.
    IndexAccess {
        /// Opening delimiter (e.g., `"[\"`).
        open: &'a str,
        /// Closing delimiter (e.g., `"\"]"`).
        close: &'a str,
    },
}

/// How `impl Trait` / `dyn Trait` bounds render.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundsPresentation<'a> {
    /// Keyword prefix (e.g., `"impl "`, `"dyn "`).
    pub keyword: &'a str,
    /// Separator between bounds (e.g., `" + "`).
    pub separator: &'a str,
}

/// How wildcard types render.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WildcardPresentation<'a> {
    /// Unbounded wildcard (e.g., `"?"`, `"*"`, `"_"`, `"any"`).
    pub unbounded: &'a str,
    /// Upper-bound prefix (e.g., `"? extends "`, `"out "`).
    pub upper_keyword: &'a str,
    /// Lower-bound prefix (e.g., `"? super "`, `"in "`).
    pub lower_keyword: &'a str,
}

/// A type name reference in generated code.
///
/// `TypeName` represents a type as it should appear in the output. The
/// [`Importable`](TypeName::Importable) variant automatically tracks imports
/// through the two-pass rendering system, so using a `TypeName` with `%T` in a
/// `CodeBlock` is enough to generate the corresponding import statement.
///
/// Variants cover common type constructs across all supported languages:
/// arrays, generics, unions, optionals, maps, pointers, slices, and function types.
/// Use [`TypeName::raw()`] as an escape hatch for forms not covered by the model.
///
/// # Examples
///
/// ```
/// use sigil_stitch::type_name::TypeName;
/// use sigil_stitch::lang::typescript::TypeScript;
///
/// // Importable type (generates `import { User } from './models'`):
/// let user = TypeName::importable("./models", "User");
///
/// // Primitive (no import needed):
/// let num = TypeName::primitive("number");
///
/// // Generic: Promise<User>
/// let promise = TypeName::generic(
///     TypeName::primitive("Promise"),
///     vec![user],
/// );
///
/// // Optional: string | null
/// let maybe_str = TypeName::optional(TypeName::primitive("string"));
/// ```
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TypeName {
    /// A type that requires an import statement.
    Importable {
        /// The module path to import from.
        module: String,
        /// The type name being imported.
        name: String,
        /// Whether this is a type-only import.
        is_type_only: bool,
        /// Optional preferred alias for this import (e.g., `Foo as Bar`).
        alias: Option<String>,
        /// When true, render as `module{sep}name` inline (e.g., `serde_json::Value`)
        /// and skip import generation. The separator comes from
        /// [`RendererLang::module_separator()`]. Falls back to unqualified rendering
        /// if the language returns `None`.
        #[serde(default)]
        qualified: bool,
    },
    /// A primitive/built-in type (no import needed).
    Primitive(String),
    /// Array type. TS: `T[]`, Go: `[]T`, Rust: `Vec<T>`.
    Array(Box<TypeName>),
    /// Readonly array type. TS: `readonly T[]`. Other languages fall back to
    /// their `Array` rendering since they lack a direct readonly-array form.
    ReadonlyArray(Box<TypeName>),
    /// Generic type. e.g., `Promise<User>`, `HashMap<String, User>`.
    Generic {
        /// The base type (e.g., `Promise`, `HashMap`).
        base: Box<TypeName>,
        /// The type parameters.
        params: Vec<TypeName>,
    },
    /// Union type. TS: `A | B | C`.
    Union(Vec<TypeName>),
    /// Intersection type. TS: `A & B`.
    Intersection(Vec<TypeName>),
    /// Pointer type. Go: `*T`.
    Pointer(Box<TypeName>),
    /// Slice type. Go: `[]T`.
    Slice(Box<TypeName>),
    /// Map type. Go: `map[K]V`, TS: `Record<K, V>`.
    Map {
        /// The key type.
        key: Box<TypeName>,
        /// The value type.
        value: Box<TypeName>,
    },
    /// Optional type. TS: `T | null`, Rust: `Option<T>`.
    Optional(Box<TypeName>),
    /// Tuple type. Rust: `(A, B)`, TS: `[A, B]`, Python: `tuple[A, B]`.
    Tuple(Vec<TypeName>),
    /// Reference type. Rust: `&T` / `&mut T` / `&'a T`, C++: `const T&` / `T&`.
    Reference {
        /// The referenced type.
        inner: Box<TypeName>,
        /// Whether the reference is mutable.
        mutable: bool,
        /// Optional lifetime (Rust only, e.g., `'a`).
        #[serde(default)]
        lifetime: Option<String>,
    },
    /// Associated/path-dependent type. Rust: `<T as Iterator>::Item`, TS: `T["key"]`.
    AssociatedType {
        /// The base type (e.g., `T`, `Vec<i32>`).
        base: Box<TypeName>,
        /// Optional qualifier trait (Rust: `Iterator` in `<T as Iterator>::Item`).
        qualifier: Option<Box<TypeName>>,
        /// The projected member name (e.g., `"Item"`, `"key"`).
        member: String,
    },
    /// `impl Trait` bounds. Rust: `impl Display + Debug`.
    ImplTrait {
        /// The trait bounds.
        bounds: Vec<TypeName>,
    },
    /// `dyn Trait` bounds. Rust: `dyn Error + Send`.
    DynTrait {
        /// The trait bounds.
        bounds: Vec<TypeName>,
    },
    /// Wildcard type. Java: `?`, `? extends T`, `? super T`. Kotlin: `*`, `out T`, `in T`.
    Wildcard {
        /// Upper bound (Java: `? extends T`, Kotlin: `out T`).
        upper_bound: Option<Box<TypeName>>,
        /// Lower bound (Java: `? super T`, Kotlin: `in T`).
        lower_bound: Option<Box<TypeName>>,
    },
    /// Function type. TS: `(a: A, b: B) => R`.
    Function {
        /// The parameter types.
        params: Vec<TypeName>,
        /// The return type.
        return_type: Box<TypeName>,
    },
    /// Raw string escape hatch. No import tracking.
    Raw(String),
}

impl TypeName {
    /// Create an importable type name.
    pub fn importable(module: &str, name: &str) -> Self {
        TypeName::Importable {
            module: module.to_string(),
            name: name.to_string(),
            is_type_only: false,
            alias: None,
            qualified: false,
        }
    }

    /// Create a type-only importable type name (TypeScript `import type`).
    pub fn importable_type(module: &str, name: &str) -> Self {
        TypeName::Importable {
            module: module.to_string(),
            name: name.to_string(),
            is_type_only: true,
            alias: None,
            qualified: false,
        }
    }

    /// Create a primitive type name (no import).
    pub fn primitive(name: &str) -> Self {
        TypeName::Primitive(name.to_string())
    }

    /// Create a qualified type name that renders inline as `module{sep}name`.
    ///
    /// The separator comes from [`RendererLang::module_separator()`] — `"::"` for
    /// Rust/C++, `"."` for Go/Python/Java/etc.  No import statement is generated.
    ///
    /// If the target language returns `None` from `module_separator()`, the
    /// qualified flag is silently ignored and the type renders as just `name`.
    ///
    /// ```ignore
    /// TypeName::qualified("serde_json", "Value")      // Rust: serde_json::Value
    /// TypeName::qualified("super", "Foo")              // Rust: super::Foo
    /// TypeName::qualified("java.util", "HashMap")      // Java: java.util.HashMap
    /// ```
    pub fn qualified(module: &str, name: &str) -> Self {
        TypeName::Importable {
            module: module.to_string(),
            name: name.to_string(),
            is_type_only: false,
            alias: None,
            qualified: true,
        }
    }

    /// Returns true if this type name renders to an empty string.
    ///
    /// Used by `ParameterSpec` to skip the type annotation when no type
    /// is specified (e.g., Python's bare `self` parameter).
    pub fn is_empty(&self) -> bool {
        matches!(self, TypeName::Primitive(s) | TypeName::Raw(s) if s.is_empty())
    }

    /// Set a preferred import alias for this type name.
    ///
    /// When a TypeName has an alias, the import statement will use it
    /// (e.g., `import { Foo as Bar }`) and `%T` rendering will emit the alias.
    ///
    /// Only affects `Importable` variants; other variants are returned unchanged.
    pub fn with_alias(mut self, alias: &str) -> Self {
        if let TypeName::Importable {
            alias: ref mut a, ..
        } = self
        {
            *a = Some(alias.to_string());
        }
        self
    }

    /// Mark this type for qualified inline rendering (`module{sep}name`).
    ///
    /// When qualified, no import statement is generated and the type renders
    /// with its full module path. Only affects `Importable` variants; other
    /// variants are returned unchanged.
    ///
    /// See [`TypeName::qualified()`] for details on separator and fallback behavior.
    pub fn qualify(mut self) -> Self {
        if let TypeName::Importable {
            qualified: ref mut q,
            ..
        } = self
        {
            *q = true;
        }
        self
    }

    /// Create an array type.
    pub fn array(inner: TypeName) -> Self {
        TypeName::Array(Box::new(inner))
    }

    /// Create a readonly array type (TypeScript: `readonly T[]`).
    ///
    /// In languages without a direct readonly-array form (Go, most C-family),
    /// this renders identically to [`TypeName::array`]. Use this variant only
    /// when the readonly distinction carries information the output should
    /// preserve (e.g. TypeScript interface fields).
    pub fn readonly_array(inner: TypeName) -> Self {
        TypeName::ReadonlyArray(Box::new(inner))
    }

    /// Create a generic type (e.g., `Promise<User>`).
    pub fn generic(base: TypeName, params: Vec<TypeName>) -> Self {
        TypeName::Generic {
            base: Box::new(base),
            params,
        }
    }

    /// Create a union type (e.g., `A | B | C`).
    pub fn union(members: Vec<TypeName>) -> Self {
        TypeName::Union(members)
    }

    /// Create an intersection type (e.g., `A & B`).
    pub fn intersection(members: Vec<TypeName>) -> Self {
        TypeName::Intersection(members)
    }

    /// Create a pointer type (e.g., Go `*T`).
    pub fn pointer(inner: TypeName) -> Self {
        TypeName::Pointer(Box::new(inner))
    }

    /// Create a slice type (e.g., Go `[]T`).
    pub fn slice(inner: TypeName) -> Self {
        TypeName::Slice(Box::new(inner))
    }

    /// Create a map type (e.g., `map[K]V`).
    pub fn map(key: TypeName, value: TypeName) -> Self {
        TypeName::Map {
            key: Box::new(key),
            value: Box::new(value),
        }
    }

    /// Create an optional type.
    pub fn optional(inner: TypeName) -> Self {
        TypeName::Optional(Box::new(inner))
    }

    /// Create a tuple type (e.g., Rust `(A, B)`, TS `[A, B]`).
    pub fn tuple(elements: Vec<TypeName>) -> Self {
        TypeName::Tuple(elements)
    }

    /// Create a unit type (empty tuple: Rust `()`).
    pub fn unit() -> Self {
        TypeName::Tuple(Vec::new())
    }

    /// Create a shared reference type (Rust `&T`, C++ `const T&`).
    pub fn reference(inner: TypeName) -> Self {
        TypeName::Reference {
            inner: Box::new(inner),
            mutable: false,
            lifetime: None,
        }
    }

    /// Create a mutable reference type (Rust `&mut T`, C++ `T&`).
    pub fn reference_mut(inner: TypeName) -> Self {
        TypeName::Reference {
            inner: Box::new(inner),
            mutable: true,
            lifetime: None,
        }
    }

    /// Create a shared reference with a lifetime (Rust `&'a T`).
    pub fn reference_with_lifetime(inner: TypeName, lifetime: &str) -> Self {
        TypeName::Reference {
            inner: Box::new(inner),
            mutable: false,
            lifetime: Some(lifetime.to_string()),
        }
    }

    /// Create a mutable reference with a lifetime (Rust `&'a mut T`).
    pub fn reference_mut_with_lifetime(inner: TypeName, lifetime: &str) -> Self {
        TypeName::Reference {
            inner: Box::new(inner),
            mutable: true,
            lifetime: Some(lifetime.to_string()),
        }
    }

    /// Create a function type.
    pub fn function(params: Vec<TypeName>, return_type: TypeName) -> Self {
        TypeName::Function {
            params,
            return_type: Box::new(return_type),
        }
    }

    /// Create a raw type string (escape hatch, no import tracking).
    pub fn raw(s: &str) -> Self {
        TypeName::Raw(s.to_string())
    }

    /// Create an associated/path-dependent type with a qualifier.
    ///
    /// Rust: `<base as qualifier>::member` (e.g., `<T as Iterator>::Item`).
    pub fn associated_type(base: TypeName, qualifier: Option<TypeName>, member: &str) -> Self {
        TypeName::AssociatedType {
            base: Box::new(base),
            qualifier: qualifier.map(Box::new),
            member: member.to_string(),
        }
    }

    /// Create a simple member type (no qualifier).
    ///
    /// Rust: `base::member` (e.g., `Self::Output`).
    pub fn member_type(base: TypeName, member: &str) -> Self {
        TypeName::AssociatedType {
            base: Box::new(base),
            qualifier: None,
            member: member.to_string(),
        }
    }

    /// Create an `impl Trait` type (Rust: `impl Display + Debug`).
    pub fn impl_trait(bounds: Vec<TypeName>) -> Self {
        TypeName::ImplTrait { bounds }
    }

    /// Create a `dyn Trait` type (Rust: `dyn Error + Send`).
    pub fn dyn_trait(bounds: Vec<TypeName>) -> Self {
        TypeName::DynTrait { bounds }
    }

    /// Create an unbounded wildcard type (Java: `?`, Kotlin: `*`).
    pub fn wildcard() -> Self {
        TypeName::Wildcard {
            upper_bound: None,
            lower_bound: None,
        }
    }

    /// Create a wildcard with an upper bound (Java: `? extends T`, Kotlin: `out T`).
    pub fn wildcard_extends(bound: TypeName) -> Self {
        TypeName::Wildcard {
            upper_bound: Some(Box::new(bound)),
            lower_bound: None,
        }
    }

    /// Create a wildcard with a lower bound (Java: `? super T`, Kotlin: `in T`).
    pub fn wildcard_super(bound: TypeName) -> Self {
        TypeName::Wildcard {
            upper_bound: None,
            lower_bound: Some(Box::new(bound)),
        }
    }

    /// Get the simple name of this type (for import resolution lookups).
    pub fn simple_name(&self) -> Option<&str> {
        match self {
            TypeName::Importable { name, .. } => Some(name),
            TypeName::Primitive(name) => Some(name),
            TypeName::Generic { base, .. } => base.simple_name(),
            TypeName::Raw(s) => Some(s),
            _ => None,
        }
    }

    /// Collect import references from this type name (recursive).
    ///
    /// Callers working with `CodeBlock` or `CodeNode` trees should use
    /// [`import_collector::collect_imports`](crate::import_collector::collect_imports)
    /// instead, which handles the full tree walk including nested blocks.
    pub(crate) fn collect_imports(&self, out: &mut Vec<ImportRef>) {
        crate::type_name_import::collect_imports(self, out)
    }

    /// Render this type name to a `pretty::BoxDoc` for width-aware formatting.
    ///
    /// The `resolved_name` closure maps (module, name) -> display name,
    /// accounting for import aliases.
    pub fn to_doc<F>(&self, resolve: &F) -> BoxDoc<'static, ()>
    where
        F: Fn(&str, &str) -> String,
    {
        crate::type_name_render::to_doc(self, resolve)
    }

    /// Render this type to a string at a given width.
    pub fn render<F>(
        &self,
        width: usize,
        resolve: &F,
    ) -> Result<String, crate::error::SigilStitchError>
    where
        F: Fn(&str, &str) -> String,
    {
        crate::type_name_render::render(self, width, resolve)
    }

    /// Language-aware variant of [`TypeName::to_doc`] that consults the lang for
    /// syntax differences (e.g., generic delimiters `<>` vs `[]`).
    pub fn to_doc_with_lang<F>(&self, resolve: &F, lang: &dyn RendererLang) -> BoxDoc<'static, ()>
    where
        F: Fn(&str, &str) -> String,
    {
        crate::type_name_render::to_doc_with_lang(self, resolve, lang)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang::typescript::TypeScript;
    use crate::type_name_render::is_compound_type;

    fn identity_resolve(module: &str, name: &str) -> String {
        let _ = module;
        name.to_string()
    }

    macro_rules! test_lang {
        ($name:ident, $base:ty { $($override:tt)* }) => {
            #[derive(Debug, Clone)]
            struct $name(pub $base);
            impl $name {
                fn new() -> Self { Self(<$base>::new()) }
            }
            impl crate::lang::RendererLang for $name {
                fn file_extension(&self) -> &str { self.0.file_extension() }
                fn reserved_words(&self) -> &[&str] { self.0.reserved_words() }
                fn render_string_literal(&self, s: &str) -> String { self.0.render_string_literal(s) }
                fn line_comment_prefix(&self) -> &str { self.0.line_comment_prefix() }
                fn type_presentation(&self) -> crate::lang::config::TypePresentationConfig<'_> { self.0.type_presentation() }
                fn block_syntax(&self) -> crate::lang::config::BlockSyntaxConfig<'_> { self.0.block_syntax() }
                $($override)*
            }
            impl crate::lang::CodeLang for $name {
                fn render_imports(&self, imports: &crate::import::ImportGroup) -> String { self.0.render_imports(imports) }
                fn render_doc_comment(&self, lines: &[&str]) -> String { self.0.render_doc_comment(lines) }
                fn render_visibility(&self, vis: crate::spec::modifiers::Visibility, ctx: crate::spec::modifiers::DeclarationContext) -> &str { self.0.render_visibility(vis, ctx) }
                fn function_keyword(&self, ctx: crate::spec::modifiers::DeclarationContext) -> &str { self.0.function_keyword(ctx) }
                fn type_keyword(&self, kind: crate::spec::modifiers::TypeKind) -> &str { self.0.type_keyword(kind) }
                fn methods_inside_type_body(&self, kind: crate::spec::modifiers::TypeKind) -> bool { self.0.methods_inside_type_body(kind) }
                fn function_syntax(&self) -> crate::lang::config::FunctionSyntaxConfig<'_> { self.0.function_syntax() }
                fn type_decl_syntax(&self) -> crate::lang::config::TypeDeclSyntaxConfig<'_> { self.0.type_decl_syntax() }
                fn enum_and_annotation(&self) -> crate::lang::config::EnumAndAnnotationConfig<'_> { self.0.enum_and_annotation() }
            }
        };
    }

    test_lang!(PrefixLang, TypeScript {
        fn generic_syntax(&self) -> crate::lang::config::GenericSyntaxConfig<'_> {
            crate::lang::config::GenericSyntaxConfig {
                application_style: GenericApplicationStyle::PrefixJuxtaposition,
                ..self.0.generic_syntax()
            }
        }
    });

    test_lang!(PostfixLang, TypeScript {
        fn generic_syntax(&self) -> crate::lang::config::GenericSyntaxConfig<'_> {
            crate::lang::config::GenericSyntaxConfig {
                application_style: GenericApplicationStyle::PostfixJuxtaposition,
                ..self.0.generic_syntax()
            }
        }
    });

    #[test]
    fn test_primitive() {
        let t = TypeName::primitive("number");
        assert_eq!(t.render(80, &identity_resolve).unwrap(), "number");
    }

    #[test]
    fn test_importable() {
        let t = TypeName::importable("./models", "User");
        assert_eq!(t.render(80, &identity_resolve).unwrap(), "User");
    }

    #[test]
    fn test_importable_with_alias() {
        let t = TypeName::importable("./other", "User");
        let resolve = |module: &str, name: &str| {
            if module == "./other" && name == "User" {
                "OtherUser".to_string()
            } else {
                name.to_string()
            }
        };
        assert_eq!(t.render(80, &resolve).unwrap(), "OtherUser");
    }

    #[test]
    fn test_array() {
        let t = TypeName::array(TypeName::primitive("string"));
        assert_eq!(t.render(80, &identity_resolve).unwrap(), "string[]");
    }

    #[test]
    fn test_generic() {
        let t = TypeName::generic(
            TypeName::primitive("Promise"),
            vec![TypeName::importable("./models", "User")],
        );
        assert_eq!(t.render(80, &identity_resolve).unwrap(), "Promise<User>");
    }

    #[test]
    fn test_generic_multiline() {
        let t = TypeName::generic(
            TypeName::primitive("Map"),
            vec![
                TypeName::primitive("VeryLongKeyTypeName"),
                TypeName::primitive("VeryLongValueTypeName"),
            ],
        );
        // At width 20, should break
        let output = t.render(20, &identity_resolve).unwrap();
        assert!(output.contains('\n'));
        assert!(output.contains("VeryLongKeyTypeName"));
        assert!(output.contains("VeryLongValueTypeName"));
    }

    #[test]
    fn test_union() {
        let t = TypeName::union(vec![
            TypeName::primitive("string"),
            TypeName::primitive("number"),
            TypeName::primitive("boolean"),
        ]);
        assert_eq!(
            t.render(80, &identity_resolve).unwrap(),
            "string | number | boolean"
        );
    }

    #[test]
    fn test_union_multiline() {
        let t = TypeName::union(vec![
            TypeName::primitive("VeryLongTypeName1"),
            TypeName::primitive("VeryLongTypeName2"),
            TypeName::primitive("VeryLongTypeName3"),
        ]);
        let output = t.render(30, &identity_resolve).unwrap();
        assert!(output.contains('\n'));
    }

    #[test]
    fn test_pointer() {
        let t = TypeName::pointer(TypeName::primitive("User"));
        assert_eq!(t.render(80, &identity_resolve).unwrap(), "*User");
    }

    #[test]
    fn test_slice() {
        let t = TypeName::slice(TypeName::primitive("User"));
        assert_eq!(t.render(80, &identity_resolve).unwrap(), "[]User");
    }

    #[test]
    fn test_map() {
        let t = TypeName::map(TypeName::primitive("string"), TypeName::primitive("User"));
        assert_eq!(t.render(80, &identity_resolve).unwrap(), "map[string]User");
    }

    #[test]
    fn test_optional() {
        let t = TypeName::optional(TypeName::primitive("string"));
        assert_eq!(t.render(80, &identity_resolve).unwrap(), "string | null");
    }

    #[test]
    fn test_function_type() {
        let t = TypeName::function(
            vec![TypeName::primitive("string"), TypeName::primitive("number")],
            TypeName::primitive("boolean"),
        );
        assert_eq!(
            t.render(80, &identity_resolve).unwrap(),
            "(string, number) => boolean"
        );
    }

    #[test]
    fn test_deeply_nested() {
        let inner = TypeName::generic(
            TypeName::primitive("Array"),
            vec![TypeName::importable("./models", "User")],
        );
        let outer = TypeName::generic(TypeName::primitive("Promise"), vec![inner]);
        assert_eq!(
            outer.render(80, &identity_resolve).unwrap(),
            "Promise<Array<User>>"
        );
    }

    #[test]
    fn test_collect_imports() {
        let t = TypeName::generic(
            TypeName::importable("./base", "Base"),
            vec![
                TypeName::importable("./models", "User"),
                TypeName::array(TypeName::importable("./models", "Tag")),
            ],
        );
        let mut imports = Vec::new();
        t.collect_imports(&mut imports);
        assert_eq!(imports.len(), 3);
        assert_eq!(imports[0].name, "Base");
        assert_eq!(imports[1].name, "User");
        assert_eq!(imports[2].name, "Tag");
    }

    #[test]
    fn test_raw_no_imports() {
        let t = TypeName::raw("any");
        let mut imports = Vec::new();
        t.collect_imports(&mut imports);
        assert!(imports.is_empty());
        assert_eq!(t.render(80, &identity_resolve).unwrap(), "any");
    }

    #[test]
    fn test_with_alias_on_importable() {
        let t = TypeName::importable("./models", "User").with_alias("MyUser");
        // Verify the alias is stored correctly.
        if let TypeName::Importable { alias, .. } = &t {
            assert_eq!(alias.as_deref(), Some("MyUser"));
        } else {
            panic!("Expected Importable variant");
        }
    }

    #[test]
    fn test_with_alias_propagates_to_import_ref() {
        let t = TypeName::importable("./models", "User").with_alias("MyUser");
        let mut imports = Vec::new();
        t.collect_imports(&mut imports);
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].name, "User");
        assert_eq!(imports[0].alias.as_deref(), Some("MyUser"));
    }

    #[test]
    fn test_with_alias_noop_on_primitive() {
        // with_alias on a non-Importable variant should be a no-op.
        let t = TypeName::primitive("number").with_alias("MyNumber");
        assert_eq!(t.render(80, &identity_resolve).unwrap(), "number");
    }

    #[test]
    fn test_with_alias_renders_alias_name() {
        let t = TypeName::importable("./models", "User").with_alias("MyUser");
        // The resolve function should map to the alias.
        let resolve = |_module: &str, _name: &str| "MyUser".to_string();
        assert_eq!(t.render(80, &resolve).unwrap(), "MyUser");
    }

    #[test]
    fn test_tuple() {
        let t = TypeName::tuple(vec![
            TypeName::primitive("string"),
            TypeName::primitive("number"),
        ]);
        assert_eq!(t.render(80, &identity_resolve).unwrap(), "(string, number)");
    }

    #[test]
    fn test_unit_tuple() {
        let t = TypeName::unit();
        assert_eq!(t.render(80, &identity_resolve).unwrap(), "()");
    }

    #[test]
    fn test_tuple_collect_imports() {
        let t = TypeName::tuple(vec![
            TypeName::importable("./models", "User"),
            TypeName::importable("./models", "Tag"),
        ]);
        let mut imports = Vec::new();
        t.collect_imports(&mut imports);
        assert_eq!(imports.len(), 2);
        assert_eq!(imports[0].name, "User");
        assert_eq!(imports[1].name, "Tag");
    }

    #[test]
    fn test_tuple_with_lang_ts() {
        let lang = TypeScript::new();
        let t = TypeName::tuple(vec![
            TypeName::primitive("string"),
            TypeName::primitive("number"),
        ]);
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "[string, number]");
    }

    #[test]
    fn test_tuple_with_lang_rust() {
        use crate::lang::rust_lang::RustLang;
        let lang = RustLang::new();
        let t = TypeName::tuple(vec![
            TypeName::primitive("String"),
            TypeName::primitive("i32"),
        ]);
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "(String, i32)");
    }

    #[test]
    fn test_tuple_with_lang_python() {
        use crate::lang::python::Python;
        let lang = Python::new();
        let t = TypeName::tuple(vec![TypeName::primitive("str"), TypeName::primitive("int")]);
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "tuple[str, int]");
    }

    #[test]
    fn test_tuple_with_lang_cpp() {
        use crate::lang::cpp_lang::CppLang;
        let lang = CppLang::new();
        let t = TypeName::tuple(vec![
            TypeName::primitive("int"),
            TypeName::primitive("std::string"),
        ]);
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(
            String::from_utf8(buf).unwrap(),
            "std::tuple<int, std::string>"
        );
    }

    #[test]
    fn test_unit_tuple_with_lang_rust() {
        use crate::lang::rust_lang::RustLang;
        let lang = RustLang::new();
        let t = TypeName::unit();
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "()");
    }

    #[test]
    fn test_reference() {
        let t = TypeName::reference(TypeName::primitive("str"));
        assert_eq!(t.render(80, &identity_resolve).unwrap(), "&str");
    }

    #[test]
    fn test_reference_mut() {
        let t = TypeName::reference_mut(TypeName::primitive("Vec<i32>"));
        assert_eq!(t.render(80, &identity_resolve).unwrap(), "&mut Vec<i32>");
    }

    #[test]
    fn test_reference_collect_imports() {
        let t = TypeName::reference(TypeName::importable("./models", "User"));
        let mut imports = Vec::new();
        t.collect_imports(&mut imports);
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].name, "User");
    }

    #[test]
    fn test_reference_with_lang_rust() {
        use crate::lang::rust_lang::RustLang;
        let lang = RustLang::new();
        let t = TypeName::reference(TypeName::primitive("String"));
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "&String");
    }

    #[test]
    fn test_reference_mut_with_lang_rust() {
        use crate::lang::rust_lang::RustLang;
        let lang = RustLang::new();
        let t = TypeName::reference_mut(TypeName::primitive("String"));
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "&mut String");
    }

    #[test]
    fn test_reference_with_lang_cpp() {
        use crate::lang::cpp_lang::CppLang;
        let lang = CppLang::new();
        let t = TypeName::reference(TypeName::primitive("std::string"));
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "const std::string&");
    }

    #[test]
    fn test_reference_mut_with_lang_cpp() {
        use crate::lang::cpp_lang::CppLang;
        let lang = CppLang::new();
        let t = TypeName::reference_mut(TypeName::primitive("std::string"));
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "std::string&");
    }

    #[test]
    fn test_reference_with_lang_ts() {
        let lang = TypeScript::new();
        let t = TypeName::reference(TypeName::primitive("string"));
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "string");
    }

    #[test]
    fn test_reference_mut_with_lang_go() {
        use crate::lang::go_lang::GoLang;
        let lang = GoLang::new();
        let t = TypeName::reference_mut(TypeName::primitive("int"));
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "*int");
    }

    #[test]
    fn test_reference_with_lang_go() {
        use crate::lang::go_lang::GoLang;
        let lang = GoLang::new();
        let t = TypeName::reference(TypeName::primitive("int"));
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "int");
    }

    #[test]
    fn test_reference_with_lang_c() {
        use crate::lang::c_lang::CLang;
        let lang = CLang::new();
        let t = TypeName::reference(TypeName::primitive("int"));
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "const int*");
    }

    #[test]
    fn test_reference_mut_with_lang_c() {
        use crate::lang::c_lang::CLang;
        let lang = CLang::new();
        let t = TypeName::reference_mut(TypeName::primitive("int"));
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "int*");
    }

    #[test]
    fn test_generic_prefix_juxtaposition() {
        let lang = PrefixLang::new();
        let t = TypeName::generic(
            TypeName::primitive("Maybe"),
            vec![TypeName::primitive("Int")],
        );
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "Maybe Int");
    }

    #[test]
    fn test_generic_prefix_juxtaposition_compound_param() {
        let lang = PrefixLang::new();
        let inner = TypeName::generic(
            TypeName::primitive("Maybe"),
            vec![TypeName::primitive("Int")],
        );
        let t = TypeName::generic(
            TypeName::primitive("Either"),
            vec![TypeName::primitive("String"), inner],
        );
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "Either String (Maybe Int)");
    }

    #[test]
    fn test_generic_postfix_juxtaposition_single() {
        let lang = PostfixLang::new();
        let t = TypeName::generic(
            TypeName::primitive("option"),
            vec![TypeName::primitive("int")],
        );
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "int option");
    }

    #[test]
    fn test_generic_postfix_juxtaposition_multi() {
        let lang = PostfixLang::new();
        let t = TypeName::generic(
            TypeName::primitive("result"),
            vec![TypeName::primitive("int"), TypeName::primitive("string")],
        );
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "(int, string) result");
    }

    #[test]
    fn test_is_compound_type() {
        assert!(is_compound_type(&TypeName::generic(
            TypeName::primitive("A"),
            vec![TypeName::primitive("B")],
        )));
        assert!(is_compound_type(&TypeName::union(vec![
            TypeName::primitive("A"),
            TypeName::primitive("B"),
        ])));
        assert!(is_compound_type(&TypeName::intersection(vec![
            TypeName::primitive("A"),
            TypeName::primitive("B"),
        ])));
        assert!(is_compound_type(&TypeName::function(
            vec![TypeName::primitive("A")],
            TypeName::primitive("B"),
        )));
        assert!(is_compound_type(&TypeName::tuple(vec![
            TypeName::primitive("A"),
            TypeName::primitive("B"),
        ])));
        assert!(!is_compound_type(&TypeName::primitive("Int")));
        assert!(!is_compound_type(&TypeName::array(TypeName::primitive(
            "Int"
        ),)));
    }

    // --- Feature 07: Associated Types ---

    #[test]
    fn test_associated_type_rust_qualified() {
        use crate::lang::rust_lang::RustLang;
        let lang = RustLang::new();
        let t = TypeName::associated_type(
            TypeName::primitive("T"),
            Some(TypeName::primitive("Iterator")),
            "Item",
        );
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "<T as Iterator>::Item");
    }

    #[test]
    fn test_associated_type_rust_simple() {
        use crate::lang::rust_lang::RustLang;
        let lang = RustLang::new();
        let t = TypeName::member_type(TypeName::primitive("Self"), "Output");
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "Self::Output");
    }

    #[test]
    fn test_associated_type_ts_index_access() {
        let lang = TypeScript::new();
        let t = TypeName::associated_type(
            TypeName::primitive("T"),
            Some(TypeName::primitive("Qual")),
            "key",
        );
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "T[\"key\"]");
    }

    #[test]
    fn test_associated_type_java_dot() {
        use crate::lang::java_lang::JavaLang;
        let lang = JavaLang::new();
        let t = TypeName::member_type(TypeName::primitive("Map"), "Entry");
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "Map.Entry");
    }

    #[test]
    fn test_associated_type_collect_imports() {
        let t = TypeName::associated_type(
            TypeName::importable("./models", "User"),
            Some(TypeName::importable("./traits", "Serializable")),
            "Output",
        );
        let mut imports = Vec::new();
        t.collect_imports(&mut imports);
        assert_eq!(imports.len(), 2);
    }

    // --- Feature 08: Existential Types ---

    #[test]
    fn test_impl_trait_rust() {
        use crate::lang::rust_lang::RustLang;
        let lang = RustLang::new();
        let t = TypeName::impl_trait(vec![
            TypeName::primitive("Display"),
            TypeName::primitive("Debug"),
        ]);
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "impl Display + Debug");
    }

    #[test]
    fn test_dyn_trait_rust() {
        use crate::lang::rust_lang::RustLang;
        let lang = RustLang::new();
        let t = TypeName::dyn_trait(vec![TypeName::primitive("Error")]);
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "dyn Error");
    }

    #[test]
    fn test_impl_trait_ts_intersection() {
        let lang = TypeScript::new();
        let t = TypeName::impl_trait(vec![
            TypeName::primitive("Serializable"),
            TypeName::primitive("Loggable"),
        ]);
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "Serializable & Loggable");
    }

    #[test]
    fn test_wildcard_java_unbounded() {
        use crate::lang::java_lang::JavaLang;
        let lang = JavaLang::new();
        let t = TypeName::wildcard();
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "?");
    }

    #[test]
    fn test_wildcard_java_extends() {
        use crate::lang::java_lang::JavaLang;
        let lang = JavaLang::new();
        let t = TypeName::wildcard_extends(TypeName::primitive("Comparable"));
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "? extends Comparable");
    }

    #[test]
    fn test_wildcard_java_super() {
        use crate::lang::java_lang::JavaLang;
        let lang = JavaLang::new();
        let t = TypeName::wildcard_super(TypeName::primitive("Number"));
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "? super Number");
    }

    #[test]
    fn test_wildcard_kotlin() {
        use crate::lang::kotlin::Kotlin;
        let lang = Kotlin::new();
        let t = TypeName::wildcard();
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "*");
    }

    #[test]
    fn test_wildcard_kotlin_out() {
        use crate::lang::kotlin::Kotlin;
        let lang = Kotlin::new();
        let t = TypeName::wildcard_extends(TypeName::primitive("Number"));
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "out Number");
    }

    #[test]
    fn test_wildcard_kotlin_in() {
        use crate::lang::kotlin::Kotlin;
        let lang = Kotlin::new();
        let t = TypeName::wildcard_super(TypeName::primitive("Number"));
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "in Number");
    }

    #[test]
    fn test_wildcard_go() {
        use crate::lang::go_lang::GoLang;
        let lang = GoLang::new();
        let t = TypeName::wildcard();
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "any");
    }

    #[test]
    fn test_wildcard_rust() {
        use crate::lang::rust_lang::RustLang;
        let lang = RustLang::new();
        let t = TypeName::wildcard();
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "_");
    }

    #[test]
    fn test_impl_trait_collect_imports() {
        let t = TypeName::impl_trait(vec![
            TypeName::importable("./traits", "Serializable"),
            TypeName::primitive("Debug"),
        ]);
        let mut imports = Vec::new();
        t.collect_imports(&mut imports);
        assert_eq!(imports.len(), 1);
    }

    #[test]
    fn test_dyn_trait_collect_imports() {
        let t = TypeName::dyn_trait(vec![TypeName::importable("./errors", "AppError")]);
        let mut imports = Vec::new();
        t.collect_imports(&mut imports);
        assert_eq!(imports.len(), 1);
    }

    #[test]
    fn test_wildcard_collect_imports() {
        let t = TypeName::wildcard_extends(TypeName::importable("./models", "User"));
        let mut imports = Vec::new();
        t.collect_imports(&mut imports);
        assert_eq!(imports.len(), 1);
    }

    #[test]
    fn test_associated_type_default_rendering() {
        let t = TypeName::associated_type(
            TypeName::primitive("T"),
            Some(TypeName::primitive("Iter")),
            "Item",
        );
        assert_eq!(
            t.render(80, &identity_resolve).unwrap(),
            "<T as Iter>::Item"
        );
    }

    #[test]
    fn test_member_type_default_rendering() {
        let t = TypeName::member_type(TypeName::primitive("Self"), "Output");
        assert_eq!(t.render(80, &identity_resolve).unwrap(), "Self::Output");
    }

    #[test]
    fn test_impl_trait_default_rendering() {
        let t = TypeName::impl_trait(vec![TypeName::primitive("Display")]);
        assert_eq!(t.render(80, &identity_resolve).unwrap(), "impl Display");
    }

    #[test]
    fn test_dyn_trait_default_rendering() {
        let t = TypeName::dyn_trait(vec![
            TypeName::primitive("Error"),
            TypeName::primitive("Send"),
        ]);
        assert_eq!(t.render(80, &identity_resolve).unwrap(), "dyn Error + Send");
    }

    #[test]
    fn test_wildcard_default_rendering() {
        assert_eq!(
            TypeName::wildcard().render(80, &identity_resolve).unwrap(),
            "?"
        );
        assert_eq!(
            TypeName::wildcard_extends(TypeName::primitive("T"))
                .render(80, &identity_resolve)
                .unwrap(),
            "? extends T"
        );
        assert_eq!(
            TypeName::wildcard_super(TypeName::primitive("T"))
                .render(80, &identity_resolve)
                .unwrap(),
            "? super T"
        );
    }

    // --- Feature 10: Lifetime Parameters ---

    #[test]
    fn test_reference_with_lifetime_rust() {
        use crate::lang::rust_lang::RustLang;
        let lang = RustLang::new();
        let t = TypeName::reference_with_lifetime(TypeName::primitive("str"), "'a");
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "&'a str");
    }

    #[test]
    fn test_reference_mut_with_lifetime_rust() {
        use crate::lang::rust_lang::RustLang;
        let lang = RustLang::new();
        let t = TypeName::reference_mut_with_lifetime(TypeName::primitive("String"), "'a");
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "&'a mut String");
    }

    #[test]
    fn test_reference_with_lifetime_default_rendering() {
        let t = TypeName::reference_with_lifetime(TypeName::primitive("str"), "'a");
        assert_eq!(t.render(80, &identity_resolve).unwrap(), "&'a str");
    }

    #[test]
    fn test_reference_without_lifetime_unchanged() {
        use crate::lang::rust_lang::RustLang;
        let lang = RustLang::new();
        let t = TypeName::reference(TypeName::primitive("String"));
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "&String");
    }

    #[test]
    fn test_qualified_renders_with_rust_separator() {
        use crate::lang::rust_lang::RustLang;
        let lang = RustLang::new();
        let t = TypeName::qualified("serde_json", "Value");
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "serde_json::Value");
    }

    #[test]
    fn test_qualified_renders_with_go_separator() {
        use crate::lang::go_lang::GoLang;
        let lang = GoLang::new();
        let t = TypeName::qualified("net/http", "Server");
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "net/http.Server");
    }

    #[test]
    fn test_qualified_skips_import_collection() {
        let t = TypeName::qualified("serde_json", "Value");
        let mut imports = Vec::new();
        t.collect_imports(&mut imports);
        assert!(imports.is_empty());
    }

    #[test]
    fn test_qualify_modifier() {
        let t = TypeName::importable("serde_json", "Value").qualify();
        let mut imports = Vec::new();
        t.collect_imports(&mut imports);
        assert!(imports.is_empty());

        use crate::lang::rust_lang::RustLang;
        let lang = RustLang::new();
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "serde_json::Value");
    }

    #[test]
    fn test_qualified_in_generic() {
        use crate::lang::rust_lang::RustLang;
        let lang = RustLang::new();
        let t = TypeName::generic(
            TypeName::qualified("std::collections", "HashMap"),
            vec![
                TypeName::primitive("String"),
                TypeName::qualified("serde_json", "Value"),
            ],
        );
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(
            String::from_utf8(buf).unwrap(),
            "std::collections::HashMap<String, serde_json::Value>"
        );
    }

    #[test]
    fn test_qualified_fallback_unsupported_lang() {
        let lang = TypeScript::new();
        let t = TypeName::qualified("serde_json", "Value");
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "Value");
    }
}
