//! Python-owned class-field grammar.

#![deny(deprecated)]

use crate::code_block::{CodeBlock, CodeBlockBuilder, CommentArg};
use crate::code_node::{CodeNode, LayoutBlock, LayoutBreakMode};
use crate::error::SigilStitchError;
use crate::lang::capability::{FieldCapability, FieldCapabilityProfile, FieldContext};
use crate::lang::python::Python;
use crate::lang::{CodeLang, RendererLang};
use crate::spec::field_spec::{FieldSequenceIntent, ValidatedFields};
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};
use unicode_normalization::UnicodeNormalization;

use super::{collect_invalid_identifiers, collect_name_collisions_by, emit_doc};

const CAPABILITIES: &[FieldCapability] =
    &[FieldCapability::ExplicitType, FieldCapability::Initializer];

pub(crate) const PROFILES: &[FieldCapabilityProfile] = &[
    FieldCapabilityProfile::new(
        FieldContext::Direct(DeclarationContext::Member),
        CAPABILITIES,
    ),
    FieldCapabilityProfile::new(
        FieldContext::Direct(DeclarationContext::InterfaceMember),
        CAPABILITIES,
    ),
    FieldCapabilityProfile::new(FieldContext::TypeMember(TypeKind::Class), CAPABILITIES),
    FieldCapabilityProfile::new(FieldContext::TypeMember(TypeKind::Struct), CAPABILITIES),
    FieldCapabilityProfile::new(FieldContext::TypeMember(TypeKind::Interface), CAPABILITIES),
    FieldCapabilityProfile::new(FieldContext::TypeMember(TypeKind::Trait), CAPABILITIES),
];

fn is_valid_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|ch| ch == '_' || unicode_ident::is_xid_start(ch))
        && chars.all(unicode_ident::is_xid_continue)
}

fn collision_identifier(lang: &Python, name: &str) -> String {
    lang.escape_field_name(name).nfkc().collect()
}

pub(crate) fn validate(
    lang: &Python,
    fields: FieldSequenceIntent<'_>,
) -> Result<(), SigilStitchError> {
    super::validation_result(|errors| collect_validation_errors(lang, fields, errors))
}

pub(crate) fn collect_validation_errors(
    lang: &Python,
    fields: FieldSequenceIntent<'_>,
    errors: &mut Vec<SigilStitchError>,
) {
    collect_invalid_identifiers(lang, fields, is_valid_identifier, errors);
    collect_name_collisions_by(
        lang,
        fields,
        |name| collision_identifier(lang, name),
        errors,
    );
    for field in fields.fields() {
        let private = field.name().starts_with('_');
        let visibility_is_valid = match field.modifiers().visibility {
            Visibility::Inherited => true,
            Visibility::Public => !private,
            Visibility::Private | Visibility::Protected => private,
            Visibility::PublicCrate | Visibility::PublicSuper => false,
        };
        if !visibility_is_valid {
            errors.push(SigilStitchError::InvalidField {
                language: lang.file_extension().to_string(),
                field_name: field.name().to_string(),
                context: fields.context(),
                reason: "Python field visibility is expressed by identifier naming conventions"
                    .to_string(),
            });
        }
        if field.field_type().is_empty() && field.initializer().is_none() {
            errors.push(SigilStitchError::InvalidField {
                language: lang.file_extension().to_string(),
                field_name: field.name().to_string(),
                context: fields.context(),
                reason: "a Python class field requires a type annotation or initializer"
                    .to_string(),
            });
        }
    }
}

pub(crate) fn lower(
    lang: &Python,
    fields: ValidatedFields<'_>,
) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    for field in fields.fields() {
        emit_doc(&mut block, lang, field);
        block.add("%L", lang.escape_field_name(field.name()));
        if !field.field_type().is_empty() {
            block.add(": %T", field.field_type().clone());
        }
        if let Some(initializer) = field.initializer() {
            add_initializer(&mut block, initializer);
        }
        block.add_line();
    }
    block.build()
}

fn add_initializer(block: &mut CodeBlockBuilder, initializer: &CodeBlock) {
    if initializer.contains_opaque_multiline() {
        block.add(" = %L", initializer.clone().for_inline_composition());
        return;
    }

    let mut value = initializer.clone().for_inline_composition();
    let pragma = take_inline_pragma(&mut value);
    let layout =
        LayoutBlock::consistent(LayoutBreakMode::Space, " = ", "", " = (", ")", value.nodes)
            .with_preferred_inner_breaks();
    block.add_code(CodeBlock {
        nodes: vec![CodeNode::Layout(layout)],
    });
    if let Some(comment) = pragma {
        block.add("  %R", (CommentArg(comment),));
    }
}

fn take_inline_pragma(initializer: &mut CodeBlock) -> Option<String> {
    let mut comment_index = initializer.nodes.len();
    if matches!(initializer.nodes.last(), Some(CodeNode::Newline)) {
        comment_index -= 1;
    }
    let comment_index = comment_index.checked_sub(1)?;
    let CodeNode::Comment(comment) = initializer.nodes.get(comment_index)? else {
        return None;
    };
    if !crate::lang::python::is_pragma_comment(comment) {
        return None;
    }
    let comment = comment.clone();
    initializer.nodes.truncate(comment_index);
    if let Some(CodeNode::Literal(text)) = initializer.nodes.last_mut() {
        text.truncate(text.trim_end_matches([' ', '\t']).len());
    }
    Some(comment)
}
