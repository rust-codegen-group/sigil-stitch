//! Kotlin-owned property grammar.

#![deny(deprecated)]

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::lang::capability::{FieldCapability, FieldCapabilityProfile, FieldContext};
use crate::lang::kotlin::Kotlin;
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
    FieldCapability::ReadOnly,
];
pub(crate) const PROFILES: &[FieldCapabilityProfile] = &[
    FieldCapabilityProfile::new(
        FieldContext::Direct(DeclarationContext::Member),
        CAPABILITIES,
    ),
    FieldCapabilityProfile::new(FieldContext::TypeMember(TypeKind::Class), CAPABILITIES),
    FieldCapabilityProfile::new(FieldContext::TypeMember(TypeKind::Struct), CAPABILITIES),
    FieldCapabilityProfile::new(FieldContext::TypeMember(TypeKind::Enum), CAPABILITIES),
];

fn is_valid_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|ch| ch == '_' || unicode_ident::is_xid_start(ch))
        && chars.all(unicode_ident::is_xid_continue)
}

pub(crate) fn validate(
    lang: &Kotlin,
    fields: FieldSequenceIntent<'_>,
) -> Result<(), SigilStitchError> {
    super::validation_result(|errors| collect_validation_errors(lang, fields, errors))
}

pub(crate) fn collect_validation_errors(
    lang: &Kotlin,
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
                reason: "Kotlin reserves the single underscore identifier".to_string(),
            });
        }
        if field.modifiers().visibility == Visibility::PublicSuper {
            errors.push(SigilStitchError::InvalidField {
                language: lang.file_extension().to_string(),
                field_name: field.name().to_string(),
                context: fields.context(),
                reason: "Kotlin properties do not support superclass-scoped visibility".to_string(),
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
        if field.field_type().is_empty() && field.initializer().is_none() {
            errors.push(SigilStitchError::InvalidField {
                language: lang.file_extension().to_string(),
                field_name: field.name().to_string(),
                context: fields.context(),
                reason: "a Kotlin property requires a type annotation or initializer".to_string(),
            });
        }
    }
}

fn visibility(visibility: Visibility) -> &'static str {
    match visibility {
        Visibility::Inherited | Visibility::Public => "",
        Visibility::Private => "private ",
        Visibility::Protected => "protected ",
        Visibility::PublicCrate => "internal ",
        Visibility::PublicSuper => unreachable!(),
    }
}

pub(crate) fn lower(
    lang: &Kotlin,
    fields: ValidatedFields<'_>,
) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    for field in fields.fields() {
        emit_doc(&mut block, lang, field);
        emit_annotations(&mut block, field, "@", "")?;
        block.add("%L", visibility(field.modifiers().visibility));
        block.add(
            if field.modifiers().is_readonly {
                "val "
            } else {
                "var "
            },
            (),
        );
        block.add("%L", lang.escape_field_name(field.name()));
        if !field.field_type().is_empty() {
            block.add(": %T", field.field_type().clone());
        }
        if let Some(initializer) = field.initializer() {
            block.add(" = %L", initializer.clone());
        }
        block.add_line();
    }
    block.build()
}
