//! OCaml-owned record-field grammar.

#![deny(deprecated)]

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::lang::capability::{FieldCapability, FieldCapabilityProfile, FieldContext};
use crate::lang::ocaml::OCaml;
use crate::lang::{CodeLang, RendererLang};
use crate::spec::field_spec::{FieldSequenceIntent, ValidatedFields};
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};

use super::{collect_escaped_name_collisions, collect_invalid_identifiers, emit_doc};

const CAPABILITIES: &[FieldCapability] =
    &[FieldCapability::ExplicitType, FieldCapability::ReadOnly];
const REQUIRED: &[FieldCapability] = &[FieldCapability::ExplicitType];

pub(crate) const PROFILES: &[FieldCapabilityProfile] = &[
    FieldCapabilityProfile::new(
        FieldContext::Direct(DeclarationContext::Member),
        CAPABILITIES,
    )
    .with_required_capabilities(REQUIRED),
    FieldCapabilityProfile::new(FieldContext::TypeMember(TypeKind::Struct), CAPABILITIES)
        .with_required_capabilities(REQUIRED),
    FieldCapabilityProfile::new(FieldContext::TypeMember(TypeKind::Class), CAPABILITIES)
        .with_required_capabilities(REQUIRED),
    FieldCapabilityProfile::new(
        FieldContext::VariantRecordPayload(TypeKind::Enum),
        CAPABILITIES,
    )
    .with_required_capabilities(REQUIRED),
    FieldCapabilityProfile::new(FieldContext::ClosedSumRecordPayload, CAPABILITIES)
        .with_required_capabilities(REQUIRED),
];

fn is_valid_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|ch| ch == '_' || ch.is_lowercase())
        && chars.all(|ch| ch == '\'' || unicode_ident::is_xid_continue(ch))
}

pub(crate) fn validate(
    lang: &OCaml,
    fields: FieldSequenceIntent<'_>,
) -> Result<(), SigilStitchError> {
    super::validation_result(|errors| collect_validation_errors(lang, fields, errors))
}

pub(crate) fn collect_validation_errors(
    lang: &OCaml,
    fields: FieldSequenceIntent<'_>,
    errors: &mut Vec<SigilStitchError>,
) {
    collect_invalid_identifiers(lang, fields, is_valid_identifier, errors);
    collect_escaped_name_collisions(lang, fields, errors);
    for field in fields.fields() {
        if field.name() == "_" {
            errors.push(SigilStitchError::InvalidField {
                language: lang.file_extension().to_string(),
                field_name: field.name().to_string(),
                context: fields.context(),
                reason: "OCaml record fields require a named label".to_string(),
            });
        }
        if field.modifiers().visibility != Visibility::Inherited {
            errors.push(SigilStitchError::InvalidField {
                language: lang.file_extension().to_string(),
                field_name: field.name().to_string(),
                context: fields.context(),
                reason: "OCaml record fields do not have declaration visibility modifiers"
                    .to_string(),
            });
        }
        if matches!(
            fields.context(),
            FieldContext::VariantRecordPayload(_) | FieldContext::ClosedSumRecordPayload
        ) && !field.doc().is_empty()
        {
            errors.push(SigilStitchError::InvalidField {
                language: lang.file_extension().to_string(),
                field_name: field.name().to_string(),
                context: fields.context(),
                reason:
                    "inline-record payload fields do not support standalone documentation blocks"
                        .to_string(),
            });
        }
    }
}

pub(crate) fn lower(
    lang: &OCaml,
    fields: ValidatedFields<'_>,
) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    let payload = matches!(
        fields.context(),
        FieldContext::VariantRecordPayload(_) | FieldContext::ClosedSumRecordPayload
    );
    for (index, field) in fields.fields().iter().enumerate() {
        if !payload {
            emit_doc(&mut block, lang, field);
        }
        if index > 0 && payload {
            block.add("; ", ());
        }
        block.add(
            "%L : %T",
            (
                lang.escape_field_name(field.name()),
                field.field_type().clone(),
            ),
        );
        if !payload {
            block.add(";", ());
            block.add_line();
        }
    }
    block.build()
}
