//! Dart-owned field grammar.

#![deny(deprecated)]

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::lang::capability::{FieldCapability, FieldCapabilityProfile, FieldContext};
use crate::lang::dart::Dart;
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
const PAYLOAD_CAPABILITIES: &[FieldCapability] =
    &[FieldCapability::ExplicitType, FieldCapability::ReadOnly];
const PAYLOAD_REQUIRED: &[FieldCapability] = &[FieldCapability::ExplicitType];
pub(crate) const PROFILES: &[FieldCapabilityProfile] = &[
    FieldCapabilityProfile::new(
        FieldContext::Direct(DeclarationContext::Member),
        CAPABILITIES,
    ),
    FieldCapabilityProfile::new(FieldContext::TypeMember(TypeKind::Class), CAPABILITIES),
    FieldCapabilityProfile::new(FieldContext::TypeMember(TypeKind::Struct), CAPABILITIES),
    FieldCapabilityProfile::new(FieldContext::ClosedSumRecordPayload, PAYLOAD_CAPABILITIES)
        .with_required_capabilities(PAYLOAD_REQUIRED),
];

fn is_valid_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|ch| ch == '_' || ch == '$' || unicode_ident::is_xid_start(ch))
        && chars.all(|ch| ch == '$' || unicode_ident::is_xid_continue(ch))
}

fn has_implicit_null_initializer(field_type: &crate::type_name::TypeName) -> bool {
    use crate::type_name::TypeName;

    match field_type {
        TypeName::Optional(_) => true,
        TypeName::Primitive(name) | TypeName::Raw(name) => name.is_empty() || name == "dynamic",
        _ => false,
    }
}

pub(crate) fn validate(
    lang: &Dart,
    fields: FieldSequenceIntent<'_>,
) -> Result<(), SigilStitchError> {
    super::validation_result(|errors| collect_validation_errors(lang, fields, errors))
}

pub(crate) fn collect_validation_errors(
    lang: &Dart,
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
                reason: "Dart closed-sum case fields require undocumented inherited visibility"
                    .to_string(),
            });
        }
        let private = field.name().starts_with('_');
        let valid = match field.modifiers().visibility {
            Visibility::Inherited => true,
            Visibility::Public => !private,
            Visibility::Private => private,
            Visibility::Protected | Visibility::PublicCrate | Visibility::PublicSuper => false,
        };
        if !valid {
            errors.push(SigilStitchError::InvalidField {
                language: lang.file_extension().to_string(),
                field_name: field.name().to_string(),
                context: fields.context(),
                reason: "Dart field privacy is determined by a leading underscore".to_string(),
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
        if field.modifiers().is_static
            && field.initializer().is_none()
            && (field.modifiers().is_readonly || !has_implicit_null_initializer(field.field_type()))
        {
            errors.push(SigilStitchError::InvalidField {
                language: lang.file_extension().to_string(),
                field_name: field.name().to_string(),
                context: fields.context(),
                reason: "a Dart static field without an initializer must be mutable and nullable, dynamic, or untyped when late fields cannot be expressed".to_string(),
            });
        }
    }
}

pub(crate) fn lower(
    lang: &Dart,
    fields: ValidatedFields<'_>,
) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    if fields.context() == FieldContext::ClosedSumRecordPayload {
        for field in fields.fields() {
            block.add(
                "final %T %L;",
                (
                    field.field_type().clone(),
                    lang.escape_field_name(field.name()),
                ),
            );
            block.add_line();
        }
        return block.build();
    }
    for field in fields.fields() {
        emit_doc(&mut block, lang, field);
        emit_annotations(&mut block, field, "@", "")?;
        if field.modifiers().is_static {
            block.add("static ", ());
        }
        if field.modifiers().is_readonly {
            block.add("final ", ());
        } else if field.field_type().is_empty() {
            block.add("var ", ());
        }
        if !field.field_type().is_empty() {
            block.add("%T ", field.field_type().clone());
        }
        block.add("%L", lang.escape_field_name(field.name()));
        if let Some(initializer) = field.initializer() {
            block.add(" = %L", initializer.clone());
        }
        block.add(";", ());
        block.add_line();
    }
    block.build()
}
