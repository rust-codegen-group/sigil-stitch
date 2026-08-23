//! Rust-owned field grammar.

#![deny(deprecated)]

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::lang::capability::{FieldCapability, FieldCapabilityProfile, FieldContext};
use crate::lang::rust::Rust;
use crate::lang::{CodeLang, RendererLang};
use crate::spec::field_spec::{FieldSequenceIntent, ValidatedFields};
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};
use unicode_normalization::UnicodeNormalization;

use super::{collect_invalid_identifiers, collect_name_collisions_by, emit_annotations, emit_doc};

const CAPABILITIES: &[FieldCapability] =
    &[FieldCapability::ExplicitType, FieldCapability::Attributes];
const REQUIRED: &[FieldCapability] = &[FieldCapability::ExplicitType];
const PAYLOAD_CAPABILITIES: &[FieldCapability] =
    &[FieldCapability::ExplicitType, FieldCapability::Attributes];

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
        PAYLOAD_CAPABILITIES,
    )
    .with_required_capabilities(REQUIRED),
];

fn is_valid_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|ch| ch == '_' || unicode_ident::is_xid_start(ch))
        && chars.all(unicode_ident::is_xid_continue)
}

pub(crate) fn validate(
    lang: &Rust,
    fields: FieldSequenceIntent<'_>,
) -> Result<(), SigilStitchError> {
    super::validation_result(|errors| collect_validation_errors(lang, fields, errors))
}

pub(crate) fn collect_validation_errors(
    lang: &Rust,
    fields: FieldSequenceIntent<'_>,
    errors: &mut Vec<SigilStitchError>,
) {
    collect_invalid_identifiers(lang, fields, is_valid_identifier, errors);
    collect_name_collisions_by(
        lang,
        fields,
        |name| lang.escape_field_name(name).nfc().collect(),
        errors,
    );
    for field in fields.fields() {
        if matches!(field.name(), "_" | "self" | "Self" | "super" | "crate") {
            errors.push(SigilStitchError::InvalidField {
                language: lang.file_extension().to_string(),
                field_name: field.name().to_string(),
                context: fields.context(),
                reason: "Rust reserves this identifier in field position".to_string(),
            });
        }
        let visibility_is_valid =
            if matches!(fields.context(), FieldContext::VariantRecordPayload(_)) {
                field.modifiers().visibility == Visibility::Inherited
            } else {
                !matches!(field.modifiers().visibility, Visibility::Protected)
            };
        if !visibility_is_valid {
            errors.push(SigilStitchError::InvalidField {
                language: lang.file_extension().to_string(),
                field_name: field.name().to_string(),
                context: fields.context(),
                reason: "Rust fields do not support this visibility in the selected context"
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
    lang: &Rust,
    fields: ValidatedFields<'_>,
) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    let payload = matches!(fields.context(), FieldContext::VariantRecordPayload(_));
    let declaration_context = match fields.context() {
        FieldContext::Direct(context) => context,
        FieldContext::TypeMember(_) | FieldContext::VariantRecordPayload(_) => {
            DeclarationContext::Member
        }
    };
    for field in fields.fields() {
        emit_doc(&mut block, lang, field);
        emit_annotations(&mut block, field, "#[", "]")?;
        if !payload {
            block.add(
                "%L",
                lang.render_visibility(field.modifiers().visibility, declaration_context),
            );
        }
        block.add("%L: ", lang.escape_field_name(field.name()));
        block.add("%T", field.field_type().clone());
        block.add(",", ());
        block.add_line();
    }
    block.build()
}
