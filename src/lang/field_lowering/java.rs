//! Java-owned field grammar.

#![deny(deprecated)]

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::lang::capability::{FieldCapability, FieldCapabilityProfile, FieldContext};
use crate::lang::java::Java;
use crate::lang::{CodeLang, RendererLang};
use crate::spec::field_spec::{FieldSequenceIntent, ValidatedFields};
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};

use super::{
    collect_escaped_name_collisions, collect_invalid_identifiers, emit_annotations, emit_doc,
};

const CAPABILITIES: &[FieldCapability] = &[
    FieldCapability::ExplicitType,
    FieldCapability::Initializer,
    FieldCapability::Attributes,
    FieldCapability::StaticField,
    FieldCapability::ReadOnly,
];
const REQUIRED: &[FieldCapability] = &[FieldCapability::ExplicitType];
const PAYLOAD_CAPABILITIES: &[FieldCapability] =
    &[FieldCapability::ExplicitType, FieldCapability::ReadOnly];

pub(crate) const PROFILES: &[FieldCapabilityProfile] = &[
    FieldCapabilityProfile::new(
        FieldContext::Direct(DeclarationContext::Member),
        CAPABILITIES,
    )
    .with_required_capabilities(REQUIRED),
    FieldCapabilityProfile::new(FieldContext::TypeMember(TypeKind::Class), CAPABILITIES)
        .with_required_capabilities(REQUIRED),
    FieldCapabilityProfile::new(FieldContext::TypeMember(TypeKind::Struct), CAPABILITIES)
        .with_required_capabilities(REQUIRED),
    FieldCapabilityProfile::new(FieldContext::TypeMember(TypeKind::Enum), CAPABILITIES)
        .with_required_capabilities(REQUIRED),
    FieldCapabilityProfile::new(FieldContext::ClosedSumRecordPayload, PAYLOAD_CAPABILITIES)
        .with_required_capabilities(REQUIRED),
];

fn is_valid_identifier(name: &str) -> bool {
    use unicode_general_category::{GeneralCategory, get_general_category};

    fn is_start(ch: char) -> bool {
        use GeneralCategory::*;
        matches!(
            get_general_category(ch),
            UppercaseLetter
                | LowercaseLetter
                | TitlecaseLetter
                | ModifierLetter
                | OtherLetter
                | LetterNumber
                | CurrencySymbol
                | ConnectorPunctuation
        )
    }

    fn is_continue(ch: char) -> bool {
        use GeneralCategory::*;
        is_start(ch)
            || matches!(
                get_general_category(ch),
                NonspacingMark | SpacingMark | DecimalNumber | Format
            )
    }

    let mut chars = name.chars();
    chars.next().is_some_and(|ch| ch == '$' || is_start(ch))
        && chars.all(|ch| ch == '$' || is_continue(ch))
}

pub(crate) fn validate(
    lang: &Java,
    fields: FieldSequenceIntent<'_>,
) -> Result<(), SigilStitchError> {
    super::validation_result(|errors| collect_validation_errors(lang, fields, errors))
}

pub(crate) fn collect_validation_errors(
    lang: &Java,
    fields: FieldSequenceIntent<'_>,
    errors: &mut Vec<SigilStitchError>,
) {
    collect_invalid_identifiers(lang, fields, is_valid_identifier, errors);
    collect_escaped_name_collisions(lang, fields, errors);
    for field in fields.fields() {
        if fields.context() == FieldContext::ClosedSumRecordPayload
            && (!field.doc().is_empty() || field.modifiers().visibility != Visibility::Inherited)
        {
            errors.push(SigilStitchError::InvalidField {
                language: lang.file_extension().to_string(),
                field_name: field.name().to_string(),
                context: fields.context(),
                reason: "Java record-case components require undocumented inherited visibility"
                    .to_string(),
            });
        }
        if field.name() == "_" {
            errors.push(SigilStitchError::InvalidField {
                language: lang.file_extension().to_string(),
                field_name: field.name().to_string(),
                context: fields.context(),
                reason: "Java reserves the single underscore identifier".to_string(),
            });
        }
        if matches!(
            field.modifiers().visibility,
            Visibility::PublicCrate | Visibility::PublicSuper
        ) {
            errors.push(SigilStitchError::InvalidField {
                language: lang.file_extension().to_string(),
                field_name: field.name().to_string(),
                context: fields.context(),
                reason: "Java fields do not support crate- or superclass-scoped visibility"
                    .to_string(),
            });
        }
        if field.tag().is_some() {
            errors.push(SigilStitchError::InvalidField {
                language: lang.file_extension().to_string(),
                field_name: field.name().to_string(),
                context: fields.context(),
                reason: "the legacy tag escape hatch is only valid for Go struct fields"
                    .to_string(),
            });
        }
    }
}

pub(crate) fn lower(
    lang: &Java,
    fields: ValidatedFields<'_>,
) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    if fields.context() == FieldContext::ClosedSumRecordPayload {
        for (index, field) in fields.fields().iter().enumerate() {
            if index > 0 {
                block.add(", ", ());
            }
            block.add(
                "%T %L",
                (
                    field.field_type().clone(),
                    lang.escape_field_name(field.name()),
                ),
            );
        }
        return block.build();
    }
    for field in fields.fields() {
        emit_doc(&mut block, lang, field);
        emit_annotations(&mut block, field, "@", "")?;
        block.add(
            "%L",
            lang.render_visibility(field.modifiers().visibility, DeclarationContext::Member),
        );
        if field.modifiers().is_static {
            block.add("static ", ());
        }
        if field.modifiers().is_readonly {
            block.add("final ", ());
        }
        block.add(
            "%T %L",
            (
                field.field_type().clone(),
                lang.escape_field_name(field.name()),
            ),
        );
        if let Some(initializer) = field.initializer() {
            block.add(" = %L", initializer.clone());
        }
        block.add(";", ());
        block.add_line();
    }
    block.build()
}
