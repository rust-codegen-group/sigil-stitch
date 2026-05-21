//! Where-clause types and rendering for generic declarations.
//!
//! Extracted from `fun_spec.rs` to give where-clause rendering its own
//! module. Consumed by both [`FunSpec`](super::fun_spec::FunSpec) and
//! [`TypeSpec`](super::type_spec::TypeSpec).

use crate::code_block::Arg;
use crate::lang::CodeLang;
use crate::type_name::TypeName;

/// A single constraint in a where clause.
///
/// Represents `Subject: Bound1 + Bound2` in Rust's `where` block.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WhereConstraint {
    pub(crate) subject: TypeName,
    pub(crate) bounds: Vec<TypeName>,
}

/// How where-clause constraints are rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum WhereClauseStyle {
    /// Bounds stay inline in the type parameter list.
    Inline,
    /// Rust-style `where` block after the signature.
    WhereBlock,
    /// C#-style per-constraint `where` clauses after the signature.
    ///
    /// Each constraint gets its own `where` keyword on an indented line:
    /// ```text
    ///     where T : IComparable
    ///     where U : ISerializable
    /// ```
    SeparateWhere,
}

/// The kind of a type parameter (for higher-kinded type support).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TypeParamKind {
    /// Type constructor taking one argument: `* -> *` (Haskell), `F[_]` (Scala).
    Constructor1,
    /// Type constructor taking two arguments: `* -> * -> *`.
    Constructor2,
    /// Arbitrary kind expressed as a raw string.
    Raw(String),
}

/// A generic type parameter with optional bounds.
///
/// Used with [`FunSpec`](super::fun_spec::FunSpec) and
/// [`TypeSpec`](super::type_spec::TypeSpec) for generic declarations
/// (e.g., `<T extends Serializable>` in TypeScript, `<T: Clone>` in Rust).
///
/// # Examples
///
/// ```
/// use sigil_stitch::prelude::*;
/// use sigil_stitch::lang::typescript::TypeScript;
///
/// let tp = TypeParamSpec::new("T")
///     .with_bound(TypeName::primitive("Serializable"));
/// let fb = FunSpec::builder("serialize")
///     .add_type_param(tp);
/// ```
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TypeParamSpec {
    pub(crate) name: String,
    pub(crate) bounds: Vec<TypeName>,
    #[serde(default)]
    pub(crate) kind: Option<TypeParamKind>,
    #[serde(default)]
    pub(crate) is_lifetime: bool,
    #[serde(default)]
    pub(crate) context_bounds: Vec<TypeName>,
}

impl TypeParamSpec {
    /// Create a new type parameter with the given name and no bounds.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            bounds: Vec::new(),
            kind: None,
            is_lifetime: false,
            context_bounds: Vec::new(),
        }
    }

    /// Create a lifetime parameter (Rust `'a`).
    ///
    /// The name should include the tick: `"'a"`, `"'static"`.
    pub fn lifetime(name: &str) -> Self {
        Self {
            name: name.to_string(),
            bounds: Vec::new(),
            kind: None,
            is_lifetime: true,
            context_bounds: Vec::new(),
        }
    }

    /// Add a trait/interface bound to this type parameter.
    pub fn with_bound(mut self, bound: TypeName) -> Self {
        self.bounds.push(bound);
        self
    }

    /// Set this parameter as a higher-kinded type constructor.
    pub fn with_kind(mut self, kind: TypeParamKind) -> Self {
        self.kind = Some(kind);
        self
    }

    /// Add a context bound (e.g., Scala `[T : Ordering]`).
    pub fn with_context_bound(mut self, bound: TypeName) -> Self {
        self.context_bounds.push(bound);
        self
    }
}

/// Render type parameters into `<T: Bound, U>` form, appending to a format
/// string and args vec. Returns the format string fragment (empty string if
/// no type params).
pub fn render_type_params(
    params: &[TypeParamSpec],
    lang: &dyn CodeLang,
    args: &mut Vec<Arg>,
) -> String {
    if params.is_empty() {
        return String::new();
    }

    let generic = lang.generic_syntax();
    let constraint_kw = generic.constraint_keyword;
    let constraint_sep = generic.constraint_separator;

    let mut fmt = String::from(generic.open);
    let mut first = true;

    // Lifetimes first (Rust convention: `<'a, 'b, T, U>`).
    for tp in params.iter().filter(|p| p.is_lifetime) {
        if !first {
            fmt.push_str(", ");
        }
        fmt.push_str(&tp.name);
        if !tp.bounds.is_empty() {
            fmt.push_str(constraint_kw);
            for (j, bound) in tp.bounds.iter().enumerate() {
                if j > 0 {
                    fmt.push_str(constraint_sep);
                }
                fmt.push_str("%T");
                args.push(Arg::TypeName(bound.clone()));
            }
        }
        first = false;
    }

    // Then type parameters.
    for tp in params.iter().filter(|p| !p.is_lifetime) {
        if !first {
            fmt.push_str(", ");
        }
        fmt.push_str(&tp.name);
        if let Some(ref kind) = tp.kind {
            fmt.push_str(&lang.render_type_param_kind(kind));
        }
        if !tp.bounds.is_empty() {
            fmt.push_str(constraint_kw);
            for (j, bound) in tp.bounds.iter().enumerate() {
                if j > 0 {
                    fmt.push_str(constraint_sep);
                }
                fmt.push_str("%T");
                args.push(Arg::TypeName(bound.clone()));
            }
        }
        let ctx_kw = generic.context_bound_keyword;
        for ctx_bound in &tp.context_bounds {
            fmt.push_str(ctx_kw);
            fmt.push_str("%T");
            args.push(Arg::TypeName(ctx_bound.clone()));
        }
        first = false;
    }

    fmt.push_str(generic.close);
    fmt
}

/// Append a where-clause block to a format string.
///
/// Renders Rust-style:
/// ```text
/// \nwhere\n    T: Clone + Send,\n    U: Debug,
/// ```
pub(crate) fn emit_where_block(
    fmt: &mut String,
    args: &mut Vec<Arg>,
    constraints: &[WhereConstraint],
    lang: &dyn CodeLang,
) {
    let generic = lang.generic_syntax();
    let constraint_sep = generic.constraint_separator;
    let indent = lang.block_syntax().indent_unit;
    fmt.push_str("\nwhere\n");
    for (i, wc) in constraints.iter().enumerate() {
        if i > 0 {
            fmt.push('\n');
        }
        fmt.push_str(indent);
        fmt.push_str("%T");
        args.push(Arg::TypeName(wc.subject.clone()));
        fmt.push_str(lang.generic_syntax().constraint_keyword);
        for (j, bound) in wc.bounds.iter().enumerate() {
            if j > 0 {
                fmt.push_str(constraint_sep);
            }
            fmt.push_str("%T");
            args.push(Arg::TypeName(bound.clone()));
        }
        fmt.push(',');
    }
}

/// Append C#-style per-constraint where clauses to a format string.
///
/// Renders:
/// ```text
/// \n    where T : IComparable\n    where U : ISerializable
/// ```
pub(crate) fn emit_separate_where_block(
    fmt: &mut String,
    args: &mut Vec<Arg>,
    constraints: &[WhereConstraint],
    lang: &dyn CodeLang,
) {
    let generic = lang.generic_syntax();
    let indent = lang.block_syntax().indent_unit;
    for wc in constraints {
        fmt.push('\n');
        fmt.push_str(indent);
        fmt.push_str("where %T");
        args.push(Arg::TypeName(wc.subject.clone()));
        fmt.push_str(generic.constraint_keyword);
        for (j, bound) in wc.bounds.iter().enumerate() {
            if j > 0 {
                fmt.push_str(generic.constraint_separator);
            }
            fmt.push_str("%T");
            args.push(Arg::TypeName(bound.clone()));
        }
    }
}
