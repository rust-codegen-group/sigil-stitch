//! Swift-owned stored-property grammar.

#![deny(deprecated)]

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::lang::capability::{FieldCapability, FieldCapabilityProfile, FieldContext};
use crate::lang::swift::Swift;
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

pub(crate) fn is_valid_identifier(name: &str) -> bool {
    fn is_head(ch: char) -> bool {
        matches!(
            ch,
            'A'..='Z'
                | 'a'..='z'
                | '_'
                | '\u{00a8}'
                | '\u{00aa}'
                | '\u{00ad}'
                | '\u{00af}'
                | '\u{00b2}'..='\u{00b5}'
                | '\u{00b7}'..='\u{00ba}'
                | '\u{00bc}'..='\u{00be}'
                | '\u{00c0}'..='\u{00d6}'
                | '\u{00d8}'..='\u{00f6}'
                | '\u{00f8}'..='\u{00ff}'
                | '\u{0100}'..='\u{02ff}'
                | '\u{0370}'..='\u{167f}'
                | '\u{1681}'..='\u{180d}'
                | '\u{180f}'..='\u{1dbf}'
                | '\u{1e00}'..='\u{1fff}'
                | '\u{200b}'..='\u{200d}'
                | '\u{202a}'..='\u{202e}'
                | '\u{203f}'..='\u{2040}'
                | '\u{2054}'
                | '\u{2060}'..='\u{206f}'
                | '\u{2070}'..='\u{218f}'
                | '\u{2460}'..='\u{24ff}'
                | '\u{2776}'..='\u{2793}'
                | '\u{2c00}'..='\u{2dff}'
                | '\u{2e80}'..='\u{2fff}'
                | '\u{3004}'..='\u{3007}'
                | '\u{3021}'..='\u{302f}'
                | '\u{3031}'..='\u{303f}'
                | '\u{3040}'..='\u{d7ff}'
                | '\u{f900}'..='\u{fd3d}'
                | '\u{fd40}'..='\u{fdcf}'
                | '\u{fdf0}'..='\u{fe1f}'
                | '\u{fe30}'..='\u{fe44}'
                | '\u{fe47}'..='\u{fffd}'
                | '\u{10000}'..='\u{efffd}'
        )
    }

    fn is_continue(ch: char) -> bool {
        is_head(ch)
            || matches!(
                ch,
                '0'..='9'
                    | '\u{0300}'..='\u{036f}'
                    | '\u{1dc0}'..='\u{1dff}'
                    | '\u{20d0}'..='\u{20ff}'
                    | '\u{fe20}'..='\u{fe2f}'
            )
    }

    let mut chars = name.chars();
    chars.next().is_some_and(is_head) && chars.all(is_continue)
}

pub(crate) fn validate(
    lang: &Swift,
    fields: FieldSequenceIntent<'_>,
) -> Result<(), SigilStitchError> {
    super::validation_result(|errors| collect_validation_errors(lang, fields, errors))
}

pub(crate) fn collect_validation_errors(
    lang: &Swift,
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
                reason: "Swift enum-case record payloads require undocumented inherited-visibility labels"
                    .to_string(),
            });
        }
        if field.name() == "_" {
            errors.push(SigilStitchError::InvalidField {
                language: lang.file_extension().to_string(),
                field_name: field.name().to_string(),
                context: fields.context(),
                reason: "Swift field declarations must bind a named property".to_string(),
            });
        }
        if !matches!(
            field.modifiers().visibility,
            Visibility::Inherited
                | Visibility::Public
                | Visibility::Private
                | Visibility::PublicCrate
        ) {
            errors.push(SigilStitchError::InvalidField {
                language: lang.file_extension().to_string(),
                field_name: field.name().to_string(),
                context: fields.context(),
                reason: "Swift stored properties do not support this visibility".to_string(),
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
                reason: "a Swift stored property requires a type annotation or initializer"
                    .to_string(),
            });
        }
        if field.modifiers().is_static
            && field.initializer().is_none()
            && (field.modifiers().is_readonly
                || !matches!(field.field_type(), crate::type_name::TypeName::Optional(_)))
        {
            errors.push(SigilStitchError::InvalidField {
                language: lang.file_extension().to_string(),
                field_name: field.name().to_string(),
                context: fields.context(),
                reason: "a Swift stored type property without an initializer must be a mutable optional value"
                    .to_string(),
            });
        }
    }
}

pub(crate) fn lower(
    lang: &Swift,
    fields: ValidatedFields<'_>,
) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    if fields.context() == FieldContext::ClosedSumRecordPayload {
        for (index, field) in fields.fields().iter().enumerate() {
            if index > 0 {
                block.add(", ", ());
            }
            block.add(
                "%L: %T",
                (
                    lang.escape_field_name(field.name()),
                    field.field_type().clone(),
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
        block.add(
            if field.modifiers().is_readonly {
                "let "
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
