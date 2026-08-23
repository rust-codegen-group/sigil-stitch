//! C#-owned field grammar.

#![deny(deprecated)]

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::lang::capability::{FieldCapability, FieldCapabilityProfile, FieldContext};
use crate::lang::csharp::CSharp;
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
        )
    }

    fn is_continue(ch: char) -> bool {
        use GeneralCategory::*;
        is_start(ch)
            || matches!(
                get_general_category(ch),
                NonspacingMark | SpacingMark | DecimalNumber | ConnectorPunctuation | Format
            )
    }

    let mut chars = name.chars();
    chars.next().is_some_and(|ch| ch == '_' || is_start(ch)) && chars.all(is_continue)
}

pub(crate) fn validate(
    lang: &CSharp,
    fields: FieldSequenceIntent<'_>,
) -> Result<(), SigilStitchError> {
    super::validation_result(|errors| collect_validation_errors(lang, fields, errors))
}

pub(crate) fn collect_validation_errors(
    lang: &CSharp,
    fields: FieldSequenceIntent<'_>,
    errors: &mut Vec<SigilStitchError>,
) {
    collect_invalid_identifiers(lang, fields, is_valid_identifier, errors);
    collect_escaped_name_collisions(lang, fields, errors);
    for field in fields.fields() {
        if field.modifiers().visibility == Visibility::PublicSuper {
            errors.push(SigilStitchError::InvalidField {
                language: lang.file_extension().to_string(),
                field_name: field.name().to_string(),
                context: fields.context(),
                reason: "C# fields do not support superclass-scoped visibility".to_string(),
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

fn visibility(visibility: Visibility) -> &'static str {
    match visibility {
        Visibility::Inherited => "",
        Visibility::Public => "public ",
        Visibility::Private => "private ",
        Visibility::Protected => "protected ",
        Visibility::PublicCrate => "internal ",
        Visibility::PublicSuper => unreachable!(),
    }
}

pub(crate) fn lower(
    lang: &CSharp,
    fields: ValidatedFields<'_>,
) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    for field in fields.fields() {
        emit_doc(&mut block, lang, field);
        emit_annotations(&mut block, field, "[", "]")?;
        block.add("%L", visibility(field.modifiers().visibility));
        if field.modifiers().is_static {
            block.add("static ", ());
        }
        if field.modifiers().is_readonly {
            block.add("readonly ", ());
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
