//! PHP-owned property grammar.

#![deny(deprecated)]

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::lang::capability::{FieldCapability, FieldCapabilityProfile, FieldContext};
use crate::lang::php::Php;
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

pub(crate) const PROFILES: &[FieldCapabilityProfile] = &[
    FieldCapabilityProfile::new(
        FieldContext::Direct(DeclarationContext::Member),
        CAPABILITIES,
    ),
    FieldCapabilityProfile::new(FieldContext::TypeMember(TypeKind::Class), CAPABILITIES),
    FieldCapabilityProfile::new(FieldContext::TypeMember(TypeKind::Struct), CAPABILITIES),
    FieldCapabilityProfile::new(FieldContext::TypeMember(TypeKind::Trait), CAPABILITIES),
];

pub(crate) fn is_valid_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|ch| ch == '_' || ch.is_ascii_alphabetic() || !ch.is_ascii())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric() || !ch.is_ascii())
}

pub(crate) fn validate(
    lang: &Php,
    fields: FieldSequenceIntent<'_>,
) -> Result<(), SigilStitchError> {
    super::validation_result(|errors| collect_validation_errors(lang, fields, errors))
}

pub(crate) fn collect_validation_errors(
    lang: &Php,
    fields: FieldSequenceIntent<'_>,
    errors: &mut Vec<SigilStitchError>,
) {
    collect_invalid_identifiers(lang, fields, is_valid_identifier, errors);
    collect_escaped_name_collisions(lang, fields, errors);
    for field in fields.fields() {
        if matches!(
            field.modifiers().visibility,
            Visibility::PublicCrate | Visibility::PublicSuper
        ) {
            errors.push(SigilStitchError::InvalidField {
                language: lang.file_extension().to_string(),
                field_name: field.name().to_string(),
                context: fields.context(),
                reason: "PHP properties do not support crate- or superclass-scoped visibility"
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
        if field.modifiers().is_readonly && field.field_type().is_empty() {
            errors.push(SigilStitchError::InvalidField {
                language: lang.file_extension().to_string(),
                field_name: field.name().to_string(),
                context: fields.context(),
                reason: "PHP readonly properties require an explicit type".to_string(),
            });
        }
        if field.modifiers().is_readonly && field.modifiers().is_static {
            errors.push(SigilStitchError::InvalidField {
                language: lang.file_extension().to_string(),
                field_name: field.name().to_string(),
                context: fields.context(),
                reason: "PHP readonly properties cannot be static".to_string(),
            });
        }
        if field.modifiers().is_readonly && field.initializer().is_some() {
            errors.push(SigilStitchError::InvalidField {
                language: lang.file_extension().to_string(),
                field_name: field.name().to_string(),
                context: fields.context(),
                reason: "PHP readonly properties cannot have declaration initializers".to_string(),
            });
        }
    }
}

fn visibility(visibility: Visibility) -> &'static str {
    match visibility {
        Visibility::Inherited | Visibility::Public => "public ",
        Visibility::Private => "private ",
        Visibility::Protected => "protected ",
        Visibility::PublicCrate | Visibility::PublicSuper => unreachable!(),
    }
}

pub(crate) fn lower(
    lang: &Php,
    fields: ValidatedFields<'_>,
) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    for field in fields.fields() {
        emit_doc(&mut block, lang, field);
        emit_annotations(&mut block, field, "#[", "]")?;
        block.add("%L", visibility(field.modifiers().visibility));
        if field.modifiers().is_static {
            block.add("static ", ());
        }
        if field.modifiers().is_readonly {
            block.add("readonly ", ());
        }
        if !field.field_type().is_empty() {
            block.add("%T ", field.field_type().clone());
        }
        block.add("$%L", lang.escape_field_name(field.name()));
        if let Some(initializer) = field.initializer() {
            block.add(" = %L", initializer.clone());
        }
        block.add(";", ());
        block.add_line();
    }
    block.build()
}
