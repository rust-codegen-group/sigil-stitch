//! Enum variant specification for type-safe enum generation.

use crate::code_block::{CodeBlock, CodeBlockBuilder};
use crate::error::SigilStitchError;
use crate::lang::CodeLang;
use crate::lang::capability::VariantCapability;
use crate::spec::annotation_spec::AnnotationSpec;
use crate::spec::field_spec::{FieldSequenceIntent, FieldSpec};
use crate::spec::modifiers::TypeKind;
use crate::spec::parameter_spec::ParameterSpec;
use crate::type_name::TypeName;

/// Legacy positional context passed to [`EnumVariantSpec::emit()`].
///
/// New code must lower variants as an owner-aware sequence. A caller-provided
/// first/last flag cannot establish whether separators, payloads, or section
/// termination are valid for the containing declaration.
#[deprecated(note = "use TypeSpec owner-aware variant sequence lowering")]
#[derive(Debug, Clone, Copy)]
pub struct VariantContext {
    /// Whether this is the first variant in the enum.
    pub is_first: bool,
    /// Whether this is the last variant in the enum.
    pub is_last: bool,
    /// Whether the enum has members (fields, properties, methods) after the variants.
    pub has_trailing_members: bool,
}

/// Owner-level semantic context relevant to one complete variant sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ConstructorArity {
    minimum_arguments: usize,
    maximum_arguments: Option<usize>,
}

impl ConstructorArity {
    pub(crate) fn from_parameters(parameters: &[ParameterSpec]) -> Self {
        let minimum_arguments = parameters
            .iter()
            .rposition(|parameter| parameter.default_value().is_none() && !parameter.is_variadic())
            .map_or(0, |index| index + 1);
        let maximum_arguments = parameters
            .iter()
            .all(|parameter| !parameter.is_variadic())
            .then_some(parameters.len());
        Self {
            minimum_arguments,
            maximum_arguments,
        }
    }

    pub(crate) fn accepts(self, argument_count: usize) -> bool {
        argument_count >= self.minimum_arguments
            && self
                .maximum_arguments
                .is_none_or(|maximum| argument_count <= maximum)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct VariantOwnerContext {
    has_following_members: bool,
    constructor_arities: Vec<ConstructorArity>,
    has_opaque_members: bool,
}

impl VariantOwnerContext {
    pub(crate) fn new(
        has_following_members: bool,
        constructor_arities: Vec<ConstructorArity>,
        has_opaque_members: bool,
    ) -> Self {
        Self {
            has_following_members,
            constructor_arities,
            has_opaque_members,
        }
    }

    fn has_following_members(&self) -> bool {
        self.has_following_members
    }

    fn has_declared_constructor(&self) -> bool {
        !self.constructor_arities.is_empty()
    }

    fn has_compatible_constructor(&self, argument_count: usize) -> bool {
        self.constructor_arities
            .iter()
            .any(|arity| arity.accepts(argument_count))
    }

    fn has_opaque_members(&self) -> bool {
        self.has_opaque_members
    }
}

/// Read-only semantic intent for one complete owner-aware variant sequence.
#[derive(Debug, Clone)]
pub struct VariantIntent<'a> {
    owner_name: &'a str,
    owner_kind: TypeKind,
    variants: &'a [EnumVariantSpec],
    owner_context: VariantOwnerContext,
}

impl<'a> VariantIntent<'a> {
    fn new(
        owner_name: &'a str,
        owner_kind: TypeKind,
        variants: &'a [EnumVariantSpec],
        owner_context: VariantOwnerContext,
    ) -> Self {
        Self {
            owner_name,
            owner_kind,
            variants,
            owner_context,
        }
    }

    /// Name of the declaration that owns this sequence.
    pub fn owner_name(&self) -> &'a str {
        self.owner_name
    }

    /// Kind of declaration that owns this sequence.
    pub fn owner_kind(&self) -> TypeKind {
        self.owner_kind
    }

    /// Variants in declaration order.
    pub fn variants(&self) -> &'a [EnumVariantSpec] {
        self.variants
    }

    /// Whether fields, properties, methods, or explicit members follow the sequence.
    pub fn has_following_members(&self) -> bool {
        self.owner_context.has_following_members()
    }

    /// Whether the owner declares a primary or structured constructor.
    pub fn has_declared_constructor(&self) -> bool {
        self.owner_context.has_declared_constructor()
    }

    /// Whether a structured constructor accepts this many enum-entry arguments.
    pub fn has_compatible_constructor(&self, argument_count: usize) -> bool {
        self.owner_context
            .has_compatible_constructor(argument_count)
    }

    /// Whether opaque member code may provide target-specific constructor syntax.
    pub fn has_opaque_members(&self) -> bool {
        self.owner_context.has_opaque_members()
    }
}

/// Variant-sequence intent whose intrinsic and target validation succeeded.
///
/// Only sigil-stitch constructs this wrapper, so lowerers cannot bypass the
/// selected adapter's capability profile and additive validation.
#[derive(Debug, Clone)]
pub struct ValidatedVariants<'a> {
    intent: VariantIntent<'a>,
}

impl<'a> ValidatedVariants<'a> {
    pub(crate) fn new(intent: VariantIntent<'a>) -> Self {
        Self { intent }
    }
}

impl<'a> std::ops::Deref for ValidatedVariants<'a> {
    type Target = VariantIntent<'a>;

    fn deref(&self) -> &Self::Target {
        &self.intent
    }
}

/// A single enum variant (e.g., `Red`, `Up = 'UP'`, `case red`).
///
/// Used with [`TypeSpec`](crate::spec::type_spec::TypeSpec) via `add_variant()`.
/// The selected language adapter validates and lowers the complete owner-aware
/// sequence, including preambles, payload grammar, separators, and section
/// termination. Deprecated enum configuration is consulted only by the
/// permissive external-adapter compatibility path.
///
/// For simple variants use [`EnumVariantSpec::new()`]; for variants with values,
/// annotations, or doc comments use [`EnumVariantSpec::builder()`].
///
/// # Variant forms
///
/// - **Simple**: `EnumVariantSpec::new("Red")` → `Red`
/// - **Discriminated**: `.discriminant(CodeBlock::of("42", ())?)` → `Red = 42`
/// - **Constructed**: `.constructor_argument(CodeBlock::of("\"red\"", ())?)` →
///   Java/Kotlin: `RED("red")`
/// - **Positional payload**: `.positional_payload(TypeName::primitive("i32"))` →
///   Rust: `Some(i32)`, Swift: `case success(Data)`
/// - **Record payload**: `.record_payload_field(FieldSpec::builder("x", TypeName::primitive("i32")).build())` →
///   Rust: `Move { x: i32, y: i32 }`
///
/// # Examples
///
/// ```
/// use sigil_stitch::prelude::*;
/// use sigil_stitch::spec::enum_variant_spec::EnumVariantSpec;
/// use sigil_stitch::lang::typescript::TypeScript;
///
/// let type_spec = TypeSpec::builder("Direction", TypeKind::Enum)
///     .add_variant(EnumVariantSpec::new("Up").unwrap())
///     .add_variant(
///         EnumVariantSpec::builder("Down")
///             .discriminant(CodeBlock::of("'DOWN'", ()).unwrap())
///             .build().unwrap(),
///     )
///     .build().unwrap();
/// ```
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EnumVariantSpec {
    pub(crate) name: String,
    pub(crate) doc: Vec<String>,
    pub(crate) value: Option<CodeBlock>,
    /// Explicit discriminant in a value-represented enum.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) discriminant: Option<CodeBlock>,
    /// Expressions passed to an enum entry constructor.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) constructor_arguments: Vec<CodeBlock>,
    pub(crate) annotations: Vec<CodeBlock>,
    pub(crate) annotation_specs: Vec<AnnotationSpec>,
    /// Associated types for tuple-style variants (e.g., `Some(T)`, `case .success(Data)`).
    pub(crate) associated_types: Vec<TypeName>,
    /// Named fields for struct-style variants (e.g., Rust `Move { x: i32, y: i32 }`).
    pub(crate) fields: Vec<FieldSpec>,
}

impl EnumVariantSpec {
    /// Create a simple variant with just a name.
    ///
    /// # Errors
    ///
    /// Returns [`SigilStitchError::EmptyName`] if `name` is empty.
    pub fn new(name: &str) -> Result<Self, crate::error::SigilStitchError> {
        snafu::ensure!(
            !name.is_empty(),
            crate::error::EmptyNameSnafu {
                builder: "EnumVariantSpec",
            }
        );
        Ok(Self {
            name: name.to_string(),
            doc: Vec::new(),
            value: None,
            discriminant: None,
            constructor_arguments: Vec::new(),
            annotations: Vec::new(),
            annotation_specs: Vec::new(),
            associated_types: Vec::new(),
            fields: Vec::new(),
        })
    }

    /// Whether this variant has any legacy or explicit value-like intent.
    #[deprecated(note = "use discriminant() or constructor_argument() intent")]
    pub fn has_value(&self) -> bool {
        self.value.is_some()
            || self.discriminant.is_some()
            || !self.constructor_arguments.is_empty()
    }

    /// Variant name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Documentation lines supplied by the caller.
    pub fn doc(&self) -> &[String] {
        &self.doc
    }

    /// Explicit discriminant, when present.
    pub fn discriminant(&self) -> Option<&CodeBlock> {
        self.discriminant.as_ref()
    }

    /// Expressions passed to the enum entry constructor.
    pub fn constructor_arguments(&self) -> &[CodeBlock] {
        &self.constructor_arguments
    }

    /// Positional payload types.
    pub fn positional_payload(&self) -> &[TypeName] {
        &self.associated_types
    }

    /// Named record-payload fields.
    pub fn record_payload(&self) -> &[FieldSpec] {
        &self.fields
    }

    /// Opaque annotation blocks supplied through the escape hatch.
    pub fn annotations(&self) -> &[CodeBlock] {
        &self.annotations
    }

    /// Structured annotation declarations.
    pub fn annotation_specs(&self) -> &[AnnotationSpec] {
        &self.annotation_specs
    }

    pub(crate) fn legacy_value(&self) -> Option<&CodeBlock> {
        self.value.as_ref()
    }

    /// Create a variant builder for more complex variants.
    pub fn builder(name: &str) -> EnumVariantSpecBuilder {
        EnumVariantSpecBuilder {
            name: name.to_string(),
            doc: Vec::new(),
            value: None,
            discriminant: None,
            constructor_arguments: Vec::new(),
            annotations: Vec::new(),
            annotation_specs: Vec::new(),
            associated_types: Vec::new(),
            fields: Vec::new(),
        }
    }

    /// Emit one legacy positional variant fragment.
    ///
    /// Strict built-in adapters reject this ownerless entry point. Add the
    /// variant to a [`TypeSpec`](crate::spec::type_spec::TypeSpec) so the
    /// adapter receives the owner and complete sequence. The method remains
    /// available only for pre-0.6.8 permissive external adapters.
    #[deprecated(note = "add the variant to TypeSpec for owner-aware sequence lowering")]
    #[allow(deprecated)]
    pub fn emit(
        &self,
        lang: &dyn CodeLang,
        ctx: VariantContext,
    ) -> Result<CodeBlock, SigilStitchError> {
        let mut cb = CodeBlock::builder();
        self.emit_into(&mut cb, lang, ctx)?;
        cb.build()
    }

    /// Emit one legacy positional variant directly into an existing builder.
    ///
    /// Like [`EnumVariantSpec::emit()`], this compatibility entry point rejects
    /// strict built-in adapters because positional context cannot prove a valid
    /// complete declaration.
    #[deprecated(note = "add the variant to TypeSpec for owner-aware sequence lowering")]
    #[allow(deprecated)]
    pub fn emit_into(
        &self,
        cb: &mut CodeBlockBuilder,
        lang: &dyn CodeLang,
        ctx: VariantContext,
    ) -> Result<(), SigilStitchError> {
        if !lang.capabilities().variant_validation_is_permissive() {
            return Err(SigilStitchError::VariantOwnerRequired {
                language: lang.file_extension().to_string(),
                variant_name: self.name.clone(),
            });
        }
        self.validate_intrinsic()?;
        crate::lang::variant_lowering::lower_legacy_into(lang, self, ctx, cb)
    }

    pub(crate) fn validate_sequence<'a>(
        owner_name: &'a str,
        owner_kind: TypeKind,
        variants: &'a [Self],
        owner_context: VariantOwnerContext,
        lang: &dyn CodeLang,
    ) -> Result<ValidatedVariants<'a>, SigilStitchError> {
        let mut errors = Vec::new();
        Self::collect_sequence_validation_errors(
            owner_name,
            owner_kind,
            variants,
            owner_context.clone(),
            lang,
            &mut errors,
        );
        if let Some(error) = errors.into_iter().next() {
            return Err(error);
        }
        Ok(ValidatedVariants::new(VariantIntent::new(
            owner_name,
            owner_kind,
            variants,
            owner_context,
        )))
    }

    pub(crate) fn collect_sequence_validation_errors(
        owner_name: &str,
        owner_kind: TypeKind,
        variants: &[Self],
        owner_context: VariantOwnerContext,
        lang: &dyn CodeLang,
        errors: &mut Vec<SigilStitchError>,
    ) {
        let intent = VariantIntent::new(owner_name, owner_kind, variants, owner_context);
        let mut seen_variant_names = std::collections::HashSet::new();
        let mut reported_variant_names = std::collections::HashSet::new();
        for variant in variants {
            if !seen_variant_names.insert(variant.name())
                && reported_variant_names.insert(variant.name())
            {
                errors.push(SigilStitchError::DuplicateVariantName {
                    type_name: owner_name.to_string(),
                    variant_name: variant.name().to_string(),
                });
            }
        }

        for variant in variants {
            variant.collect_intrinsic_validation_errors(errors);
        }

        let capabilities = lang.capabilities();
        if !capabilities.supports_variant_owner(owner_kind) {
            errors.push(SigilStitchError::UnsupportedVariantOwner {
                language: lang.file_extension().to_string(),
                type_name: owner_name.to_string(),
                owner_kind,
            });
            return;
        }

        for variant in variants {
            let unsupported: Vec<_> = variant
                .requested_capabilities()
                .into_iter()
                .filter(|capability| {
                    !capabilities.supports_variant_capability(owner_kind, *capability)
                })
                .collect();
            if !unsupported.is_empty() {
                errors.push(SigilStitchError::UnsupportedVariantCapabilities {
                    language: lang.file_extension().to_string(),
                    type_name: owner_name.to_string(),
                    variant_name: variant.name.clone(),
                    owner_kind,
                    capabilities: unsupported,
                });
            }

            if !variant.record_payload().is_empty() {
                let field_intent = FieldSequenceIntent::variant_record_payload(
                    variant.record_payload(),
                    owner_name,
                    owner_kind,
                    variant.name(),
                );
                if capabilities
                    .supports_variant_capability(owner_kind, VariantCapability::RecordPayload)
                {
                    FieldSpec::collect_sequence_validation_errors(field_intent, lang, errors);
                } else {
                    FieldSpec::collect_sequence_intrinsic_validation_errors(field_intent, errors);
                }
            }
        }

        lang.collect_variant_validation_errors(intent, errors);
    }

    pub(crate) fn lower_sequence(
        owner_name: &str,
        owner_kind: TypeKind,
        variants: &[Self],
        owner_context: VariantOwnerContext,
        lang: &dyn CodeLang,
    ) -> Result<CodeBlock, SigilStitchError> {
        let variants =
            Self::validate_sequence(owner_name, owner_kind, variants, owner_context, lang)?;
        lang.lower_variants(variants)
    }

    fn requested_capabilities(&self) -> Vec<VariantCapability> {
        let mut requested = Vec::new();
        if self.discriminant.is_some() {
            requested.push(VariantCapability::Discriminant);
        }
        if !self.constructor_arguments.is_empty() {
            requested.push(VariantCapability::ConstructorArguments);
        }
        if !self.associated_types.is_empty() {
            requested.push(VariantCapability::PositionalPayload);
        }
        if !self.fields.is_empty() {
            requested.push(VariantCapability::RecordPayload);
        }
        if !self.annotations.is_empty() || !self.annotation_specs.is_empty() {
            requested.push(VariantCapability::Attributes);
        }
        requested
    }

    fn validate_intrinsic(&self) -> Result<(), SigilStitchError> {
        let mut errors = Vec::new();
        self.collect_intrinsic_validation_errors(&mut errors);
        if let Some(error) = errors.into_iter().next() {
            return Err(error);
        }
        Ok(())
    }

    fn collect_intrinsic_validation_errors(&self, errors: &mut Vec<SigilStitchError>) {
        let forms: Vec<_> = self
            .requested_capabilities()
            .into_iter()
            .filter(|capability| *capability != VariantCapability::Attributes)
            .collect();
        if forms.len() > 1 || (self.value.is_some() && !forms.is_empty()) {
            errors.push(SigilStitchError::IncompatibleVariantCapabilities {
                variant_name: self.name.clone(),
                capabilities: forms,
            });
        }

        if self.value.as_ref().is_some_and(CodeBlock::is_empty) {
            errors.push(SigilStitchError::EmptyVariantOperand {
                variant_name: self.name.clone(),
                operand: "legacy value".to_string(),
            });
        }
        if self.discriminant.as_ref().is_some_and(CodeBlock::is_empty) {
            errors.push(SigilStitchError::EmptyVariantOperand {
                variant_name: self.name.clone(),
                operand: "discriminant".to_string(),
            });
        }
        for (index, argument) in self.constructor_arguments.iter().enumerate() {
            if argument.is_empty() {
                errors.push(SigilStitchError::EmptyVariantOperand {
                    variant_name: self.name.clone(),
                    operand: format!("constructor argument {index}"),
                });
            }
        }
        for (index, payload) in self.associated_types.iter().enumerate() {
            if payload.is_empty() {
                errors.push(SigilStitchError::EmptyVariantOperand {
                    variant_name: self.name.clone(),
                    operand: format!("positional payload type {index}"),
                });
            }
        }

        let mut seen_field_names = std::collections::HashSet::new();
        let mut reported_field_names = std::collections::HashSet::new();
        for field in &self.fields {
            if field.field_type().is_empty() {
                errors.push(SigilStitchError::EmptyVariantOperand {
                    variant_name: self.name.clone(),
                    operand: format!("record field {:?} type", field.name()),
                });
            }
            if !seen_field_names.insert(field.name()) && reported_field_names.insert(field.name()) {
                errors.push(SigilStitchError::DuplicateVariantRecordFieldName {
                    variant_name: self.name.clone(),
                    field_name: field.name().to_string(),
                });
            }
        }
    }
}

/// Builder for [`EnumVariantSpec`].
#[derive(Debug)]
pub struct EnumVariantSpecBuilder {
    name: String,
    doc: Vec<String>,
    value: Option<CodeBlock>,
    discriminant: Option<CodeBlock>,
    constructor_arguments: Vec<CodeBlock>,
    annotations: Vec<CodeBlock>,
    annotation_specs: Vec<AnnotationSpec>,
    associated_types: Vec<TypeName>,
    fields: Vec<FieldSpec>,
}

impl EnumVariantSpecBuilder {
    /// Add a doc comment line.
    pub fn doc(mut self, line: &str) -> Self {
        self.doc.push(line.to_string());
        self
    }

    /// Set the legacy target-dependent variant value.
    ///
    /// This method conflates discriminants with enum-entry constructor
    /// arguments. New code must state which meaning it intends.
    #[deprecated(note = "use discriminant() or constructor_argument()")]
    pub fn value(mut self, val: CodeBlock) -> Self {
        self.value = Some(val);
        self
    }

    /// Set an explicit enum-member discriminant.
    pub fn discriminant(mut self, discriminant: CodeBlock) -> Self {
        self.discriminant = Some(discriminant);
        self
    }

    /// Add an expression passed to the enum entry constructor.
    pub fn constructor_argument(mut self, argument: CodeBlock) -> Self {
        self.constructor_arguments.push(argument);
        self
    }

    /// Add an annotation (e.g., `#[default]`, `@JsonValue`).
    pub fn annotation(mut self, ann: CodeBlock) -> Self {
        self.annotations.push(ann);
        self
    }

    /// Add a structured annotation.
    pub fn annotate(mut self, spec: AnnotationSpec) -> Self {
        self.annotation_specs.push(spec);
        self
    }

    /// Add a type to the variant's positional payload.
    ///
    /// Call multiple times for multi-element payloads.
    pub fn positional_payload(mut self, ty: TypeName) -> Self {
        self.associated_types.push(ty);
        self
    }

    /// Add an associated type for tuple-style variants.
    ///
    /// Call multiple times for multi-element tuples.
    /// Example: `Some(i32)` or `case .success(Data, Int)`.
    #[deprecated(note = "use positional_payload()")]
    pub fn associated_type(mut self, ty: TypeName) -> Self {
        self.associated_types.push(ty);
        self
    }

    /// Add a named field to the variant's record payload.
    pub fn record_payload_field(mut self, field: FieldSpec) -> Self {
        self.fields.push(field);
        self
    }

    /// Add a named field for struct-style variants.
    ///
    /// Example: Rust `Move { x: i32, y: i32 }`.
    #[deprecated(note = "use record_payload_field()")]
    pub fn add_field(mut self, field: FieldSpec) -> Self {
        self.fields.push(field);
        self
    }

    /// Build the variant spec.
    ///
    /// # Errors
    ///
    /// Returns [`SigilStitchError::EmptyName`] if `name` is empty.
    pub fn build(self) -> Result<EnumVariantSpec, crate::error::SigilStitchError> {
        snafu::ensure!(
            !self.name.is_empty(),
            crate::error::EmptyNameSnafu {
                builder: "EnumVariantSpecBuilder",
            }
        );
        Ok(EnumVariantSpec {
            name: self.name,
            doc: self.doc,
            value: self.value,
            discriminant: self.discriminant,
            constructor_arguments: self.constructor_arguments,
            annotations: self.annotations,
            annotation_specs: self.annotation_specs,
            associated_types: self.associated_types,
            fields: self.fields,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang::CodeLang;
    use crate::lang::rust::Rust;
    use crate::lang::swift::Swift;
    use crate::lang::typescript::TypeScript;
    use crate::spec::field_spec::FieldSpec;
    use crate::spec::modifiers::TypeKind;
    use crate::spec::type_spec::TypeSpec;
    use crate::type_name::TypeName;

    fn render_enum(ts: &TypeSpec, lang: &dyn CodeLang) -> String {
        let blocks = ts.emit(lang).unwrap();
        let imports = crate::import::ImportGroup::new();
        let mut output = String::new();
        for (i, block) in blocks.iter().enumerate() {
            if i > 0 {
                output.push('\n');
            }
            let mut renderer = crate::code_renderer::CodeRenderer::new(lang, &imports, 80);
            output.push_str(&renderer.render(block).unwrap());
        }
        output
    }

    #[test]
    fn test_simple_variants() {
        let ts = TypeSpec::builder("Color", TypeKind::Enum)
            .add_variant(EnumVariantSpec::new("Red").unwrap())
            .add_variant(EnumVariantSpec::new("Green").unwrap())
            .add_variant(EnumVariantSpec::new("Blue").unwrap())
            .build()
            .unwrap();
        let output = render_enum(&ts, &Rust::new());
        assert!(output.contains("Red,"));
        assert!(output.contains("Green,"));
        assert!(output.contains("Blue,"));
    }

    #[test]
    fn test_variant_with_value() {
        let ts = TypeSpec::builder("Direction", TypeKind::Enum)
            .add_variant(
                EnumVariantSpec::builder("Up")
                    .discriminant(CodeBlock::of("'UP'", ()).unwrap())
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();
        let output = render_enum(&ts, &TypeScript::new());
        assert!(output.contains("Up = 'UP',"));
    }

    #[test]
    fn test_swift_variant_prefix() {
        let ts = TypeSpec::builder("Color", TypeKind::Enum)
            .add_variant(EnumVariantSpec::new("red").unwrap())
            .add_variant(EnumVariantSpec::new("green").unwrap())
            .build()
            .unwrap();
        let output = render_enum(&ts, &Swift::new());
        assert!(output.contains("case red"));
        assert!(output.contains("case green"));
        // Swift has no separator.
        assert!(!output.contains("case red,"));
    }

    #[test]
    fn test_trailing_separator() {
        let ts = TypeSpec::builder("Color", TypeKind::Enum)
            .add_variant(EnumVariantSpec::new("Red").unwrap())
            .build()
            .unwrap();
        let output = render_enum(&ts, &Rust::new());
        // Rust has trailing comma.
        assert!(output.contains("Red,"));
    }

    #[test]
    fn test_no_trailing_separator() {
        let ts = TypeSpec::builder("Color", TypeKind::Enum)
            .add_variant(EnumVariantSpec::new("RED").unwrap())
            .add_variant(EnumVariantSpec::new("GREEN").unwrap())
            .build()
            .unwrap();
        let output = render_enum(&ts, &crate::lang::c::C::new());
        assert!(output.contains("RED,"));
        // Last variant has no trailing comma in C.
        assert!(output.contains("GREEN\n"));
        assert!(!output.contains("GREEN,"));
    }

    #[test]
    fn test_new_empty_name_errors() {
        let result = EnumVariantSpec::new("");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("'name' must not be empty")
        );
    }

    #[test]
    fn test_build_empty_name_errors() {
        let result = EnumVariantSpec::builder("").build();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("'name' must not be empty")
        );
    }

    #[test]
    fn test_tuple_variant() {
        let ts = TypeSpec::builder("Expr", TypeKind::Enum)
            .add_variant(
                EnumVariantSpec::builder("Literal")
                    .positional_payload(TypeName::primitive("i64"))
                    .build()
                    .unwrap(),
            )
            .add_variant(EnumVariantSpec::new("Unit").unwrap())
            .build()
            .unwrap();
        let output = render_enum(&ts, &Rust::new());
        assert!(output.contains("Literal(i64),"));
        assert!(output.contains("Unit,"));
    }

    #[test]
    fn test_multi_tuple_variant() {
        let ts = TypeSpec::builder("Pair", TypeKind::Enum)
            .add_variant(
                EnumVariantSpec::builder("Both")
                    .positional_payload(TypeName::primitive("String"))
                    .positional_payload(TypeName::primitive("i32"))
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();
        let output = render_enum(&ts, &Rust::new());
        assert!(output.contains("Both(String, i32),"));
    }

    #[test]
    fn test_struct_variant() {
        let ts = TypeSpec::builder("Msg", TypeKind::Enum)
            .add_variant(EnumVariantSpec::new("Quit").unwrap())
            .add_variant(
                EnumVariantSpec::builder("Move")
                    .record_payload_field(
                        FieldSpec::builder("x", TypeName::primitive("i32"))
                            .build()
                            .unwrap(),
                    )
                    .record_payload_field(
                        FieldSpec::builder("y", TypeName::primitive("i32"))
                            .build()
                            .unwrap(),
                    )
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();
        let output = render_enum(&ts, &Rust::new());
        assert!(output.contains("Quit,"));
        assert!(output.contains("Move {"));
        assert!(output.contains("x: i32,"));
        assert!(output.contains("y: i32,"));
    }

    #[test]
    fn test_swift_associated_value() {
        let ts = TypeSpec::builder("Result", TypeKind::Enum)
            .add_variant(
                EnumVariantSpec::builder("success")
                    .positional_payload(TypeName::primitive("Data"))
                    .build()
                    .unwrap(),
            )
            .add_variant(EnumVariantSpec::new("failure").unwrap())
            .build()
            .unwrap();
        let output = render_enum(&ts, &Swift::new());
        assert!(output.contains("case success(Data)"));
        assert!(output.contains("case failure"));
    }

    #[test]
    #[allow(deprecated)]
    fn adapter_result_hooks_match_additive_validation() {
        let legacy = [EnumVariantSpec::builder("Legacy")
            .value(CodeBlock::of("1", ()).unwrap())
            .build()
            .unwrap()];
        let plain = [EnumVariantSpec::new("Plain").unwrap()];
        let context = VariantOwnerContext::new(false, Vec::new(), false);
        let languages: Vec<Box<dyn CodeLang>> = vec![
            Box::new(crate::lang::dart::Dart::new()),
            Box::new(crate::lang::haskell::Haskell::new()),
            Box::new(crate::lang::java::Java::new()),
            Box::new(crate::lang::kotlin::Kotlin::new()),
            Box::new(crate::lang::ocaml::OCaml::new()),
            Box::new(crate::lang::php::Php::new()),
            Box::new(crate::lang::scala::Scala::new()),
            Box::new(crate::lang::swift::Swift::new()),
        ];

        for lang in languages {
            let invalid = VariantIntent::new("Owner", TypeKind::Enum, &legacy, context.clone());
            assert!(lang.validate_variants(invalid).is_err());
            let valid = VariantIntent::new("Owner", TypeKind::Enum, &plain, context.clone());
            assert!(lang.validate_variants(valid).is_ok());
        }

        let invalid_rust = [EnumVariantSpec::builder("Record")
            .record_payload_field(
                FieldSpec::builder("value", TypeName::primitive("i32"))
                    .tag("unsupported")
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap()];
        let rust = Rust::new();
        assert!(
            FieldSpec::validate_sequence(
                FieldSequenceIntent::variant_record_payload(
                    invalid_rust[0].record_payload(),
                    "Owner",
                    TypeKind::Enum,
                    "Record",
                ),
                &rust,
            )
            .is_err()
        );
        assert!(
            rust.validate_variants(VariantIntent::new("Owner", TypeKind::Enum, &plain, context,))
                .is_ok()
        );
    }
}
