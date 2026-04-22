use pretty::BoxDoc;

use crate::import::ImportRef;
use crate::lang::CodeLang;

/// Syntactic pattern for rendering a compound type construct.
///
/// Each variant describes a structural pattern for assembling already-rendered
/// inner type docs into the output. The rendering engine in `type_name.rs`
/// interprets these patterns — language implementations never build `BoxDoc`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypePresentation<'a> {
    /// `name<P1, P2>` — delimiters from `generic_open()`/`generic_close()`.
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
    /// `Base<P1, P2>` or `Base[P1, P2]` — uses `generic_open()`/`generic_close()`.
    Delimited,
    /// `Base P1 P2` — Haskell-style prefix juxtaposition.
    /// Compound params are parenthesized: `Either String (Maybe Int)`.
    PrefixJuxtaposition,
    /// `P1 Base` (single) or `(P1, P2) Base` (multi) — OCaml-style postfix.
    PostfixJuxtaposition,
}

/// Syntactic pattern for rendering a function type expression.
#[derive(Debug, Clone, PartialEq, Eq)]
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

/// How `TypeName::AssociatedType` renders across languages.
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundsPresentation<'a> {
    /// Keyword prefix (e.g., `"impl "`, `"dyn "`).
    pub keyword: &'a str,
    /// Separator between bounds (e.g., `" + "`).
    pub separator: &'a str,
}

/// How wildcard types render.
#[derive(Debug, Clone, PartialEq, Eq)]
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
/// let user = TypeName::<TypeScript>::importable("./models", "User");
///
/// // Primitive (no import needed):
/// let num = TypeName::<TypeScript>::primitive("number");
///
/// // Generic: Promise<User>
/// let promise = TypeName::<TypeScript>::generic(
///     TypeName::primitive("Promise"),
///     vec![user],
/// );
///
/// // Optional: string | null
/// let maybe_str = TypeName::<TypeScript>::optional(TypeName::primitive("string"));
/// ```
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(bound = "")]
pub enum TypeName<L: CodeLang> {
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
    },
    /// A primitive/built-in type (no import needed).
    Primitive(String),
    /// Array type. TS: `T[]`, Go: `[]T`, Rust: `Vec<T>`.
    Array(Box<TypeName<L>>),
    /// Readonly array type. TS: `readonly T[]`. Other languages fall back to
    /// their `Array` rendering since they lack a direct readonly-array form.
    ReadonlyArray(Box<TypeName<L>>),
    /// Generic type. e.g., `Promise<User>`, `HashMap<String, User>`.
    Generic {
        /// The base type (e.g., `Promise`, `HashMap`).
        base: Box<TypeName<L>>,
        /// The type parameters.
        params: Vec<TypeName<L>>,
    },
    /// Union type. TS: `A | B | C`.
    Union(Vec<TypeName<L>>),
    /// Intersection type. TS: `A & B`.
    Intersection(Vec<TypeName<L>>),
    /// Pointer type. Go: `*T`.
    Pointer(Box<TypeName<L>>),
    /// Slice type. Go: `[]T`.
    Slice(Box<TypeName<L>>),
    /// Map type. Go: `map[K]V`, TS: `Record<K, V>`.
    Map {
        /// The key type.
        key: Box<TypeName<L>>,
        /// The value type.
        value: Box<TypeName<L>>,
    },
    /// Optional type. TS: `T | null`, Rust: `Option<T>`.
    Optional(Box<TypeName<L>>),
    /// Tuple type. Rust: `(A, B)`, TS: `[A, B]`, Python: `tuple[A, B]`.
    Tuple(Vec<TypeName<L>>),
    /// Reference type. Rust: `&T` / `&mut T` / `&'a T`, C++: `const T&` / `T&`.
    Reference {
        /// The referenced type.
        inner: Box<TypeName<L>>,
        /// Whether the reference is mutable.
        mutable: bool,
        /// Optional lifetime (Rust only, e.g., `'a`).
        #[serde(default)]
        lifetime: Option<String>,
    },
    /// Associated/path-dependent type. Rust: `<T as Iterator>::Item`, TS: `T["key"]`.
    AssociatedType {
        /// The base type (e.g., `T`, `Vec<i32>`).
        base: Box<TypeName<L>>,
        /// Optional qualifier trait (Rust: `Iterator` in `<T as Iterator>::Item`).
        qualifier: Option<Box<TypeName<L>>>,
        /// The projected member name (e.g., `"Item"`, `"key"`).
        member: String,
    },
    /// `impl Trait` bounds. Rust: `impl Display + Debug`.
    ImplTrait {
        /// The trait bounds.
        bounds: Vec<TypeName<L>>,
    },
    /// `dyn Trait` bounds. Rust: `dyn Error + Send`.
    DynTrait {
        /// The trait bounds.
        bounds: Vec<TypeName<L>>,
    },
    /// Wildcard type. Java: `?`, `? extends T`, `? super T`. Kotlin: `*`, `out T`, `in T`.
    Wildcard {
        /// Upper bound (Java: `? extends T`, Kotlin: `out T`).
        upper_bound: Option<Box<TypeName<L>>>,
        /// Lower bound (Java: `? super T`, Kotlin: `in T`).
        lower_bound: Option<Box<TypeName<L>>>,
    },
    /// Function type. TS: `(a: A, b: B) => R`.
    Function {
        /// The parameter types.
        params: Vec<TypeName<L>>,
        /// The return type.
        return_type: Box<TypeName<L>>,
    },
    /// Raw string escape hatch. No import tracking.
    Raw(String),

    /// Phantom data to carry L without requiring it in all variants.
    #[doc(hidden)]
    _Phantom(std::marker::PhantomData<L>),
}

fn render_presentation(
    pres: &TypePresentation<'_>,
    inner_docs: Vec<BoxDoc<'static, ()>>,
    lang: &impl CodeLang,
) -> BoxDoc<'static, ()> {
    match pres {
        TypePresentation::GenericWrap { name } => {
            let sep = BoxDoc::text(",").append(BoxDoc::softline());
            let params = BoxDoc::intersperse(inner_docs, sep);
            BoxDoc::text(name.to_string())
                .append(BoxDoc::text(lang.generic_open().to_string()))
                .append(params.nest(2).group())
                .append(BoxDoc::text(lang.generic_close().to_string()))
        }
        TypePresentation::Prefix { prefix } => {
            debug_assert_eq!(inner_docs.len(), 1);
            let inner = inner_docs.into_iter().next().unwrap_or_else(BoxDoc::nil);
            BoxDoc::text(prefix.to_string()).append(inner)
        }
        TypePresentation::Postfix { suffix } => {
            debug_assert_eq!(inner_docs.len(), 1);
            let inner = inner_docs.into_iter().next().unwrap_or_else(BoxDoc::nil);
            inner.append(BoxDoc::text(suffix.to_string()))
        }
        TypePresentation::Surround { prefix, suffix } => {
            debug_assert_eq!(inner_docs.len(), 1);
            let inner = inner_docs.into_iter().next().unwrap_or_else(BoxDoc::nil);
            BoxDoc::text(prefix.to_string())
                .append(inner)
                .append(BoxDoc::text(suffix.to_string()))
        }
        TypePresentation::Delimited { open, sep, close } => {
            let separator = BoxDoc::text(sep.to_string());
            let body = BoxDoc::intersperse(inner_docs, separator);
            BoxDoc::text(open.to_string())
                .append(body.nest(2).group())
                .append(BoxDoc::text(close.to_string()))
        }
        TypePresentation::Infix { sep } => {
            let sep_trimmed = sep.trim_start();
            let separator = BoxDoc::softline().append(BoxDoc::text(sep_trimmed.to_string()));
            BoxDoc::intersperse(inner_docs, separator).group()
        }
    }
}

fn render_function_presentation(
    pres: &FunctionPresentation<'_>,
    param_docs: Vec<BoxDoc<'static, ()>>,
    return_doc: BoxDoc<'static, ()>,
) -> BoxDoc<'static, ()> {
    if pres.curried {
        // Haskell style: A -> B -> R
        let mut all = param_docs;
        all.push(return_doc);
        let sep = BoxDoc::text(pres.arrow.to_string());
        return BoxDoc::intersperse(all, sep);
    }

    let sep = BoxDoc::text(pres.params_sep.to_string());
    let params_doc = BoxDoc::intersperse(param_docs, sep);
    let params_block = BoxDoc::text(pres.params_open.to_string())
        .append(params_doc.nest(2).group())
        .append(BoxDoc::text(pres.params_close.to_string()));

    let keyword_doc = if pres.keyword.is_empty() {
        BoxDoc::nil()
    } else {
        BoxDoc::text(pres.keyword.to_string())
    };

    let signature = if pres.return_first {
        // C++/Dart: R keyword(A, B)
        return_doc.append(keyword_doc).append(params_block)
    } else {
        // TS/Rust/Go: keyword(A, B) -> R
        keyword_doc
            .append(params_block)
            .append(BoxDoc::text(pres.arrow.to_string()))
            .append(return_doc)
    };

    if pres.wrapper_open.is_empty() {
        signature
    } else {
        BoxDoc::text(pres.wrapper_open.to_string())
            .append(signature)
            .append(BoxDoc::text(pres.wrapper_close.to_string()))
    }
}

fn is_compound_type<L: CodeLang>(t: &TypeName<L>) -> bool {
    matches!(
        t,
        TypeName::Generic { .. }
            | TypeName::Union(_)
            | TypeName::Intersection(_)
            | TypeName::Function { .. }
            | TypeName::Tuple(_)
    )
}

impl<L: CodeLang> TypeName<L> {
    /// Create an importable type name.
    pub fn importable(module: &str, name: &str) -> Self {
        TypeName::Importable {
            module: module.to_string(),
            name: name.to_string(),
            is_type_only: false,
            alias: None,
        }
    }

    /// Create a type-only importable type name (TypeScript `import type`).
    pub fn importable_type(module: &str, name: &str) -> Self {
        TypeName::Importable {
            module: module.to_string(),
            name: name.to_string(),
            is_type_only: true,
            alias: None,
        }
    }

    /// Create a primitive type name (no import).
    pub fn primitive(name: &str) -> Self {
        TypeName::Primitive(name.to_string())
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

    /// Create an array type.
    pub fn array(inner: TypeName<L>) -> Self {
        TypeName::Array(Box::new(inner))
    }

    /// Create a readonly array type (TypeScript: `readonly T[]`).
    ///
    /// In languages without a direct readonly-array form (Go, most C-family),
    /// this renders identically to [`TypeName::array`]. Use this variant only
    /// when the readonly distinction carries information the output should
    /// preserve (e.g. TypeScript interface fields).
    pub fn readonly_array(inner: TypeName<L>) -> Self {
        TypeName::ReadonlyArray(Box::new(inner))
    }

    /// Create a generic type (e.g., `Promise<User>`).
    pub fn generic(base: TypeName<L>, params: Vec<TypeName<L>>) -> Self {
        TypeName::Generic {
            base: Box::new(base),
            params,
        }
    }

    /// Create a union type (e.g., `A | B | C`).
    pub fn union(members: Vec<TypeName<L>>) -> Self {
        TypeName::Union(members)
    }

    /// Create an intersection type (e.g., `A & B`).
    pub fn intersection(members: Vec<TypeName<L>>) -> Self {
        TypeName::Intersection(members)
    }

    /// Create a pointer type (e.g., Go `*T`).
    pub fn pointer(inner: TypeName<L>) -> Self {
        TypeName::Pointer(Box::new(inner))
    }

    /// Create a slice type (e.g., Go `[]T`).
    pub fn slice(inner: TypeName<L>) -> Self {
        TypeName::Slice(Box::new(inner))
    }

    /// Create a map type (e.g., `map[K]V`).
    pub fn map(key: TypeName<L>, value: TypeName<L>) -> Self {
        TypeName::Map {
            key: Box::new(key),
            value: Box::new(value),
        }
    }

    /// Create an optional type.
    pub fn optional(inner: TypeName<L>) -> Self {
        TypeName::Optional(Box::new(inner))
    }

    /// Create a tuple type (e.g., Rust `(A, B)`, TS `[A, B]`).
    pub fn tuple(elements: Vec<TypeName<L>>) -> Self {
        TypeName::Tuple(elements)
    }

    /// Create a unit type (empty tuple: Rust `()`).
    pub fn unit() -> Self {
        TypeName::Tuple(Vec::new())
    }

    /// Create a shared reference type (Rust `&T`, C++ `const T&`).
    pub fn reference(inner: TypeName<L>) -> Self {
        TypeName::Reference {
            inner: Box::new(inner),
            mutable: false,
            lifetime: None,
        }
    }

    /// Create a mutable reference type (Rust `&mut T`, C++ `T&`).
    pub fn reference_mut(inner: TypeName<L>) -> Self {
        TypeName::Reference {
            inner: Box::new(inner),
            mutable: true,
            lifetime: None,
        }
    }

    /// Create a shared reference with a lifetime (Rust `&'a T`).
    pub fn reference_with_lifetime(inner: TypeName<L>, lifetime: &str) -> Self {
        TypeName::Reference {
            inner: Box::new(inner),
            mutable: false,
            lifetime: Some(lifetime.to_string()),
        }
    }

    /// Create a mutable reference with a lifetime (Rust `&'a mut T`).
    pub fn reference_mut_with_lifetime(inner: TypeName<L>, lifetime: &str) -> Self {
        TypeName::Reference {
            inner: Box::new(inner),
            mutable: true,
            lifetime: Some(lifetime.to_string()),
        }
    }

    /// Create a function type.
    pub fn function(params: Vec<TypeName<L>>, return_type: TypeName<L>) -> Self {
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
    pub fn associated_type(
        base: TypeName<L>,
        qualifier: Option<TypeName<L>>,
        member: &str,
    ) -> Self {
        TypeName::AssociatedType {
            base: Box::new(base),
            qualifier: qualifier.map(Box::new),
            member: member.to_string(),
        }
    }

    /// Create a simple member type (no qualifier).
    ///
    /// Rust: `base::member` (e.g., `Self::Output`).
    pub fn member_type(base: TypeName<L>, member: &str) -> Self {
        TypeName::AssociatedType {
            base: Box::new(base),
            qualifier: None,
            member: member.to_string(),
        }
    }

    /// Create an `impl Trait` type (Rust: `impl Display + Debug`).
    pub fn impl_trait(bounds: Vec<TypeName<L>>) -> Self {
        TypeName::ImplTrait { bounds }
    }

    /// Create a `dyn Trait` type (Rust: `dyn Error + Send`).
    pub fn dyn_trait(bounds: Vec<TypeName<L>>) -> Self {
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
    pub fn wildcard_extends(bound: TypeName<L>) -> Self {
        TypeName::Wildcard {
            upper_bound: Some(Box::new(bound)),
            lower_bound: None,
        }
    }

    /// Create a wildcard with a lower bound (Java: `? super T`, Kotlin: `in T`).
    pub fn wildcard_super(bound: TypeName<L>) -> Self {
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
    pub fn collect_imports(&self, out: &mut Vec<ImportRef>) {
        match self {
            TypeName::Importable {
                module,
                name,
                is_type_only,
                alias,
            } => {
                out.push(ImportRef {
                    module: module.clone(),
                    name: name.clone(),
                    is_type_only: *is_type_only,
                    alias: alias.clone(),
                });
            }
            TypeName::Array(inner)
            | TypeName::ReadonlyArray(inner)
            | TypeName::Pointer(inner)
            | TypeName::Slice(inner)
            | TypeName::Optional(inner) => {
                inner.collect_imports(out);
            }
            TypeName::Reference { inner, .. } => {
                inner.collect_imports(out);
            }
            TypeName::Generic { base, params } => {
                base.collect_imports(out);
                for p in params {
                    p.collect_imports(out);
                }
            }
            TypeName::Union(members)
            | TypeName::Intersection(members)
            | TypeName::Tuple(members) => {
                for m in members {
                    m.collect_imports(out);
                }
            }
            TypeName::Map { key, value } => {
                key.collect_imports(out);
                value.collect_imports(out);
            }
            TypeName::Function {
                params,
                return_type,
            } => {
                for p in params {
                    p.collect_imports(out);
                }
                return_type.collect_imports(out);
            }
            TypeName::AssociatedType {
                base, qualifier, ..
            } => {
                base.collect_imports(out);
                if let Some(q) = qualifier {
                    q.collect_imports(out);
                }
            }
            TypeName::ImplTrait { bounds } | TypeName::DynTrait { bounds } => {
                for b in bounds {
                    b.collect_imports(out);
                }
            }
            TypeName::Wildcard {
                upper_bound,
                lower_bound,
            } => {
                if let Some(ub) = upper_bound {
                    ub.collect_imports(out);
                }
                if let Some(lb) = lower_bound {
                    lb.collect_imports(out);
                }
            }
            TypeName::Primitive(_) | TypeName::Raw(_) | TypeName::_Phantom(_) => {}
        }
    }

    /// Render this type name to a `pretty::BoxDoc` for width-aware formatting.
    ///
    /// The `resolved_name` closure maps (module, name) -> display name,
    /// accounting for import aliases.
    pub fn to_doc<F>(&self, resolve: &F) -> BoxDoc<'static, ()>
    where
        F: Fn(&str, &str) -> String,
    {
        match self {
            TypeName::Importable { module, name, .. } => {
                let display = resolve(module, name);
                BoxDoc::text(display)
            }
            TypeName::Primitive(name) => BoxDoc::text(name.clone()),
            TypeName::Raw(s) => BoxDoc::text(s.clone()),
            TypeName::Array(inner) => {
                // Default: TypeScript-style T[]
                inner.to_doc(resolve).append(BoxDoc::text("[]"))
            }
            TypeName::ReadonlyArray(inner) => {
                // Default: TypeScript-style `readonly T[]`; languages without
                // a direct readonly-array form fall through here too.
                BoxDoc::text("readonly ")
                    .append(inner.to_doc(resolve))
                    .append(BoxDoc::text("[]"))
            }
            TypeName::Generic { base, params } => {
                let base_doc = base.to_doc(resolve);
                let params_docs: Vec<_> = params.iter().map(|p| p.to_doc(resolve)).collect();
                let sep = BoxDoc::text(",").append(BoxDoc::softline());
                let params_doc = BoxDoc::intersperse(params_docs, sep);
                base_doc
                    .append(BoxDoc::text("<"))
                    .append(params_doc.nest(2).group())
                    .append(BoxDoc::text(">"))
            }
            TypeName::Union(members) => {
                let docs: Vec<_> = members.iter().map(|m| m.to_doc(resolve)).collect();
                let sep = BoxDoc::softline().append(BoxDoc::text("| "));
                BoxDoc::intersperse(docs, sep).group()
            }
            TypeName::Intersection(members) => {
                let docs: Vec<_> = members.iter().map(|m| m.to_doc(resolve)).collect();
                let sep = BoxDoc::softline().append(BoxDoc::text("& "));
                BoxDoc::intersperse(docs, sep).group()
            }
            TypeName::Pointer(inner) => BoxDoc::text("*").append(inner.to_doc(resolve)),
            TypeName::Slice(inner) => BoxDoc::text("[]").append(inner.to_doc(resolve)),
            TypeName::Map { key, value } => BoxDoc::text("map[")
                .append(key.to_doc(resolve))
                .append(BoxDoc::text("]"))
                .append(value.to_doc(resolve)),
            TypeName::Optional(inner) => {
                // Default: TypeScript-style T | null
                let inner_doc = inner.to_doc(resolve);
                inner_doc
                    .append(BoxDoc::softline())
                    .append(BoxDoc::text("| null"))
                    .group()
            }
            TypeName::Tuple(elements) => {
                let docs: Vec<_> = elements.iter().map(|e| e.to_doc(resolve)).collect();
                if docs.is_empty() {
                    return BoxDoc::text("()");
                }
                let sep = BoxDoc::text(",").append(BoxDoc::softline());
                BoxDoc::text("(")
                    .append(BoxDoc::intersperse(docs, sep).nest(2).group())
                    .append(BoxDoc::text(")"))
            }
            TypeName::Reference {
                inner,
                mutable,
                lifetime,
            } => {
                let mut prefix = String::from("&");
                if let Some(lt) = lifetime {
                    prefix.push_str(lt);
                    prefix.push(' ');
                }
                if *mutable {
                    prefix.push_str("mut ");
                }
                BoxDoc::text(prefix).append(inner.to_doc(resolve))
            }
            TypeName::Function {
                params,
                return_type,
            } => {
                let params_docs: Vec<_> = params.iter().map(|p| p.to_doc(resolve)).collect();
                let sep = BoxDoc::text(",").append(BoxDoc::softline());
                let params_doc = BoxDoc::intersperse(params_docs, sep);
                BoxDoc::text("(")
                    .append(params_doc.nest(2).group())
                    .append(BoxDoc::text(") => "))
                    .append(return_type.to_doc(resolve))
            }
            TypeName::AssociatedType {
                base,
                qualifier,
                member,
            } => {
                if let Some(qual) = qualifier {
                    // Default: Rust-style <Base as Qual>::Member
                    BoxDoc::text("<")
                        .append(base.to_doc(resolve))
                        .append(BoxDoc::text(" as "))
                        .append(qual.to_doc(resolve))
                        .append(BoxDoc::text(">::"))
                        .append(BoxDoc::text(member.clone()))
                } else {
                    // Default: Base::Member
                    base.to_doc(resolve)
                        .append(BoxDoc::text("::"))
                        .append(BoxDoc::text(member.clone()))
                }
            }
            TypeName::ImplTrait { bounds } => {
                let docs: Vec<_> = bounds.iter().map(|b| b.to_doc(resolve)).collect();
                let sep = BoxDoc::text(" + ");
                BoxDoc::text("impl ").append(BoxDoc::intersperse(docs, sep))
            }
            TypeName::DynTrait { bounds } => {
                let docs: Vec<_> = bounds.iter().map(|b| b.to_doc(resolve)).collect();
                let sep = BoxDoc::text(" + ");
                BoxDoc::text("dyn ").append(BoxDoc::intersperse(docs, sep))
            }
            TypeName::Wildcard {
                upper_bound,
                lower_bound,
            } => {
                debug_assert!(
                    upper_bound.is_none() || lower_bound.is_none(),
                    "Wildcard cannot have both upper and lower bounds"
                );
                if let Some(ub) = upper_bound {
                    BoxDoc::text("? extends ").append(ub.to_doc(resolve))
                } else if let Some(lb) = lower_bound {
                    BoxDoc::text("? super ").append(lb.to_doc(resolve))
                } else {
                    BoxDoc::text("?")
                }
            }
            TypeName::_Phantom(_) => BoxDoc::nil(),
        }
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
        let doc = self.to_doc(resolve);
        let mut buf = Vec::new();
        doc.render(width, &mut buf)
            .map_err(|e| crate::error::SigilStitchError::Render {
                context: "TypeName::render".to_string(),
                message: e.to_string(),
            })?;
        String::from_utf8(buf).map_err(|e| crate::error::SigilStitchError::Render {
            context: "TypeName::render UTF-8 conversion".to_string(),
            message: e.to_string(),
        })
    }

    /// Language-aware variant of [`TypeName::to_doc`] that consults the lang for
    /// syntax differences (e.g., generic delimiters `<>` vs `[]`).
    pub fn to_doc_with_lang<F>(&self, resolve: &F, lang: &L) -> BoxDoc<'static, ()>
    where
        F: Fn(&str, &str) -> String,
    {
        match self {
            TypeName::Generic { base, params } => {
                let base_doc = base.to_doc_with_lang(resolve, lang);
                let params_docs: Vec<_> = params
                    .iter()
                    .map(|p| p.to_doc_with_lang(resolve, lang))
                    .collect();
                match lang.generic_application_style() {
                    GenericApplicationStyle::Delimited => {
                        let sep = BoxDoc::text(",").append(BoxDoc::softline());
                        let params_doc = BoxDoc::intersperse(params_docs, sep);
                        base_doc
                            .append(BoxDoc::text(lang.generic_open().to_string()))
                            .append(params_doc.nest(2).group())
                            .append(BoxDoc::text(lang.generic_close().to_string()))
                    }
                    GenericApplicationStyle::PrefixJuxtaposition => {
                        let mut doc = base_doc;
                        for (i, param_doc) in params_docs.into_iter().enumerate() {
                            doc = doc.append(BoxDoc::text(" "));
                            if is_compound_type(&params[i]) {
                                doc = doc
                                    .append(BoxDoc::text("("))
                                    .append(param_doc)
                                    .append(BoxDoc::text(")"));
                            } else {
                                doc = doc.append(param_doc);
                            }
                        }
                        doc
                    }
                    GenericApplicationStyle::PostfixJuxtaposition => {
                        if params_docs.len() == 1 {
                            params_docs
                                .into_iter()
                                .next()
                                .unwrap()
                                .append(BoxDoc::text(" "))
                                .append(base_doc)
                        } else {
                            let sep = BoxDoc::text(",").append(BoxDoc::softline());
                            let params_doc = BoxDoc::intersperse(params_docs, sep);
                            BoxDoc::text("(")
                                .append(params_doc.nest(2).group())
                                .append(BoxDoc::text(") "))
                                .append(base_doc)
                        }
                    }
                }
            }
            TypeName::Array(inner) => {
                let inner_doc = inner.to_doc_with_lang(resolve, lang);
                render_presentation(&lang.present_array(), vec![inner_doc], lang)
            }
            TypeName::ReadonlyArray(inner) => {
                let inner_doc = inner.to_doc_with_lang(resolve, lang);
                if let Some(pres) = lang.present_readonly_array() {
                    render_presentation(&pres, vec![inner_doc], lang)
                } else {
                    // Default: "readonly " + array rendering
                    let array_doc =
                        render_presentation(&lang.present_array(), vec![inner_doc], lang);
                    BoxDoc::text("readonly ").append(array_doc)
                }
            }
            TypeName::Union(members) => {
                let docs: Vec<_> = members
                    .iter()
                    .map(|m| m.to_doc_with_lang(resolve, lang))
                    .collect();
                render_presentation(&lang.present_union(), docs, lang)
            }
            TypeName::Intersection(members) => {
                let docs: Vec<_> = members
                    .iter()
                    .map(|m| m.to_doc_with_lang(resolve, lang))
                    .collect();
                render_presentation(&lang.present_intersection(), docs, lang)
            }
            TypeName::Pointer(inner) => {
                let inner_doc = inner.to_doc_with_lang(resolve, lang);
                render_presentation(&lang.present_pointer(), vec![inner_doc], lang)
            }
            TypeName::Slice(inner) => {
                let inner_doc = inner.to_doc_with_lang(resolve, lang);
                render_presentation(&lang.present_slice(), vec![inner_doc], lang)
            }
            TypeName::Map { key, value } => {
                let key_doc = key.to_doc_with_lang(resolve, lang);
                let value_doc = value.to_doc_with_lang(resolve, lang);
                render_presentation(&lang.present_map(), vec![key_doc, value_doc], lang)
            }
            TypeName::Optional(inner) => {
                let inner_doc = inner.to_doc_with_lang(resolve, lang);
                let pres = lang.present_optional();
                match &pres {
                    TypePresentation::Infix { .. } => {
                        let null_doc = BoxDoc::text(lang.optional_absent_literal().to_string());
                        render_presentation(&pres, vec![inner_doc, null_doc], lang)
                    }
                    _ => render_presentation(&pres, vec![inner_doc], lang),
                }
            }
            TypeName::Tuple(elements) => {
                let docs: Vec<_> = elements
                    .iter()
                    .map(|e| e.to_doc_with_lang(resolve, lang))
                    .collect();
                render_presentation(&lang.present_tuple(), docs, lang)
            }
            TypeName::Reference {
                inner,
                mutable,
                lifetime,
            } => {
                let inner_doc = inner.to_doc_with_lang(resolve, lang);
                if let Some(lt) = lifetime {
                    let mut prefix = String::from("&");
                    prefix.push_str(lt);
                    prefix.push(' ');
                    if *mutable {
                        prefix.push_str("mut ");
                    }
                    BoxDoc::text(prefix).append(inner_doc)
                } else {
                    let pres = if *mutable {
                        lang.present_reference_mut()
                    } else {
                        lang.present_reference()
                    };
                    render_presentation(&pres, vec![inner_doc], lang)
                }
            }
            TypeName::Function {
                params,
                return_type,
            } => {
                let param_docs: Vec<_> = params
                    .iter()
                    .map(|p| p.to_doc_with_lang(resolve, lang))
                    .collect();
                let return_doc = return_type.to_doc_with_lang(resolve, lang);
                render_function_presentation(&lang.present_function(), param_docs, return_doc)
            }
            TypeName::AssociatedType {
                base,
                qualifier,
                member,
            } => {
                let base_doc = base.to_doc_with_lang(resolve, lang);
                let style = lang.present_associated_type();
                match style {
                    AssociatedTypeStyle::QualifiedPath {
                        open,
                        as_kw,
                        close_sep,
                        simple_sep,
                    } => {
                        if let Some(qual) = qualifier {
                            let qual_doc = qual.to_doc_with_lang(resolve, lang);
                            BoxDoc::text(open.to_string())
                                .append(base_doc)
                                .append(BoxDoc::text(as_kw.to_string()))
                                .append(qual_doc)
                                .append(BoxDoc::text(close_sep.to_string()))
                                .append(BoxDoc::text(member.clone()))
                        } else {
                            base_doc
                                .append(BoxDoc::text(simple_sep.to_string()))
                                .append(BoxDoc::text(member.clone()))
                        }
                    }
                    AssociatedTypeStyle::DotAccess => base_doc
                        .append(BoxDoc::text("."))
                        .append(BoxDoc::text(member.clone())),
                    AssociatedTypeStyle::IndexAccess { open, close } => base_doc
                        .append(BoxDoc::text(open.to_string()))
                        .append(BoxDoc::text(member.clone()))
                        .append(BoxDoc::text(close.to_string())),
                }
            }
            TypeName::ImplTrait { bounds } => {
                let docs: Vec<_> = bounds
                    .iter()
                    .map(|b| b.to_doc_with_lang(resolve, lang))
                    .collect();
                let pres = lang.present_impl_trait();
                let sep = BoxDoc::text(pres.separator.to_string());
                BoxDoc::text(pres.keyword.to_string()).append(BoxDoc::intersperse(docs, sep))
            }
            TypeName::DynTrait { bounds } => {
                let docs: Vec<_> = bounds
                    .iter()
                    .map(|b| b.to_doc_with_lang(resolve, lang))
                    .collect();
                let pres = lang.present_dyn_trait();
                let sep = BoxDoc::text(pres.separator.to_string());
                BoxDoc::text(pres.keyword.to_string()).append(BoxDoc::intersperse(docs, sep))
            }
            TypeName::Wildcard {
                upper_bound,
                lower_bound,
            } => {
                debug_assert!(
                    upper_bound.is_none() || lower_bound.is_none(),
                    "Wildcard cannot have both upper and lower bounds"
                );
                let pres = lang.present_wildcard();
                if let Some(ub) = upper_bound {
                    let ub_doc = ub.to_doc_with_lang(resolve, lang);
                    BoxDoc::text(pres.upper_keyword.to_string()).append(ub_doc)
                } else if let Some(lb) = lower_bound {
                    let lb_doc = lb.to_doc_with_lang(resolve, lang);
                    BoxDoc::text(pres.lower_keyword.to_string()).append(lb_doc)
                } else {
                    BoxDoc::text(pres.unbounded.to_string())
                }
            }
            // Leaf variants delegate to to_doc (no recursion needed).
            _ => self.to_doc(resolve),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang::typescript::TypeScript;

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
            impl crate::lang::CodeLang for $name {
                fn file_extension(&self) -> &str { self.0.file_extension() }
                fn reserved_words(&self) -> &[&str] { self.0.reserved_words() }
                fn render_imports(&self, imports: &crate::import::ImportGroup) -> String { self.0.render_imports(imports) }
                fn render_string_literal(&self, s: &str) -> String { self.0.render_string_literal(s) }
                fn render_doc_comment(&self, lines: &[&str]) -> String { self.0.render_doc_comment(lines) }
                fn line_comment_prefix(&self) -> &str { self.0.line_comment_prefix() }
                fn indent_unit(&self) -> &str { self.0.indent_unit() }
                fn uses_semicolons(&self) -> bool { self.0.uses_semicolons() }
                fn render_visibility(&self, vis: crate::spec::modifiers::Visibility, ctx: crate::spec::modifiers::DeclarationContext) -> &str { self.0.render_visibility(vis, ctx) }
                fn function_keyword(&self, ctx: crate::spec::modifiers::DeclarationContext) -> &str { self.0.function_keyword(ctx) }
                fn return_type_separator(&self) -> &str { self.0.return_type_separator() }
                fn type_keyword(&self, kind: crate::spec::modifiers::TypeKind) -> &str { self.0.type_keyword(kind) }
                fn field_terminator(&self) -> &str { self.0.field_terminator() }
                fn methods_inside_type_body(&self, kind: crate::spec::modifiers::TypeKind) -> bool { self.0.methods_inside_type_body(kind) }
                fn generic_constraint_keyword(&self) -> &str { self.0.generic_constraint_keyword() }
                fn generic_constraint_separator(&self) -> &str { self.0.generic_constraint_separator() }
                fn super_type_keyword(&self) -> &str { self.0.super_type_keyword() }
                fn implements_keyword(&self) -> &str { self.0.implements_keyword() }
                $($override)*
            }
        };
    }

    test_lang!(PrefixLang, TypeScript {
        fn generic_application_style(&self) -> GenericApplicationStyle {
            GenericApplicationStyle::PrefixJuxtaposition
        }
    });

    test_lang!(PostfixLang, TypeScript {
        fn generic_application_style(&self) -> GenericApplicationStyle {
            GenericApplicationStyle::PostfixJuxtaposition
        }
    });

    #[test]
    fn test_primitive() {
        let t = TypeName::<TypeScript>::primitive("number");
        assert_eq!(t.render(80, &identity_resolve).unwrap(), "number");
    }

    #[test]
    fn test_importable() {
        let t = TypeName::<TypeScript>::importable("./models", "User");
        assert_eq!(t.render(80, &identity_resolve).unwrap(), "User");
    }

    #[test]
    fn test_importable_with_alias() {
        let t = TypeName::<TypeScript>::importable("./other", "User");
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
        let t = TypeName::<TypeScript>::array(TypeName::primitive("string"));
        assert_eq!(t.render(80, &identity_resolve).unwrap(), "string[]");
    }

    #[test]
    fn test_generic() {
        let t = TypeName::<TypeScript>::generic(
            TypeName::primitive("Promise"),
            vec![TypeName::importable("./models", "User")],
        );
        assert_eq!(t.render(80, &identity_resolve).unwrap(), "Promise<User>");
    }

    #[test]
    fn test_generic_multiline() {
        let t = TypeName::<TypeScript>::generic(
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
        let t = TypeName::<TypeScript>::union(vec![
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
        let t = TypeName::<TypeScript>::union(vec![
            TypeName::primitive("VeryLongTypeName1"),
            TypeName::primitive("VeryLongTypeName2"),
            TypeName::primitive("VeryLongTypeName3"),
        ]);
        let output = t.render(30, &identity_resolve).unwrap();
        assert!(output.contains('\n'));
    }

    #[test]
    fn test_pointer() {
        let t = TypeName::<TypeScript>::pointer(TypeName::primitive("User"));
        assert_eq!(t.render(80, &identity_resolve).unwrap(), "*User");
    }

    #[test]
    fn test_slice() {
        let t = TypeName::<TypeScript>::slice(TypeName::primitive("User"));
        assert_eq!(t.render(80, &identity_resolve).unwrap(), "[]User");
    }

    #[test]
    fn test_map() {
        let t =
            TypeName::<TypeScript>::map(TypeName::primitive("string"), TypeName::primitive("User"));
        assert_eq!(t.render(80, &identity_resolve).unwrap(), "map[string]User");
    }

    #[test]
    fn test_optional() {
        let t = TypeName::<TypeScript>::optional(TypeName::primitive("string"));
        assert_eq!(t.render(80, &identity_resolve).unwrap(), "string | null");
    }

    #[test]
    fn test_function_type() {
        let t = TypeName::<TypeScript>::function(
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
        let inner = TypeName::<TypeScript>::generic(
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
        let t = TypeName::<TypeScript>::generic(
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
        let t = TypeName::<TypeScript>::raw("any");
        let mut imports = Vec::new();
        t.collect_imports(&mut imports);
        assert!(imports.is_empty());
        assert_eq!(t.render(80, &identity_resolve).unwrap(), "any");
    }

    #[test]
    fn test_with_alias_on_importable() {
        let t = TypeName::<TypeScript>::importable("./models", "User").with_alias("MyUser");
        // Verify the alias is stored correctly.
        if let TypeName::Importable { alias, .. } = &t {
            assert_eq!(alias.as_deref(), Some("MyUser"));
        } else {
            panic!("Expected Importable variant");
        }
    }

    #[test]
    fn test_with_alias_propagates_to_import_ref() {
        let t = TypeName::<TypeScript>::importable("./models", "User").with_alias("MyUser");
        let mut imports = Vec::new();
        t.collect_imports(&mut imports);
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].name, "User");
        assert_eq!(imports[0].alias.as_deref(), Some("MyUser"));
    }

    #[test]
    fn test_with_alias_noop_on_primitive() {
        // with_alias on a non-Importable variant should be a no-op.
        let t = TypeName::<TypeScript>::primitive("number").with_alias("MyNumber");
        assert_eq!(t.render(80, &identity_resolve).unwrap(), "number");
    }

    #[test]
    fn test_with_alias_renders_alias_name() {
        let t = TypeName::<TypeScript>::importable("./models", "User").with_alias("MyUser");
        // The resolve function should map to the alias.
        let resolve = |_module: &str, _name: &str| "MyUser".to_string();
        assert_eq!(t.render(80, &resolve).unwrap(), "MyUser");
    }

    #[test]
    fn test_tuple() {
        let t = TypeName::<TypeScript>::tuple(vec![
            TypeName::primitive("string"),
            TypeName::primitive("number"),
        ]);
        assert_eq!(t.render(80, &identity_resolve).unwrap(), "(string, number)");
    }

    #[test]
    fn test_unit_tuple() {
        let t = TypeName::<TypeScript>::unit();
        assert_eq!(t.render(80, &identity_resolve).unwrap(), "()");
    }

    #[test]
    fn test_tuple_collect_imports() {
        let t = TypeName::<TypeScript>::tuple(vec![
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
        let t = TypeName::<TypeScript>::tuple(vec![
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
        let t = TypeName::<RustLang>::tuple(vec![
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
        let t =
            TypeName::<Python>::tuple(vec![TypeName::primitive("str"), TypeName::primitive("int")]);
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "tuple[str, int]");
    }

    #[test]
    fn test_tuple_with_lang_cpp() {
        use crate::lang::cpp_lang::CppLang;
        let lang = CppLang::new();
        let t = TypeName::<CppLang>::tuple(vec![
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
        let t = TypeName::<RustLang>::unit();
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "()");
    }

    #[test]
    fn test_reference() {
        let t = TypeName::<TypeScript>::reference(TypeName::primitive("str"));
        assert_eq!(t.render(80, &identity_resolve).unwrap(), "&str");
    }

    #[test]
    fn test_reference_mut() {
        let t = TypeName::<TypeScript>::reference_mut(TypeName::primitive("Vec<i32>"));
        assert_eq!(t.render(80, &identity_resolve).unwrap(), "&mut Vec<i32>");
    }

    #[test]
    fn test_reference_collect_imports() {
        let t = TypeName::<TypeScript>::reference(TypeName::importable("./models", "User"));
        let mut imports = Vec::new();
        t.collect_imports(&mut imports);
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].name, "User");
    }

    #[test]
    fn test_reference_with_lang_rust() {
        use crate::lang::rust_lang::RustLang;
        let lang = RustLang::new();
        let t = TypeName::<RustLang>::reference(TypeName::primitive("String"));
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "&String");
    }

    #[test]
    fn test_reference_mut_with_lang_rust() {
        use crate::lang::rust_lang::RustLang;
        let lang = RustLang::new();
        let t = TypeName::<RustLang>::reference_mut(TypeName::primitive("String"));
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "&mut String");
    }

    #[test]
    fn test_reference_with_lang_cpp() {
        use crate::lang::cpp_lang::CppLang;
        let lang = CppLang::new();
        let t = TypeName::<CppLang>::reference(TypeName::primitive("std::string"));
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "const std::string&");
    }

    #[test]
    fn test_reference_mut_with_lang_cpp() {
        use crate::lang::cpp_lang::CppLang;
        let lang = CppLang::new();
        let t = TypeName::<CppLang>::reference_mut(TypeName::primitive("std::string"));
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "std::string&");
    }

    #[test]
    fn test_reference_with_lang_ts() {
        let lang = TypeScript::new();
        let t = TypeName::<TypeScript>::reference(TypeName::primitive("string"));
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "string");
    }

    #[test]
    fn test_reference_mut_with_lang_go() {
        use crate::lang::go_lang::GoLang;
        let lang = GoLang::new();
        let t = TypeName::<GoLang>::reference_mut(TypeName::primitive("int"));
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "*int");
    }

    #[test]
    fn test_reference_with_lang_go() {
        use crate::lang::go_lang::GoLang;
        let lang = GoLang::new();
        let t = TypeName::<GoLang>::reference(TypeName::primitive("int"));
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "int");
    }

    #[test]
    fn test_reference_with_lang_c() {
        use crate::lang::c_lang::CLang;
        let lang = CLang::new();
        let t = TypeName::<CLang>::reference(TypeName::primitive("int"));
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "const int*");
    }

    #[test]
    fn test_reference_mut_with_lang_c() {
        use crate::lang::c_lang::CLang;
        let lang = CLang::new();
        let t = TypeName::<CLang>::reference_mut(TypeName::primitive("int"));
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "int*");
    }

    #[test]
    fn test_generic_prefix_juxtaposition() {
        let lang = PrefixLang::new();
        let t = TypeName::<PrefixLang>::generic(
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
        let inner = TypeName::<PrefixLang>::generic(
            TypeName::primitive("Maybe"),
            vec![TypeName::primitive("Int")],
        );
        let t = TypeName::<PrefixLang>::generic(
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
        let t = TypeName::<PostfixLang>::generic(
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
        let t = TypeName::<PostfixLang>::generic(
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
        assert!(is_compound_type(&TypeName::<TypeScript>::generic(
            TypeName::primitive("A"),
            vec![TypeName::primitive("B")],
        )));
        assert!(is_compound_type(&TypeName::<TypeScript>::union(vec![
            TypeName::primitive("A"),
            TypeName::primitive("B"),
        ])));
        assert!(is_compound_type(&TypeName::<TypeScript>::intersection(
            vec![TypeName::primitive("A"), TypeName::primitive("B"),]
        )));
        assert!(is_compound_type(&TypeName::<TypeScript>::function(
            vec![TypeName::primitive("A")],
            TypeName::primitive("B"),
        )));
        assert!(is_compound_type(&TypeName::<TypeScript>::tuple(vec![
            TypeName::primitive("A"),
            TypeName::primitive("B"),
        ])));
        assert!(!is_compound_type(&TypeName::<TypeScript>::primitive("Int")));
        assert!(!is_compound_type(&TypeName::<TypeScript>::array(
            TypeName::primitive("Int"),
        )));
    }

    // --- Feature 07: Associated Types ---

    #[test]
    fn test_associated_type_rust_qualified() {
        use crate::lang::rust_lang::RustLang;
        let lang = RustLang::new();
        let t = TypeName::<RustLang>::associated_type(
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
        let t = TypeName::<RustLang>::member_type(TypeName::primitive("Self"), "Output");
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "Self::Output");
    }

    #[test]
    fn test_associated_type_ts_index_access() {
        let lang = TypeScript::new();
        let t = TypeName::<TypeScript>::associated_type(
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
        let t = TypeName::<JavaLang>::member_type(TypeName::primitive("Map"), "Entry");
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "Map.Entry");
    }

    #[test]
    fn test_associated_type_collect_imports() {
        let t = TypeName::<TypeScript>::associated_type(
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
        let t = TypeName::<RustLang>::impl_trait(vec![
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
        let t = TypeName::<RustLang>::dyn_trait(vec![TypeName::primitive("Error")]);
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "dyn Error");
    }

    #[test]
    fn test_impl_trait_ts_intersection() {
        let lang = TypeScript::new();
        let t = TypeName::<TypeScript>::impl_trait(vec![
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
        let t = TypeName::<JavaLang>::wildcard();
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "?");
    }

    #[test]
    fn test_wildcard_java_extends() {
        use crate::lang::java_lang::JavaLang;
        let lang = JavaLang::new();
        let t = TypeName::<JavaLang>::wildcard_extends(TypeName::primitive("Comparable"));
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "? extends Comparable");
    }

    #[test]
    fn test_wildcard_java_super() {
        use crate::lang::java_lang::JavaLang;
        let lang = JavaLang::new();
        let t = TypeName::<JavaLang>::wildcard_super(TypeName::primitive("Number"));
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "? super Number");
    }

    #[test]
    fn test_wildcard_kotlin() {
        use crate::lang::kotlin::Kotlin;
        let lang = Kotlin::new();
        let t = TypeName::<Kotlin>::wildcard();
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "*");
    }

    #[test]
    fn test_wildcard_kotlin_out() {
        use crate::lang::kotlin::Kotlin;
        let lang = Kotlin::new();
        let t = TypeName::<Kotlin>::wildcard_extends(TypeName::primitive("Number"));
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "out Number");
    }

    #[test]
    fn test_wildcard_kotlin_in() {
        use crate::lang::kotlin::Kotlin;
        let lang = Kotlin::new();
        let t = TypeName::<Kotlin>::wildcard_super(TypeName::primitive("Number"));
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "in Number");
    }

    #[test]
    fn test_wildcard_go() {
        use crate::lang::go_lang::GoLang;
        let lang = GoLang::new();
        let t = TypeName::<GoLang>::wildcard();
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "any");
    }

    #[test]
    fn test_wildcard_rust() {
        use crate::lang::rust_lang::RustLang;
        let lang = RustLang::new();
        let t = TypeName::<RustLang>::wildcard();
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "_");
    }

    #[test]
    fn test_impl_trait_collect_imports() {
        let t = TypeName::<TypeScript>::impl_trait(vec![
            TypeName::importable("./traits", "Serializable"),
            TypeName::primitive("Debug"),
        ]);
        let mut imports = Vec::new();
        t.collect_imports(&mut imports);
        assert_eq!(imports.len(), 1);
    }

    #[test]
    fn test_dyn_trait_collect_imports() {
        let t =
            TypeName::<TypeScript>::dyn_trait(vec![TypeName::importable("./errors", "AppError")]);
        let mut imports = Vec::new();
        t.collect_imports(&mut imports);
        assert_eq!(imports.len(), 1);
    }

    #[test]
    fn test_wildcard_collect_imports() {
        let t = TypeName::<TypeScript>::wildcard_extends(TypeName::importable("./models", "User"));
        let mut imports = Vec::new();
        t.collect_imports(&mut imports);
        assert_eq!(imports.len(), 1);
    }

    #[test]
    fn test_associated_type_default_rendering() {
        let t = TypeName::<TypeScript>::associated_type(
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
        let t = TypeName::<TypeScript>::member_type(TypeName::primitive("Self"), "Output");
        assert_eq!(t.render(80, &identity_resolve).unwrap(), "Self::Output");
    }

    #[test]
    fn test_impl_trait_default_rendering() {
        let t = TypeName::<TypeScript>::impl_trait(vec![TypeName::primitive("Display")]);
        assert_eq!(t.render(80, &identity_resolve).unwrap(), "impl Display");
    }

    #[test]
    fn test_dyn_trait_default_rendering() {
        let t = TypeName::<TypeScript>::dyn_trait(vec![
            TypeName::primitive("Error"),
            TypeName::primitive("Send"),
        ]);
        assert_eq!(t.render(80, &identity_resolve).unwrap(), "dyn Error + Send");
    }

    #[test]
    fn test_wildcard_default_rendering() {
        assert_eq!(
            TypeName::<TypeScript>::wildcard()
                .render(80, &identity_resolve)
                .unwrap(),
            "?"
        );
        assert_eq!(
            TypeName::<TypeScript>::wildcard_extends(TypeName::primitive("T"))
                .render(80, &identity_resolve)
                .unwrap(),
            "? extends T"
        );
        assert_eq!(
            TypeName::<TypeScript>::wildcard_super(TypeName::primitive("T"))
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
        let t = TypeName::<RustLang>::reference_with_lifetime(TypeName::primitive("str"), "'a");
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "&'a str");
    }

    #[test]
    fn test_reference_mut_with_lifetime_rust() {
        use crate::lang::rust_lang::RustLang;
        let lang = RustLang::new();
        let t =
            TypeName::<RustLang>::reference_mut_with_lifetime(TypeName::primitive("String"), "'a");
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "&'a mut String");
    }

    #[test]
    fn test_reference_with_lifetime_default_rendering() {
        let t = TypeName::<TypeScript>::reference_with_lifetime(TypeName::primitive("str"), "'a");
        assert_eq!(t.render(80, &identity_resolve).unwrap(), "&'a str");
    }

    #[test]
    fn test_reference_without_lifetime_unchanged() {
        use crate::lang::rust_lang::RustLang;
        let lang = RustLang::new();
        let t = TypeName::<RustLang>::reference(TypeName::primitive("String"));
        let doc = t.to_doc_with_lang(&identity_resolve, &lang);
        let mut buf = Vec::new();
        doc.render(80, &mut buf).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "&String");
    }
}
