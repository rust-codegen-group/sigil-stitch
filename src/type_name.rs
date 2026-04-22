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
        return_doc
            .append(keyword_doc)
            .append(params_block)
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
            TypeName::Generic { base, params } => {
                base.collect_imports(out);
                for p in params {
                    p.collect_imports(out);
                }
            }
            TypeName::Union(members) | TypeName::Intersection(members) => {
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
                let sep = BoxDoc::text(",").append(BoxDoc::softline());
                let params_doc = BoxDoc::intersperse(params_docs, sep);
                base_doc
                    .append(BoxDoc::text(lang.generic_open().to_string()))
                    .append(params_doc.nest(2).group())
                    .append(BoxDoc::text(lang.generic_close().to_string()))
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
                        let null_doc =
                            BoxDoc::text(lang.optional_absent_literal().to_string());
                        render_presentation(&pres, vec![inner_doc, null_doc], lang)
                    }
                    _ => render_presentation(&pres, vec![inner_doc], lang),
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
}
