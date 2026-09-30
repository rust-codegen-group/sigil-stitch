//! Dedicated semantic cases for [`ClosedSumSpec`](crate::spec::closed_sum_spec::ClosedSumSpec).

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::lang::capability::ClosedSumCaseForm;
use crate::spec::annotation_spec::AnnotationSpec;
use crate::spec::field_spec::FieldSpec;
use crate::type_name::TypeName;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
enum ClosedSumCaseData {
    Unit,
    Positional(Vec<TypeName>),
    Record(Vec<FieldSpec>),
}

/// A named unit, positional, or record case in a closed sum.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClosedSumCaseSpec {
    name: String,
    doc: Vec<String>,
    annotations: Vec<CodeBlock>,
    annotation_specs: Vec<AnnotationSpec>,
    data: ClosedSumCaseData,
}

impl ClosedSumCaseSpec {
    /// Create a unit case.
    pub fn new(name: &str) -> Result<Self, SigilStitchError> {
        Self::unit(name)
    }

    /// Create a unit case.
    pub fn unit(name: &str) -> Result<Self, SigilStitchError> {
        Self::builder(name).build()
    }

    /// Create a positional-payload case.
    pub fn positional(name: &str, payload: Vec<TypeName>) -> Result<Self, SigilStitchError> {
        if payload.is_empty() {
            return Err(SigilStitchError::InvalidClosedSumCasePayload {
                case_name: name.to_string(),
                form: ClosedSumCaseForm::PositionalPayload,
                reason: "positional payload must not be empty".to_string(),
            });
        }
        let mut builder = Self::builder(name);
        for ty in payload {
            builder = builder.positional_payload(ty);
        }
        builder.build()
    }

    /// Create a record-payload case.
    pub fn record(name: &str, fields: Vec<FieldSpec>) -> Result<Self, SigilStitchError> {
        if fields.is_empty() {
            return Err(SigilStitchError::InvalidClosedSumCasePayload {
                case_name: name.to_string(),
                form: ClosedSumCaseForm::RecordPayload,
                reason: "record payload must not be empty".to_string(),
            });
        }
        let mut builder = Self::builder(name);
        for field in fields {
            builder = builder.record_field(field);
        }
        builder.build()
    }

    /// Start building a case.
    pub fn builder(name: &str) -> ClosedSumCaseSpecBuilder {
        ClosedSumCaseSpecBuilder {
            name: name.to_string(),
            doc: Vec::new(),
            annotations: Vec::new(),
            annotation_specs: Vec::new(),
            data: ClosedSumCaseData::Unit,
            form_conflict: None,
        }
    }

    /// Case name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Case form.
    pub fn form(&self) -> ClosedSumCaseForm {
        match self.data {
            ClosedSumCaseData::Unit => ClosedSumCaseForm::Unit,
            ClosedSumCaseData::Positional(_) => ClosedSumCaseForm::PositionalPayload,
            ClosedSumCaseData::Record(_) => ClosedSumCaseForm::RecordPayload,
        }
    }

    /// Documentation lines.
    pub fn doc(&self) -> &[String] {
        &self.doc
    }

    /// Positional payload types, when this is a positional case.
    pub fn positional_payload(&self) -> &[TypeName] {
        match &self.data {
            ClosedSumCaseData::Positional(payload) => payload,
            _ => &[],
        }
    }

    /// Record payload fields, when this is a record case.
    pub fn record_payload(&self) -> &[FieldSpec] {
        match &self.data {
            ClosedSumCaseData::Record(fields) => fields,
            _ => &[],
        }
    }

    /// Opaque annotations.
    pub fn annotations(&self) -> &[CodeBlock] {
        &self.annotations
    }

    /// Structured annotations.
    pub fn annotation_specs(&self) -> &[AnnotationSpec] {
        &self.annotation_specs
    }

    pub(crate) fn collect_intrinsic_validation_errors(&self, errors: &mut Vec<SigilStitchError>) {
        if self.name.is_empty() {
            errors.push(SigilStitchError::EmptyName {
                builder: "ClosedSumCaseSpec",
            });
        }
        let form = self.form();
        match &self.data {
            ClosedSumCaseData::Unit => {}
            ClosedSumCaseData::Positional(payload) => {
                if payload.is_empty() {
                    errors.push(SigilStitchError::InvalidClosedSumCasePayload {
                        case_name: self.name.clone(),
                        form,
                        reason: "positional payload must not be empty".to_string(),
                    });
                }
                for (index, ty) in payload.iter().enumerate() {
                    if ty.is_empty() {
                        errors.push(SigilStitchError::InvalidClosedSumCasePayload {
                            case_name: self.name.clone(),
                            form,
                            reason: format!("positional payload type {index} is empty"),
                        });
                    }
                }
            }
            ClosedSumCaseData::Record(fields) => {
                if fields.is_empty() {
                    errors.push(SigilStitchError::InvalidClosedSumCasePayload {
                        case_name: self.name.clone(),
                        form,
                        reason: "record payload must not be empty".to_string(),
                    });
                }
                let mut seen = std::collections::HashSet::new();
                let mut reported = std::collections::HashSet::new();
                for field in fields {
                    if !seen.insert(field.name()) && reported.insert(field.name()) {
                        errors.push(SigilStitchError::DuplicateClosedSumRecordFieldName {
                            case_name: self.name.clone(),
                            field_name: field.name().to_string(),
                        });
                    }
                    field.collect_intrinsic_validation_errors(
                        crate::lang::capability::FieldContext::ClosedSumRecordPayload,
                        errors,
                    );
                    if field.field_type().is_empty() {
                        errors.push(SigilStitchError::InvalidClosedSumCasePayload {
                            case_name: self.name.clone(),
                            form,
                            reason: format!("record field {:?} has an empty type", field.name()),
                        });
                    }
                }
            }
        }
        for (index, annotation) in self.annotations.iter().enumerate() {
            if annotation.is_empty() {
                errors.push(SigilStitchError::InvalidClosedSumCasePayload {
                    case_name: self.name.clone(),
                    form,
                    reason: format!("opaque annotation {index} is empty"),
                });
            }
        }
        for (index, annotation) in self.annotation_specs.iter().enumerate() {
            let name_is_empty = match annotation.name() {
                crate::spec::annotation_spec::AnnotationNameRef::Simple(name) => name.is_empty(),
                crate::spec::annotation_spec::AnnotationNameRef::Importable(type_name) => {
                    type_name.is_empty()
                }
            };
            if name_is_empty {
                errors.push(SigilStitchError::InvalidClosedSumCasePayload {
                    case_name: self.name.clone(),
                    form,
                    reason: format!("structured annotation {index} has an empty name"),
                });
            }
        }
    }
}

/// Builder for [`ClosedSumCaseSpec`].
#[derive(Debug)]
pub struct ClosedSumCaseSpecBuilder {
    name: String,
    doc: Vec<String>,
    annotations: Vec<CodeBlock>,
    annotation_specs: Vec<AnnotationSpec>,
    data: ClosedSumCaseData,
    form_conflict: Option<(ClosedSumCaseForm, ClosedSumCaseForm)>,
}

impl ClosedSumCaseSpecBuilder {
    /// Add a documentation line.
    pub fn doc(mut self, line: &str) -> Self {
        self.doc.push(line.to_string());
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

    /// Select a positional payload and append one type.
    pub fn positional_payload(mut self, ty: TypeName) -> Self {
        match &mut self.data {
            ClosedSumCaseData::Unit => self.data = ClosedSumCaseData::Positional(vec![ty]),
            ClosedSumCaseData::Positional(payload) => payload.push(ty),
            ClosedSumCaseData::Record(_) => {
                self.form_conflict.get_or_insert((
                    ClosedSumCaseForm::RecordPayload,
                    ClosedSumCaseForm::PositionalPayload,
                ));
            }
        }
        self
    }

    /// Select a record payload and append one field.
    pub fn record_field(mut self, field: FieldSpec) -> Self {
        match &mut self.data {
            ClosedSumCaseData::Unit => self.data = ClosedSumCaseData::Record(vec![field]),
            ClosedSumCaseData::Record(fields) => fields.push(field),
            ClosedSumCaseData::Positional(_) => {
                self.form_conflict.get_or_insert((
                    ClosedSumCaseForm::PositionalPayload,
                    ClosedSumCaseForm::RecordPayload,
                ));
            }
        }
        self
    }

    /// Build the case.
    pub fn build(self) -> Result<ClosedSumCaseSpec, SigilStitchError> {
        if self.name.is_empty() {
            return Err(SigilStitchError::EmptyName {
                builder: "ClosedSumCaseSpecBuilder",
            });
        }
        if let Some((existing, requested)) = self.form_conflict {
            return Err(SigilStitchError::InvalidClosedSumCasePayload {
                case_name: self.name,
                form: requested,
                reason: format!("cannot combine {existing:?} and {requested:?} payload builders"),
            });
        }
        let spec = ClosedSumCaseSpec {
            name: self.name,
            doc: self.doc,
            annotations: self.annotations,
            annotation_specs: self.annotation_specs,
            data: self.data,
        };
        let mut errors = Vec::new();
        spec.collect_intrinsic_validation_errors(&mut errors);
        if let Some(error) = errors.into_iter().next() {
            return Err(error);
        }
        Ok(spec)
    }
}
