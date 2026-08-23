//! Go-owned struct-field grammar.

#![deny(deprecated)]

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::lang::capability::{FieldCapability, FieldCapabilityProfile, FieldContext};
use crate::lang::go::Go;
use crate::lang::{CodeLang, RendererLang};
use crate::spec::field_spec::{FieldSequenceIntent, ValidatedFields};
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};

use super::{collect_escaped_name_collisions, collect_invalid_identifiers, emit_doc};

const CAPABILITIES: &[FieldCapability] =
    &[FieldCapability::ExplicitType, FieldCapability::Attributes];
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
];

fn is_valid_identifier(name: &str) -> bool {
    use unicode_general_category::{GeneralCategory, get_general_category};

    fn is_letter(ch: char) -> bool {
        use GeneralCategory::*;
        matches!(
            get_general_category(ch),
            UppercaseLetter | LowercaseLetter | TitlecaseLetter | ModifierLetter | OtherLetter
        )
    }

    let mut chars = name.chars();
    chars.next().is_some_and(|ch| ch == '_' || is_letter(ch))
        && chars.all(|ch| {
            ch == '_' || is_letter(ch) || get_general_category(ch) == GeneralCategory::DecimalNumber
        })
}

pub(crate) fn validate(lang: &Go, fields: FieldSequenceIntent<'_>) -> Result<(), SigilStitchError> {
    super::validation_result(|errors| collect_validation_errors(lang, fields, errors))
}

pub(crate) fn collect_validation_errors(
    lang: &Go,
    fields: FieldSequenceIntent<'_>,
    errors: &mut Vec<SigilStitchError>,
) {
    collect_invalid_identifiers(lang, fields, is_valid_identifier, errors);
    collect_escaped_name_collisions(lang, fields, errors);
    for field in fields.fields() {
        let exported = field.name().chars().next().is_some_and(char::is_uppercase);
        let visibility_is_valid = match field.modifiers().visibility {
            Visibility::Inherited => true,
            Visibility::Public => exported,
            Visibility::Private => !exported,
            Visibility::Protected | Visibility::PublicCrate | Visibility::PublicSuper => false,
        };
        if !visibility_is_valid {
            errors.push(SigilStitchError::InvalidField {
                language: lang.file_extension().to_string(),
                field_name: field.name().to_string(),
                context: fields.context(),
                reason: "Go field visibility is determined by the identifier's initial letter"
                    .to_string(),
            });
        }
        if !field.annotation_specs().is_empty() {
            errors.push(SigilStitchError::InvalidField {
                language: lang.file_extension().to_string(),
                field_name: field.name().to_string(),
                context: fields.context(),
                reason: "Go struct fields do not support structured annotations; use tag() for a struct tag"
                    .to_string(),
            });
        }
        if !field.annotations().is_empty() {
            errors.push(SigilStitchError::InvalidField {
                language: lang.file_extension().to_string(),
                field_name: field.name().to_string(),
                context: fields.context(),
                reason: "Go struct fields do not support raw annotation blocks; use tag() for a struct tag"
                    .to_string(),
            });
        }
        if field
            .tag()
            .is_some_and(|tag| tag.contains(['`', '\n', '\r']))
        {
            errors.push(SigilStitchError::InvalidField {
                language: lang.file_extension().to_string(),
                field_name: field.name().to_string(),
                context: fields.context(),
                reason: "Go struct tags cannot contain backticks or line breaks".to_string(),
            });
        }
    }
}

pub(crate) fn lower(lang: &Go, fields: ValidatedFields<'_>) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    for field in fields.fields() {
        emit_doc(&mut block, lang, field);
        block.add("%L ", lang.escape_field_name(field.name()));
        block.add("%T", field.field_type().clone());
        if let Some(tag) = field.tag() {
            block.add(" `%L`", tag.to_string());
        }
        block.add_line();
    }
    block.build()
}
