//! Dedicated closed-sum type declarations.

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::lang::CodeLang;
use crate::lang::capability::ClosedSumCaseForm;
use crate::spec::annotation_spec::{AnnotationNameRef, AnnotationSpec};
use crate::spec::closed_sum_case_spec::ClosedSumCaseSpec;
use crate::spec::field_spec::ValidatedFields;
use crate::spec::modifiers::{Modifiers, Visibility};
use crate::spec::where_spec::{TypeParamSpec, WhereConstraint};
use crate::type_name::TypeName;

/// Read-only semantic intent for a complete ClosedSum declaration.
#[derive(Debug, Clone, Copy)]
pub struct ClosedSumIntent<'a> {
    spec: &'a ClosedSumSpec,
}

impl<'a> ClosedSumIntent<'a> {
    /// Declaration name.
    pub fn name(self) -> &'a str {
        &self.spec.name
    }
    /// Declaration modifiers.
    pub fn modifiers(self) -> &'a Modifiers {
        &self.spec.modifiers
    }
    /// Declaration visibility.
    pub fn visibility(self) -> Visibility {
        self.spec.modifiers.visibility
    }
    /// Documentation lines.
    pub fn doc(self) -> &'a [String] {
        &self.spec.doc
    }
    /// Type parameters.
    pub fn type_params(self) -> &'a [TypeParamSpec] {
        &self.spec.type_params
    }
    /// Explicit declaration constraints.
    pub fn where_constraints(self) -> &'a [WhereConstraint] {
        &self.spec.where_constraints
    }
    /// Opaque root annotations.
    pub fn annotations(self) -> &'a [CodeBlock] {
        &self.spec.annotations
    }
    /// Structured root annotations.
    pub fn annotation_specs(self) -> &'a [AnnotationSpec] {
        &self.spec.annotation_specs
    }
    /// Ordered named cases.
    pub fn cases(self) -> &'a [ClosedSumCaseSpec] {
        &self.spec.cases
    }
    /// Whether this declaration has no cases.
    pub fn is_empty(self) -> bool {
        self.spec.cases.is_empty()
    }
}

/// A case view supplied to a complete ClosedSum lowerer.
#[derive(Debug, Clone, Copy)]
pub struct ClosedSumCaseIntent<'a> {
    case: &'a ClosedSumCaseSpec,
}

impl<'a> ClosedSumCaseIntent<'a> {
    /// Case name.
    pub fn name(self) -> &'a str {
        self.case.name()
    }
    /// Case form.
    pub fn form(self) -> ClosedSumCaseForm {
        self.case.form()
    }
    /// Documentation lines.
    pub fn doc(self) -> &'a [String] {
        self.case.doc()
    }
    /// Positional payload types.
    pub fn positional_payload(self) -> &'a [crate::type_name::TypeName] {
        self.case.positional_payload()
    }
    /// Record payload fields.
    pub fn record_payload(self) -> &'a [crate::spec::field_spec::FieldSpec] {
        self.case.record_payload()
    }
    /// Opaque annotations.
    pub fn annotations(self) -> &'a [CodeBlock] {
        self.case.annotations()
    }
    /// Structured annotations.
    pub fn annotation_specs(self) -> &'a [AnnotationSpec] {
        self.case.annotation_specs()
    }
}

/// A complete ClosedSum declaration whose validation succeeded.
#[derive(Debug, Clone)]
pub struct ValidatedClosedSum<'a> {
    intent: ClosedSumIntent<'a>,
    cases: Vec<ValidatedClosedSumCase<'a>>,
}

/// A case whose payload validation succeeded for the selected language.
#[derive(Debug, Clone)]
pub struct ValidatedClosedSumCase<'a> {
    intent: ClosedSumCaseIntent<'a>,
    record_fields: Option<ValidatedFields<'a>>,
}

impl<'a> ValidatedClosedSumCase<'a> {
    pub(crate) fn intent(&self) -> ClosedSumCaseIntent<'a> {
        self.intent
    }
    /// Case name.
    pub fn name(&self) -> &'a str {
        self.intent.name()
    }
    /// Case form.
    pub fn form(&self) -> ClosedSumCaseForm {
        self.intent.form()
    }
    /// Documentation lines.
    pub fn doc(&self) -> &'a [String] {
        self.intent.doc()
    }
    /// Positional payload types.
    pub fn positional_payload(&self) -> &'a [crate::type_name::TypeName] {
        self.intent.positional_payload()
    }
    /// Validated record payload fields, when this is a record case.
    pub fn record_payload(&self) -> Option<&ValidatedFields<'a>> {
        self.record_fields.as_ref()
    }
    /// Opaque annotations.
    pub fn annotations(&self) -> &'a [CodeBlock] {
        self.intent.annotations()
    }
    /// Structured annotations.
    pub fn annotation_specs(&self) -> &'a [AnnotationSpec] {
        self.intent.annotation_specs()
    }
}

impl<'a> ValidatedClosedSum<'a> {
    pub(crate) fn intent(&self) -> ClosedSumIntent<'a> {
        self.intent
    }
    /// Declaration name.
    pub fn name(&self) -> &'a str {
        self.intent.name()
    }
    /// Declaration modifiers.
    pub fn modifiers(&self) -> &'a Modifiers {
        self.intent.modifiers()
    }
    /// Declaration visibility.
    pub fn visibility(&self) -> Visibility {
        self.intent.visibility()
    }
    /// Documentation lines.
    pub fn doc(&self) -> &'a [String] {
        self.intent.doc()
    }
    /// Type parameters.
    pub fn type_params(&self) -> &'a [TypeParamSpec] {
        self.intent.type_params()
    }
    /// Explicit declaration constraints.
    pub fn where_constraints(&self) -> &'a [WhereConstraint] {
        self.intent.where_constraints()
    }
    /// Opaque root annotations.
    pub fn annotations(&self) -> &'a [CodeBlock] {
        self.intent.annotations()
    }
    /// Structured root annotations.
    pub fn annotation_specs(&self) -> &'a [AnnotationSpec] {
        self.intent.annotation_specs()
    }
    /// Ordered validated case views.
    pub fn cases(&self) -> impl Iterator<Item = ValidatedClosedSumCase<'a>> + '_ {
        self.cases.iter().cloned()
    }
}

/// A complete ClosedSum declaration with named cases and no ordinary enum mode.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClosedSumSpec {
    pub(crate) name: String,
    pub(crate) modifiers: Modifiers,
    pub(crate) doc: Vec<String>,
    pub(crate) type_params: Vec<TypeParamSpec>,
    pub(crate) where_constraints: Vec<WhereConstraint>,
    pub(crate) annotations: Vec<CodeBlock>,
    pub(crate) annotation_specs: Vec<AnnotationSpec>,
    pub(crate) cases: Vec<ClosedSumCaseSpec>,
}

impl ClosedSumSpec {
    /// Start building a ClosedSum declaration.
    pub fn builder(name: &str) -> ClosedSumSpecBuilder {
        ClosedSumSpecBuilder {
            name: name.to_string(),
            modifiers: Modifiers::default(),
            doc: Vec::new(),
            type_params: Vec::new(),
            where_constraints: Vec::new(),
            annotations: Vec::new(),
            annotation_specs: Vec::new(),
            cases: Vec::new(),
        }
    }

    /// Declaration name.
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Ordered cases.
    pub fn cases(&self) -> &[ClosedSumCaseSpec] {
        &self.cases
    }
    /// Read-only semantic intent.
    pub fn intent(&self) -> ClosedSumIntent<'_> {
        ClosedSumIntent { spec: self }
    }

    fn collect_intrinsic_validation_errors(&self, errors: &mut Vec<SigilStitchError>) {
        if self.name.is_empty() {
            errors.push(SigilStitchError::EmptyName {
                builder: "ClosedSumSpec",
            });
        }
        let mut invalid_modifiers = Vec::new();
        if self.modifiers.is_static {
            invalid_modifiers.push("static");
        }
        if self.modifiers.is_abstract {
            invalid_modifiers.push("abstract");
        }
        if self.modifiers.is_readonly {
            invalid_modifiers.push("readonly");
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
            errors.push(SigilStitchError::InvalidTypeModifiers {
                type_name: self.name.clone(),
                modifiers: invalid_modifiers,
            });
        }
        for (index, annotation) in self.annotations.iter().enumerate() {
            if annotation.is_empty() {
                errors.push(SigilStitchError::InvalidTypeDeclaration {
                    type_name: self.name.clone(),
                    reason: format!("opaque annotation {index} is empty"),
                });
            }
        }
        for (index, annotation) in self.annotation_specs.iter().enumerate() {
            let name_is_empty = match annotation.name() {
                AnnotationNameRef::Simple(name) => name.is_empty(),
                AnnotationNameRef::Importable(type_name) => type_name.is_empty(),
            };
            if name_is_empty {
                errors.push(SigilStitchError::InvalidTypeDeclaration {
                    type_name: self.name.clone(),
                    reason: format!("structured annotation {index} has an empty name"),
                });
            }
        }
        let mut seen_type_params = std::collections::HashSet::new();
        let mut reported_type_params = std::collections::HashSet::new();
        for parameter in &self.type_params {
            if parameter.name().is_empty() {
                errors.push(SigilStitchError::InvalidTypeParameter {
                    type_name: self.name.clone(),
                    parameter_name: String::new(),
                    reason: "parameter name is empty".to_string(),
                });
            }
            if !seen_type_params.insert(parameter.name())
                && reported_type_params.insert(parameter.name())
            {
                errors.push(SigilStitchError::DuplicateTypeParameterName {
                    type_name: self.name.clone(),
                    parameter_name: parameter.name().to_string(),
                });
            }
            if parameter.bounds().iter().any(TypeName::is_empty)
                || parameter.context_bounds().iter().any(TypeName::is_empty)
            {
                errors.push(SigilStitchError::InvalidTypeParameter {
                    type_name: self.name.clone(),
                    parameter_name: parameter.name().to_string(),
                    reason: "bounds must not contain an empty type".to_string(),
                });
            }
        }
        for constraint in &self.where_constraints {
            if constraint.subject().is_empty() || constraint.bounds().is_empty() {
                errors.push(SigilStitchError::InvalidTypeParameter {
                    type_name: self.name.clone(),
                    parameter_name: format!("{:?}", constraint.subject()),
                    reason: "where constraints require a non-empty subject and at least one bound"
                        .to_string(),
                });
            } else if constraint.bounds().iter().any(TypeName::is_empty) {
                errors.push(SigilStitchError::InvalidTypeParameter {
                    type_name: self.name.clone(),
                    parameter_name: format!("{:?}", constraint.subject()),
                    reason: "where-constraint bounds must not contain an empty type".to_string(),
                });
            }
        }
        let mut seen = std::collections::HashSet::new();
        let mut reported = std::collections::HashSet::new();
        for case in &self.cases {
            case.collect_intrinsic_validation_errors(errors);
        }
        for case in &self.cases {
            if !seen.insert(case.name()) && reported.insert(case.name()) {
                errors.push(SigilStitchError::DuplicateClosedSumCaseName {
                    type_name: self.name.clone(),
                    case_name: case.name().to_string(),
                });
            }
        }
    }

    pub(crate) fn collect_validation_errors(
        &self,
        lang: &dyn CodeLang,
        errors: &mut Vec<SigilStitchError>,
    ) {
        self.collect_intrinsic_validation_errors(errors);
        let capabilities = lang.capabilities();
        let Some(profile) = capabilities.closed_sum_profile() else {
            errors.push(SigilStitchError::UnsupportedClosedSum {
                language: lang.file_extension().to_string(),
                type_name: self.name.clone(),
            });
            return;
        };

        let missing: Vec<_> = super::type_declaration::requested_capabilities(
            &self.type_params,
            &self.where_constraints,
            !self.annotations.is_empty() || !self.annotation_specs.is_empty(),
        )
        .into_iter()
        .filter(|capability| !profile.supports_declaration_capability(*capability))
        .collect();
        if !missing.is_empty() {
            errors.push(SigilStitchError::UnsupportedTypeDeclarationCapabilities {
                language: lang.file_extension().to_string(),
                type_name: self.name.clone(),
                capabilities: missing,
            });
        }
        if self.cases.is_empty() && !profile.supports_empty_sum() {
            errors.push(SigilStitchError::UnsupportedEmptyClosedSum {
                language: lang.file_extension().to_string(),
                type_name: self.name.clone(),
            });
        }
        for case in &self.cases {
            if !profile.supports_case_form(case.form()) {
                errors.push(SigilStitchError::UnsupportedClosedSumCaseForm {
                    language: lang.file_extension().to_string(),
                    type_name: self.name.clone(),
                    case_name: case.name().to_string(),
                    form: case.form(),
                });
            }
        }
        for case in &self.cases {
            if case.form() == ClosedSumCaseForm::RecordPayload
                && profile.supports_case_form(case.form())
            {
                let fields =
                    crate::spec::field_spec::FieldSequenceIntent::closed_sum_record_payload(
                        case.record_payload(),
                        &self.name,
                        case.name(),
                    );
                crate::spec::field_spec::FieldSpec::collect_sequence_target_validation_errors(
                    fields, lang, errors,
                );
            }
        }
        lang.collect_closed_sum_validation_errors(self.intent(), errors);
    }

    pub(crate) fn validate_complete<'a>(
        &'a self,
        lang: &dyn CodeLang,
    ) -> Result<ValidatedClosedSum<'a>, SigilStitchError> {
        let mut errors = Vec::new();
        self.collect_validation_errors(lang, &mut errors);
        if let Some(error) = errors.into_iter().next() {
            return Err(error);
        }
        let cases = self
            .cases
            .iter()
            .map(|case| {
                let record_fields = (case.form() == ClosedSumCaseForm::RecordPayload).then(|| {
                    ValidatedFields::new(
                        crate::spec::field_spec::FieldSequenceIntent::closed_sum_record_payload(
                            case.record_payload(),
                            &self.name,
                            case.name(),
                        ),
                    )
                });
                ValidatedClosedSumCase {
                    intent: ClosedSumCaseIntent { case },
                    record_fields,
                }
            })
            .collect();
        Ok(ValidatedClosedSum {
            intent: self.intent(),
            cases,
        })
    }

    /// Validate this declaration against a language adapter.
    pub fn validate(&self, lang: &dyn CodeLang) -> Result<(), SigilStitchError> {
        let mut errors = Vec::new();
        self.collect_validation_errors(lang, &mut errors);
        errors.into_iter().next().map_or(Ok(()), Err)
    }

    /// Emit this declaration through the complete ClosedSum lowering seam.
    pub fn emit(&self, lang: &dyn CodeLang) -> Result<Vec<CodeBlock>, SigilStitchError> {
        let validated = self.validate_complete(lang)?;
        let blocks = lang.lower_closed_sum(validated)?;
        if blocks.is_empty() {
            return Err(SigilStitchError::EmptyClosedSumLowering {
                language: lang.file_extension().to_string(),
                type_name: self.name.clone(),
                block_index: None,
            });
        }
        if let Some((index, _)) = blocks
            .iter()
            .enumerate()
            .find(|(_, block)| block.is_empty())
        {
            return Err(SigilStitchError::EmptyClosedSumLowering {
                language: lang.file_extension().to_string(),
                type_name: self.name.clone(),
                block_index: Some(index),
            });
        }
        Ok(blocks)
    }
}

impl crate::spec::emittable::Emittable for ClosedSumSpec {
    fn collect_validation_errors(&self, lang: &dyn CodeLang, errors: &mut Vec<SigilStitchError>) {
        self.collect_validation_errors(lang, errors);
    }

    fn emit_members(&self, lang: &dyn CodeLang) -> Result<Vec<CodeBlock>, SigilStitchError> {
        self.emit(lang)
    }
}

/// Builder for [`ClosedSumSpec`].
#[derive(Debug)]
pub struct ClosedSumSpecBuilder {
    name: String,
    modifiers: Modifiers,
    doc: Vec<String>,
    type_params: Vec<TypeParamSpec>,
    where_constraints: Vec<WhereConstraint>,
    annotations: Vec<CodeBlock>,
    annotation_specs: Vec<AnnotationSpec>,
    cases: Vec<ClosedSumCaseSpec>,
}

impl ClosedSumSpecBuilder {
    /// Set declaration visibility.
    pub fn visibility(mut self, visibility: Visibility) -> Self {
        self.modifiers.visibility = visibility;
        self
    }
    /// Add a documentation line.
    pub fn doc(mut self, line: &str) -> Self {
        self.doc.push(line.to_string());
        self
    }
    /// Add a type parameter.
    pub fn add_type_param(mut self, parameter: TypeParamSpec) -> Self {
        self.type_params.push(parameter);
        self
    }
    /// Add a where constraint.
    pub fn add_where_constraint(
        mut self,
        subject: crate::type_name::TypeName,
        bounds: Vec<crate::type_name::TypeName>,
    ) -> Self {
        self.where_constraints
            .push(WhereConstraint { subject, bounds });
        self
    }
    /// Add an opaque annotation.
    pub fn annotation(mut self, annotation: CodeBlock) -> Self {
        self.annotations.push(annotation);
        self
    }
    /// Add a structured annotation.
    pub fn annotate(mut self, annotation: AnnotationSpec) -> Self {
        self.annotation_specs.push(annotation);
        self
    }
    /// Add one ordered case.
    pub fn add_case(mut self, case: ClosedSumCaseSpec) -> Self {
        self.cases.push(case);
        self
    }
    /// Build the declaration.
    pub fn build(self) -> Result<ClosedSumSpec, SigilStitchError> {
        if self.name.is_empty() {
            return Err(SigilStitchError::EmptyName {
                builder: "ClosedSumSpecBuilder",
            });
        }
        Ok(ClosedSumSpec {
            name: self.name,
            modifiers: self.modifiers,
            doc: self.doc,
            type_params: self.type_params,
            where_constraints: self.where_constraints,
            annotations: self.annotations,
            annotation_specs: self.annotation_specs,
            cases: self.cases,
        })
    }
}
