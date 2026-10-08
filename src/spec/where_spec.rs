//! Semantic type-parameter and where-constraint values.
//!
//! Consumed by both
//! [`FunSpec`](crate::spec::fun_spec::FunSpec) and
//! [`TypeSpec`](crate::spec::type_spec::TypeSpec). The rendering helpers at the
//! end of this module are frozen 0.6.8 compatibility facades; current built-in
//! declaration lowerers own their complete target grammar locally.

use crate::code_block::Arg;
use crate::error::SigilStitchError;
use crate::lang::CodeLang;
use crate::type_name::TypeName;

/// The semantic domain of a declaration binding.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum GenericParamDomain {
    /// One type-level binding, optionally annotated with a kind.
    Single {
        /// Optional kind annotation.
        kind: Option<KindExpr>,
    },
    /// A type-level pack whose elements have an optional kind annotation.
    Pack {
        /// Optional kind annotation for each element.
        element_kind: Option<KindExpr>,
    },
    /// A lifetime binding.
    Lifetime,
}

/// A structural kind annotation for a generic binding.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum KindExpr {
    /// The ordinary type kind.
    Type,
    /// A named/importable kind leaf.
    Named(TypeName),
    /// A constructor kind with parameter and result kinds.
    Constructor {
        /// Constructor parameter kinds.
        parameters: Vec<KindExpr>,
        /// Constructor result kind.
        result: Box<KindExpr>,
    },
}

/// One argument in a type application.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum TypeArgument {
    /// One ordinary type argument.
    Single(TypeName),
    /// An argument-list expansion of a complete type pattern.
    Expansion {
        /// Complete type pattern to expand.
        pattern: TypeName,
    },
}

/// Whether one callable slot is required or may be omitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum CallableParamPresence {
    /// The caller must supply the slot.
    Required,
    /// The caller may omit the slot.
    Optional,
}

/// One structural parameter or segment in a callable type.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum CallableParam {
    /// One scalar callable slot.
    Single {
        /// Optional source label requested for this slot.
        name: Option<String>,
        /// Slot type.
        type_name: TypeName,
        /// Whether the slot may be omitted.
        presence: CallableParamPresence,
    },
    /// A homogeneous repeated segment.
    Repeated {
        /// Optional source label for the segment.
        name: Option<String>,
        /// Type of each repeated element.
        element_type: TypeName,
    },
    /// An expansion of a previously declared pack pattern.
    Expansion {
        /// Optional source label for the segment.
        name: Option<String>,
        /// Complete expansion pattern.
        pattern: TypeName,
    },
}

/// A single constraint in a where clause.
///
/// Represents `Subject: Bound1 + Bound2` in Rust's `where` block.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WhereConstraint {
    pub(crate) subject: TypeName,
    pub(crate) bounds: Vec<TypeName>,
}

impl WhereConstraint {
    /// Return the constrained type or type parameter.
    pub fn subject(&self) -> &TypeName {
        &self.subject
    }

    /// Return the declared bounds.
    pub fn bounds(&self) -> &[TypeName] {
        &self.bounds
    }

    pub(crate) fn parameter_subject_name(&self) -> Option<&str> {
        match &self.subject {
            TypeName::Primitive(name) | TypeName::Parameter(name) => Some(name),
            _ => None,
        }
    }
}

/// How where-clause constraints are rendered.
#[deprecated(
    note = "legacy 0.6.8 declaration grammar; migrate declarations to language-owned lowering"
)]
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
#[deprecated(note = "legacy kind metadata; use GenericParamDomain and KindExpr")]
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
#[deprecated(note = "legacy declaration binding; use GenericParamSpec")]
pub struct TypeParamSpec {
    pub(crate) name: String,
    pub(crate) bounds: Vec<TypeName>,
    #[serde(default)]
    #[expect(
        deprecated,
        reason = "retain released compatibility metadata and hooks"
    )]
    pub(crate) kind: Option<TypeParamKind>,
    #[serde(default)]
    pub(crate) is_lifetime: bool,
    #[serde(default)]
    pub(crate) context_bounds: Vec<TypeName>,
}

#[allow(
    deprecated,
    reason = "released compatibility constructors retain their old data model"
)]
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
    /// The name must include the apostrophe and name a declared lifetime such
    /// as `"'a"`. Reserved lifetimes such as `"'static"` and `"'_"` are not
    /// valid declaration parameter names.
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

    /// Return the declared parameter name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the direct bounds on this parameter.
    pub fn bounds(&self) -> &[TypeName] {
        &self.bounds
    }

    /// Return the higher-kinded parameter shape, when present.
    pub fn kind(&self) -> Option<&TypeParamKind> {
        self.kind.as_ref()
    }

    /// Whether this parameter represents a lifetime.
    pub fn is_lifetime(&self) -> bool {
        self.is_lifetime
    }

    /// Return context bounds such as Scala context parameters.
    pub fn context_bounds(&self) -> &[TypeName] {
        &self.context_bounds
    }
}

/// A modern declaration binding with an explicit semantic domain.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct GenericParamSpec {
    name: String,
    domain: GenericParamDomain,
    bounds: Vec<TypeName>,
    context_bounds: Vec<TypeName>,
}

impl GenericParamSpec {
    /// Start a generic binding with an explicit semantic domain.
    pub fn builder(name: impl Into<String>, domain: GenericParamDomain) -> GenericParamSpecBuilder {
        GenericParamSpecBuilder {
            spec: Self {
                name: name.into(),
                domain,
                bounds: Vec::new(),
                context_bounds: Vec::new(),
            },
        }
    }
    /// Construct and intrinsically validate one binding.
    pub fn new(
        name: impl Into<String>,
        domain: GenericParamDomain,
    ) -> Result<Self, SigilStitchError> {
        let spec = Self {
            name: name.into(),
            domain,
            bounds: Vec::new(),
            context_bounds: Vec::new(),
        };
        spec.validate()?;
        Ok(spec)
    }

    /// Construct one ordinary type binding.
    pub fn single(name: impl Into<String>) -> Result<Self, SigilStitchError> {
        Self::new(name, GenericParamDomain::Single { kind: None })
    }

    /// Construct one type-pack binding.
    pub fn pack(name: impl Into<String>) -> Result<Self, SigilStitchError> {
        Self::new(name, GenericParamDomain::Pack { element_kind: None })
    }

    /// Construct one lifetime binding.
    pub fn lifetime(name: impl Into<String>) -> Result<Self, SigilStitchError> {
        Self::new(name, GenericParamDomain::Lifetime)
    }

    /// Add a declaration bound.
    pub fn with_bound(mut self, bound: TypeName) -> Result<Self, SigilStitchError> {
        self.bounds.push(bound);
        self.validate()?;
        Ok(self)
    }

    /// Add a context bound.
    pub fn with_context_bound(mut self, bound: TypeName) -> Result<Self, SigilStitchError> {
        self.context_bounds.push(bound);
        self.validate()?;
        Ok(self)
    }

    /// Binding name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Binding domain.
    pub fn domain(&self) -> &GenericParamDomain {
        &self.domain
    }

    /// Direct bounds.
    pub fn bounds(&self) -> &[TypeName] {
        &self.bounds
    }

    /// Context bounds.
    pub fn context_bounds(&self) -> &[TypeName] {
        &self.context_bounds
    }

    /// Validate the binding's own structure.
    pub fn validate(&self) -> Result<(), SigilStitchError> {
        if self.name.trim().is_empty() {
            return Err(SigilStitchError::InvalidTypeParameter {
                parameter_name: self.name.clone(),
                type_name: String::new(),
                reason: "binding name must not be blank".to_string(),
            });
        }
        if self
            .name
            .chars()
            .any(|character| character.is_control() || matches!(character, '\u{2028}' | '\u{2029}'))
        {
            return Err(SigilStitchError::InvalidTypeParameter {
                parameter_name: self.name.clone(),
                type_name: String::new(),
                reason: "binding name must not contain control characters".to_string(),
            });
        }
        match &self.domain {
            GenericParamDomain::Single { kind: Some(kind) }
            | GenericParamDomain::Pack {
                element_kind: Some(kind),
            } => validate_kind(kind)?,
            _ => {}
        }
        for (index, bound) in self
            .bounds
            .iter()
            .chain(self.context_bounds.iter())
            .enumerate()
        {
            crate::type_name_lowering::validation::validate_type_name(
                bound,
                &crate::type_name_lowering::DiagnosticPath::root(&format!(
                    "generic_param.bound[{index}]"
                )),
            )?;
        }
        Ok(())
    }
}

/// Builder whose completion checks one binding's intrinsic structure.
#[derive(Debug, Clone)]
pub struct GenericParamSpecBuilder {
    spec: GenericParamSpec,
}

impl GenericParamSpecBuilder {
    /// Add a direct bound without selecting its target grammar.
    pub fn with_bound(mut self, bound: TypeName) -> Self {
        self.spec.bounds.push(bound);
        self
    }

    /// Add a context bound without selecting its target grammar.
    pub fn with_context_bound(mut self, bound: TypeName) -> Self {
        self.spec.context_bounds.push(bound);
        self
    }

    /// Validate and complete the binding.
    pub fn build(self) -> Result<GenericParamSpec, SigilStitchError> {
        self.spec.validate()?;
        Ok(self.spec)
    }
}

fn validate_kind(kind: &KindExpr) -> Result<(), SigilStitchError> {
    match kind {
        KindExpr::Type => Ok(()),
        KindExpr::Named(value) => {
            if !matches!(
                value,
                TypeName::Primitive(_) | TypeName::Importable { .. } | TypeName::Parameter(_)
            ) {
                return Err(SigilStitchError::InvalidTypeName {
                    context: "generic_param.kind".into(),
                    reason: "a named kind requires a named leaf, not raw or compound source".into(),
                });
            }
            crate::type_name_lowering::validation::validate_type_name(
                value,
                &crate::type_name_lowering::DiagnosticPath::root("generic_param.kind"),
            )
        }
        KindExpr::Constructor { parameters, result } => {
            for parameter in parameters {
                validate_kind(parameter)?;
            }
            validate_kind(result)
        }
    }
}

/// One ordered binding, retained privately so old and modern inputs share a view.
#[allow(
    deprecated,
    reason = "derive compatibility storage for released bindings"
)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) enum GenericParamEntry {
    /// Modern semantic input.
    Modern(GenericParamSpec),
    /// Released compatibility input.
    Legacy(TypeParamSpec),
}

/// Borrowed semantic view over one modern or compatibility binding.
#[derive(Debug, Clone)]
pub struct GenericParamView<'a> {
    name: &'a str,
    domain: std::borrow::Cow<'a, GenericParamDomain>,
    pub(crate) bounds: std::borrow::Cow<'a, [TypeName]>,
    context_bounds: &'a [TypeName],
    #[expect(
        deprecated,
        reason = "retain released compatibility metadata and hooks"
    )]
    legacy_kind: Option<&'a TypeParamKind>,
}

/// Iterator over the single ordered binding store.
#[derive(Debug, Clone)]
pub(crate) struct GenericParamIter<'a> {
    entries: std::slice::Iter<'a, GenericParamEntry>,
}

impl<'a> GenericParamIter<'a> {
    pub(crate) fn new(entries: &'a [GenericParamEntry]) -> Self {
        Self {
            entries: entries.iter(),
        }
    }
}

impl<'a> Iterator for GenericParamIter<'a> {
    type Item = GenericParamView<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.entries.next().map(GenericParamView::from_entry)
    }
}

impl<'a> GenericParamView<'a> {
    pub(crate) fn from_entry(entry: &'a GenericParamEntry) -> Self {
        match entry {
            GenericParamEntry::Modern(parameter) => Self {
                name: parameter.name(),
                domain: std::borrow::Cow::Borrowed(parameter.domain()),
                bounds: std::borrow::Cow::Borrowed(parameter.bounds()),
                context_bounds: parameter.context_bounds(),
                legacy_kind: None,
            },
            GenericParamEntry::Legacy(parameter) => Self::from_legacy(parameter),
        }
    }

    #[expect(
        deprecated,
        reason = "retain released compatibility metadata and hooks"
    )]
    pub(crate) fn from_legacy(parameter: &'a TypeParamSpec) -> Self {
        let domain = if parameter.is_lifetime() {
            GenericParamDomain::Lifetime
        } else {
            GenericParamDomain::Single { kind: None }
        };
        Self {
            name: parameter.name(),
            domain: std::borrow::Cow::Owned(domain),
            bounds: std::borrow::Cow::Borrowed(parameter.bounds()),
            context_bounds: parameter.context_bounds(),
            legacy_kind: parameter.kind(),
        }
    }

    /// Binding name.
    pub fn name(&self) -> &'a str {
        self.name
    }
    /// Structural domain.
    pub fn domain(&self) -> std::borrow::Cow<'a, GenericParamDomain> {
        self.domain.clone()
    }
    /// Direct bounds.
    pub fn bounds(&self) -> &[TypeName] {
        &self.bounds
    }
    /// Context bounds.
    pub fn context_bounds(&self) -> &'a [TypeName] {
        self.context_bounds
    }
    /// Deprecated compatibility metadata, when this is a legacy entry.
    #[deprecated(note = "legacy type-parameter metadata; use domain()")]
    #[expect(
        deprecated,
        reason = "retain released compatibility metadata and hooks"
    )]
    pub fn legacy_kind(&self) -> Option<&'a TypeParamKind> {
        self.legacy_kind
    }

    #[expect(
        deprecated,
        reason = "retain released compatibility metadata and hooks"
    )]
    pub(crate) fn kind(&self) -> Option<&'a TypeParamKind> {
        self.legacy_kind
    }

    pub(crate) fn has_kind_or_pack_domain(&self) -> bool {
        matches!(
            self.domain.as_ref(),
            GenericParamDomain::Pack { .. } | GenericParamDomain::Single { kind: Some(_) }
        )
    }

    pub(crate) fn is_lifetime(&self) -> bool {
        matches!(self.domain.as_ref(), GenericParamDomain::Lifetime)
    }

    pub(crate) fn has_constructor_kind(&self) -> bool {
        self.legacy_kind.is_some()
            || matches!(
                self.domain.as_ref(),
                GenericParamDomain::Single {
                    kind: Some(KindExpr::Constructor { .. })
                } | GenericParamDomain::Pack {
                    element_kind: Some(KindExpr::Constructor { .. })
                }
            )
    }

    #[expect(
        deprecated,
        reason = "retain released compatibility metadata and hooks"
    )]
    fn compatibility_metadata(&self) -> TypeParamSpec {
        TypeParamSpec {
            name: self.name.to_string(),
            bounds: self.bounds().to_vec(),
            context_bounds: self.context_bounds.to_vec(),
            kind: self.legacy_kind.cloned(),
            is_lifetime: self.is_lifetime(),
        }
    }

    #[expect(
        deprecated,
        reason = "retain released compatibility metadata and hooks"
    )]
    pub(crate) fn compatibility_input(
        &self,
        language: &str,
    ) -> Result<TypeParamSpec, SigilStitchError> {
        if !matches!(
            self.domain.as_ref(),
            GenericParamDomain::Single { kind: None } | GenericParamDomain::Lifetime
        ) {
            return Err(SigilStitchError::UnsupportedTypeName {
                language: language.to_string(),
                context: format!("generic_param.{}", self.name),
                reason: "the compatibility declaration lowerer cannot preserve this binding domain"
                    .into(),
            });
        }
        Ok(self.compatibility_metadata())
    }
}

/// Render type parameters through the frozen 0.6.8 shared grammar.
///
/// Returns an empty string when `params` is empty. New adapters should instead
/// implement complete [`CodeLang::lower_function`] and [`CodeLang::lower_type`]
/// operations.
#[deprecated(note = "legacy 0.6.8 declaration grammar; implement complete language-owned lowering")]
#[expect(
    deprecated,
    reason = "retain released compatibility metadata and hooks"
)]
pub fn render_type_params(
    params: &[TypeParamSpec],
    lang: &dyn CodeLang,
    args: &mut Vec<Arg>,
) -> String {
    render_type_params_for(params, lang, args)
}

#[expect(deprecated, reason = "0.6.8 generic declaration compatibility bridge")]
pub(crate) fn render_type_params_for<L: CodeLang + ?Sized>(
    params: &[TypeParamSpec],
    lang: &L,
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
#[expect(deprecated, reason = "0.6.8 declaration grammar compatibility bridge")]
pub(crate) fn emit_where_block<L: CodeLang + ?Sized>(
    fmt: &mut String,
    args: &mut Vec<Arg>,
    constraints: &[WhereConstraint],
    lang: &L,
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
#[expect(deprecated, reason = "0.6.8 declaration grammar compatibility bridge")]
pub(crate) fn emit_separate_where_block<L: CodeLang + ?Sized>(
    fmt: &mut String,
    args: &mut Vec<Arg>,
    constraints: &[WhereConstraint],
    lang: &L,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_parameter_domains_validate_and_preserve_bounds() {
        let parameter = GenericParamSpec::pack("Ts")
            .unwrap()
            .with_bound(TypeName::primitive("Clone"))
            .unwrap();
        assert_eq!(parameter.name(), "Ts");
        assert!(matches!(
            parameter.domain(),
            GenericParamDomain::Pack { .. }
        ));
        assert_eq!(parameter.bounds(), &[TypeName::primitive("Clone")]);
    }

    #[test]
    fn generic_parameter_rejects_blank_names() {
        let error = GenericParamSpec::single(" ").unwrap_err();
        assert!(error.to_string().contains("binding name must not be blank"));
    }

    #[test]
    fn generic_parameter_serde_round_trip_preserves_domain() {
        let parameter = GenericParamSpec::new(
            "F",
            GenericParamDomain::Single {
                kind: Some(KindExpr::Constructor {
                    parameters: vec![KindExpr::Type],
                    result: Box::new(KindExpr::Type),
                }),
            },
        )
        .unwrap();
        let encoded = serde_json::to_string(&parameter).unwrap();
        let decoded: GenericParamSpec = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, parameter);
    }
}
