//! Field specification for struct fields / class properties.

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::lang::CodeLang;
use crate::lang::capability::{FieldCapability, FieldContext};
use crate::spec::annotation_spec::AnnotationSpec;
use crate::spec::modifiers::{DeclarationContext, Modifiers, TypeKind, Visibility};
use crate::type_name::TypeName;

/// Read-only semantic intent for one complete field sequence.
///
/// The selected context carries owner meaning without exposing target grammar
/// or caller-supplied separator state.
#[derive(Debug, Clone, Copy)]
pub struct FieldSequenceIntent<'a> {
    fields: &'a [FieldSpec],
    context: FieldContext,
    owner_name: Option<&'a str>,
    variant_name: Option<&'a str>,
}

impl<'a> FieldSequenceIntent<'a> {
    fn direct(fields: &'a [FieldSpec], declaration_context: DeclarationContext) -> Self {
        Self {
            fields,
            context: FieldContext::Direct(declaration_context),
            owner_name: None,
            variant_name: None,
        }
    }

    pub(crate) fn type_members(
        fields: &'a [FieldSpec],
        owner_name: &'a str,
        owner_kind: TypeKind,
    ) -> Self {
        Self {
            fields,
            context: FieldContext::TypeMember(owner_kind),
            owner_name: Some(owner_name),
            variant_name: None,
        }
    }

    pub(crate) fn variant_record_payload(
        fields: &'a [FieldSpec],
        owner_name: &'a str,
        owner_kind: TypeKind,
        variant_name: &'a str,
    ) -> Self {
        Self {
            fields,
            context: FieldContext::VariantRecordPayload(owner_kind),
            owner_name: Some(owner_name),
            variant_name: Some(variant_name),
        }
    }

    pub(crate) fn closed_sum_record_payload(
        fields: &'a [FieldSpec],
        owner_name: &'a str,
        variant_name: &'a str,
    ) -> Self {
        Self {
            fields,
            context: FieldContext::ClosedSumRecordPayload,
            owner_name: Some(owner_name),
            variant_name: Some(variant_name),
        }
    }

    /// Fields in declaration order.
    pub fn fields(&self) -> &'a [FieldSpec] {
        self.fields
    }

    /// Semantic context for the complete sequence.
    pub fn context(&self) -> FieldContext {
        self.context
    }

    /// Name of the owning type, when the sequence has one.
    pub fn owner_name(&self) -> Option<&'a str> {
        self.owner_name
    }

    /// Name of the owning variant for a record payload.
    pub fn variant_name(&self) -> Option<&'a str> {
        self.variant_name
    }
}

/// Field-sequence intent whose intrinsic and target validation succeeded.
///
/// Only sigil-stitch constructs this wrapper, so lowerers cannot bypass the
/// selected adapter's capability profile and additive validation.
#[derive(Debug, Clone)]
pub struct ValidatedFields<'a> {
    intent: FieldSequenceIntent<'a>,
}

impl<'a> ValidatedFields<'a> {
    fn new(intent: FieldSequenceIntent<'a>) -> Self {
        Self { intent }
    }
}

impl<'a> std::ops::Deref for ValidatedFields<'a> {
    type Target = FieldSequenceIntent<'a>;

    fn deref(&self) -> &Self::Target {
        &self.intent
    }
}

/// A single field/property in a struct or class.
///
/// `FieldSpec` represents a named, typed member of a type declaration. It supports
/// visibility modifiers, static/readonly flags, initializers, doc comments,
/// annotations, and struct tags (for Go). The emitted format adapts to the target
/// language (e.g., `name: string;` in TypeScript vs `pub name: String,` in Rust).
///
/// Use [`FieldSpec::builder()`] to construct, then add to a
/// [`TypeSpec`](crate::spec::type_spec::TypeSpec) with `add_field()`.
///
/// # Examples
///
/// ```
/// use sigil_stitch::prelude::*;
/// use sigil_stitch::lang::typescript::TypeScript;
///
/// let field = FieldSpec::builder("name", TypeName::primitive("string"))
///     .visibility(Visibility::Private)
///     .is_readonly()
///     .build().unwrap();
/// ```
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FieldSpec {
    pub(crate) name: String,
    pub(crate) field_type: TypeName,
    pub(crate) modifiers: Modifiers,
    pub(crate) doc: Vec<String>,
    pub(crate) initializer: Option<CodeBlock>,
    pub(crate) annotations: Vec<CodeBlock>,
    pub(crate) annotation_specs: Vec<AnnotationSpec>,
    /// Struct tag (e.g., Go: `` `json:"name"` ``). Emitted inline after the type.
    pub(crate) tag: Option<String>,
    /// Whether this field is optional (key may be absent from the containing value).
    ///
    /// Distinct from nullability (value may be `null`), which is expressed via
    /// [`TypeName::Optional`]. The selected language adapter validates and
    /// lowers this intent in the complete field sequence.
    pub(crate) is_optional: bool,
}

impl FieldSpec {
    /// Create a new [`FieldSpecBuilder`] with the given name and type.
    pub fn builder(name: &str, field_type: TypeName) -> FieldSpecBuilder {
        FieldSpecBuilder {
            name: name.to_string(),
            field_type,
            modifiers: Modifiers::default(),
            doc: Vec::new(),
            initializer: None,
            annotations: Vec::new(),
            annotation_specs: Vec::new(),
            tag: None,
            is_optional: false,
        }
    }

    /// Convenience constructor for a simple field (name + type, no modifiers).
    pub fn new(name: &str, field_type: TypeName) -> Result<Self, crate::error::SigilStitchError> {
        Self::builder(name, field_type).build()
    }

    /// Infallible convenience constructor for a simple field.
    ///
    /// # Panics
    ///
    /// Panics if `name` is empty.
    pub fn of(name: &str, field_type: TypeName) -> Self {
        Self::new(name, field_type).expect("FieldSpec name must not be empty")
    }

    /// Returns the field name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the field type.
    pub fn field_type(&self) -> &TypeName {
        &self.field_type
    }

    /// Returns the semantic field modifiers.
    pub fn modifiers(&self) -> &Modifiers {
        &self.modifiers
    }

    /// Returns the field documentation lines.
    pub fn doc(&self) -> &[String] {
        &self.doc
    }

    /// Returns the initializer expression, when present.
    pub fn initializer(&self) -> Option<&CodeBlock> {
        self.initializer.as_ref()
    }

    /// Returns opaque annotation blocks supplied through the escape hatch.
    pub fn annotations(&self) -> &[CodeBlock] {
        &self.annotations
    }

    /// Returns structured annotation declarations.
    pub fn annotation_specs(&self) -> &[AnnotationSpec] {
        &self.annotation_specs
    }

    /// Returns the target-specific struct tag, when present.
    pub fn tag(&self) -> Option<&str> {
        self.tag.as_deref()
    }

    /// Whether the field may be absent from the containing value.
    pub fn is_optional(&self) -> bool {
        self.is_optional
    }

    /// Emit this field as a CodeBlock.
    pub fn emit(
        &self,
        lang: &dyn CodeLang,
        ctx: DeclarationContext,
    ) -> Result<CodeBlock, SigilStitchError> {
        let fields = std::slice::from_ref(self);
        Self::lower_sequence(FieldSequenceIntent::direct(fields, ctx), lang)
    }

    pub(crate) fn collect_sequence_validation_errors(
        intent: FieldSequenceIntent<'_>,
        lang: &dyn CodeLang,
        errors: &mut Vec<SigilStitchError>,
    ) {
        Self::collect_sequence_intrinsic_validation_errors(intent, errors);
        Self::collect_sequence_target_validation_errors(intent, lang, errors);
    }

    pub(crate) fn collect_sequence_intrinsic_validation_errors(
        intent: FieldSequenceIntent<'_>,
        errors: &mut Vec<SigilStitchError>,
    ) {
        let context = intent.context();
        let mut seen_names = std::collections::HashSet::new();
        let mut reported_names = std::collections::HashSet::new();
        for field in intent.fields() {
            field.collect_intrinsic_validation_errors(context, errors);

            if !seen_names.insert(field.name())
                && reported_names.insert(field.name())
                && intent.variant_name().is_none()
            {
                errors.push(SigilStitchError::DuplicateFieldName {
                    type_name: intent.owner_name().unwrap_or("<direct fields>").to_string(),
                    field_name: field.name().to_string(),
                });
            }
        }
    }

    fn collect_sequence_target_validation_errors(
        intent: FieldSequenceIntent<'_>,
        lang: &dyn CodeLang,
        errors: &mut Vec<SigilStitchError>,
    ) {
        let context = intent.context();
        let capabilities = lang.capabilities();
        if !capabilities.supports_field_context(context) {
            errors.push(SigilStitchError::UnsupportedFieldContext {
                language: lang.file_extension().to_string(),
                context,
                owner_name: intent.owner_name().map(str::to_string),
            });
            return;
        }

        for field in intent.fields() {
            let requested = field.requested_capabilities();
            let unsupported: Vec<_> = requested
                .iter()
                .copied()
                .filter(|capability| !capabilities.supports_field_capability(context, *capability))
                .collect();
            if !unsupported.is_empty() {
                errors.push(SigilStitchError::UnsupportedFieldCapabilities {
                    language: lang.file_extension().to_string(),
                    field_name: field.name.clone(),
                    context,
                    capabilities: unsupported,
                });
            }

            let missing: Vec<_> = capabilities
                .required_field_capabilities(context)
                .iter()
                .copied()
                .filter(|capability| !requested.contains(capability))
                .collect();
            if !missing.is_empty() {
                errors.push(SigilStitchError::MissingRequiredFieldCapabilities {
                    language: lang.file_extension().to_string(),
                    field_name: field.name.clone(),
                    context,
                    capabilities: missing,
                });
            }
        }

        lang.collect_field_validation_errors(intent, errors);
    }

    pub(crate) fn collect_intrinsic_validation_errors(
        &self,
        context: FieldContext,
        errors: &mut Vec<SigilStitchError>,
    ) {
        if self.name.is_empty() {
            errors.push(SigilStitchError::EmptyName {
                builder: "FieldSpec",
            });
        }
        if self.initializer.as_ref().is_some_and(CodeBlock::is_empty) {
            errors.push(SigilStitchError::EmptyFieldOperand {
                field_name: self.name.clone(),
                context,
                operand: "initializer",
            });
        }

        let mut invalid_modifiers = Vec::new();
        if self.modifiers.is_abstract {
            invalid_modifiers.push("abstract");
        }
        if self.modifiers.is_async {
            invalid_modifiers.push("async");
        }
        if self.modifiers.is_override {
            invalid_modifiers.push("override");
        }
        if self.modifiers.is_constructor {
            invalid_modifiers.push("constructor");
        }
        if !invalid_modifiers.is_empty() {
            errors.push(SigilStitchError::InvalidFieldModifiers {
                field_name: self.name.clone(),
                context,
                modifiers: invalid_modifiers,
            });
        }
    }

    pub(crate) fn validate_sequence<'a>(
        intent: FieldSequenceIntent<'a>,
        lang: &dyn CodeLang,
    ) -> Result<ValidatedFields<'a>, SigilStitchError> {
        let mut errors = Vec::new();
        Self::collect_sequence_validation_errors(intent, lang, &mut errors);
        if let Some(error) = errors.into_iter().next() {
            return Err(error);
        }
        Ok(ValidatedFields::new(intent))
    }

    pub(crate) fn lower_sequence(
        intent: FieldSequenceIntent<'_>,
        lang: &dyn CodeLang,
    ) -> Result<CodeBlock, SigilStitchError> {
        let fields = Self::validate_sequence(intent, lang)?;
        lang.lower_fields(fields)
    }

    fn requested_capabilities(&self) -> Vec<FieldCapability> {
        let mut capabilities = Vec::new();
        if !self.field_type.is_empty() {
            capabilities.push(FieldCapability::ExplicitType);
        }
        if self.initializer.is_some() {
            capabilities.push(FieldCapability::Initializer);
        }
        if !self.annotations.is_empty() || !self.annotation_specs.is_empty() || self.tag.is_some() {
            capabilities.push(FieldCapability::Attributes);
        }
        if self.modifiers.is_static {
            capabilities.push(FieldCapability::StaticField);
        }
        if self.modifiers.is_readonly {
            capabilities.push(FieldCapability::ReadOnly);
        }
        if self.is_optional {
            capabilities.push(FieldCapability::OptionalPresence);
        }
        capabilities
    }
}

/// Builder for [`FieldSpec`].
#[derive(Debug)]
pub struct FieldSpecBuilder {
    name: String,
    field_type: TypeName,
    modifiers: Modifiers,
    doc: Vec<String>,
    initializer: Option<CodeBlock>,
    annotations: Vec<CodeBlock>,
    annotation_specs: Vec<AnnotationSpec>,
    tag: Option<String>,
    is_optional: bool,
}

impl FieldSpecBuilder {
    /// Set the visibility modifier.
    pub fn visibility(mut self, vis: Visibility) -> Self {
        self.modifiers.visibility = vis;
        self
    }

    /// Mark this field as static.
    pub fn is_static(mut self) -> Self {
        self.modifiers.is_static = true;
        self
    }

    /// Mark this field as readonly.
    pub fn is_readonly(mut self) -> Self {
        self.modifiers.is_readonly = true;
        self
    }

    /// Mark this field as optional (the key may be absent from the containing value).
    ///
    /// The selected language adapter must declare and lower this capability;
    /// unsupported contexts fail validation rather than dropping the intent.
    pub fn is_optional(mut self) -> Self {
        self.is_optional = true;
        self
    }

    /// Add a doc comment line.
    pub fn doc(mut self, line: &str) -> Self {
        self.doc.push(line.to_string());
        self
    }

    /// Set the field initializer expression.
    pub fn initializer(mut self, init: CodeBlock) -> Self {
        self.initializer = Some(init);
        self
    }

    /// Add a raw annotation [`CodeBlock`].
    pub fn annotation(mut self, ann: CodeBlock) -> Self {
        self.annotations.push(ann);
        self
    }

    /// Add a structured [`AnnotationSpec`].
    pub fn annotate(mut self, spec: AnnotationSpec) -> Self {
        self.annotation_specs.push(spec);
        self
    }

    /// Set the struct tag (e.g., Go's `` `json:"name"` ``).
    pub fn tag(mut self, t: &str) -> Self {
        self.tag = Some(t.to_string());
        self
    }

    /// Build the [`FieldSpec`] from this builder.
    ///
    /// # Errors
    ///
    /// Returns [`SigilStitchError::EmptyName`] if `name` is empty.
    pub fn build(self) -> Result<FieldSpec, crate::error::SigilStitchError> {
        snafu::ensure!(
            !self.name.is_empty(),
            crate::error::EmptyNameSnafu {
                builder: "FieldSpecBuilder",
            }
        );
        Ok(FieldSpec {
            name: self.name,
            field_type: self.field_type,
            modifiers: self.modifiers,
            doc: self.doc,
            initializer: self.initializer,
            annotations: self.annotations,
            annotation_specs: self.annotation_specs,
            tag: self.tag,
            is_optional: self.is_optional,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang::dart::Dart;

    fn field_languages() -> Vec<Box<dyn CodeLang>> {
        vec![
            Box::new(crate::lang::c::C::new()),
            Box::new(crate::lang::cpp::Cpp::new()),
            Box::new(crate::lang::csharp::CSharp::new()),
            Box::new(crate::lang::dart::Dart::new()),
            Box::new(crate::lang::go::Go::new()),
            Box::new(crate::lang::haskell::Haskell::new()),
            Box::new(crate::lang::java::Java::new()),
            Box::new(crate::lang::javascript::JavaScript::new()),
            Box::new(crate::lang::kotlin::Kotlin::new()),
            Box::new(crate::lang::ocaml::OCaml::new()),
            Box::new(crate::lang::php::Php::new()),
            Box::new(crate::lang::python::Python::new()),
            Box::new(crate::lang::rust::Rust::new()),
            Box::new(crate::lang::scala::Scala::new()),
            Box::new(crate::lang::swift::Swift::new()),
            Box::new(crate::lang::typescript::TypeScript::new()),
        ]
    }

    #[test]
    fn built_in_result_hook_matches_the_additive_field_collector() {
        let fields = ["first", "second"].map(|name| {
            FieldSpec::builder(name, TypeName::primitive("String"))
                .is_static()
                .is_readonly()
                .build()
                .unwrap()
        });
        let intent = FieldSequenceIntent::direct(&fields, DeclarationContext::Member);
        let lang = Dart::new();

        assert!(lang.validate_fields(intent).is_err());
        let mut errors = Vec::new();
        lang.collect_field_validation_errors(intent, &mut errors);
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn every_built_in_result_hook_accepts_valid_and_rejects_invalid_identifiers() {
        let valid_fields = [FieldSpec::of("valid", TypeName::primitive("Value"))];
        let valid = FieldSequenceIntent::direct(&valid_fields, DeclarationContext::Member);
        let mut invalid_field = FieldSpec::of("valid", TypeName::primitive("Value"));
        invalid_field.name = "bad name".to_string();
        let invalid_fields = [invalid_field];
        let invalid = FieldSequenceIntent::direct(&invalid_fields, DeclarationContext::Member);

        for lang in field_languages() {
            assert!(
                lang.validate_fields(valid).is_ok(),
                ".{} rejected a valid field",
                lang.file_extension()
            );
            assert!(
                lang.validate_fields(invalid).is_err(),
                ".{} accepted an invalid identifier",
                lang.file_extension()
            );
        }
    }

    #[test]
    fn every_built_in_lowers_a_valid_direct_field_sequence() {
        for lang in field_languages() {
            let field_type = if lang.file_extension() == "js" {
                TypeName::primitive("")
            } else {
                TypeName::primitive("Value")
            };
            let field = FieldSpec::of("valid", field_type);
            let output = field
                .emit(lang.as_ref(), DeclarationContext::Member)
                .and_then(|block| block.render_standalone(lang.as_ref(), 120))
                .unwrap_or_else(|error| {
                    panic!(
                        ".{} failed to lower a valid field: {error}",
                        lang.file_extension()
                    )
                });
            assert!(
                output.contains("valid"),
                ".{}: {output}",
                lang.file_extension()
            );
        }
    }
}
