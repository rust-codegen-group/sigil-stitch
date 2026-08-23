//! Scala-owned accessor-method property grammar.

#![deny(deprecated)]

use crate::code_block::{CodeBlock, CodeBlockBuilder};
use crate::error::SigilStitchError;
use crate::lang::capability::{PropertyCapability, PropertyCapabilityProfile, PropertyContext};
use crate::lang::field_lowering::scala::is_valid_identifier;
use crate::lang::scala::Scala;
use crate::lang::{CodeLang, RendererLang};
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};
use crate::spec::property_spec::{PropertyIntent, PropertySpec, ValidatedProperty};

const CAPABILITIES: &[PropertyCapability] = &[
    PropertyCapability::ExplicitType,
    PropertyCapability::ReadAccessor,
    PropertyCapability::WriteAccessor,
    PropertyCapability::Attributes,
];

pub(crate) const PROFILES: &[PropertyCapabilityProfile] = &[
    PropertyCapabilityProfile::new(
        PropertyContext::Direct(DeclarationContext::Member),
        CAPABILITIES,
    ),
    PropertyCapabilityProfile::new(PropertyContext::TypeMember(TypeKind::Class), CAPABILITIES),
    PropertyCapabilityProfile::new(PropertyContext::TypeMember(TypeKind::Struct), CAPABILITIES),
    PropertyCapabilityProfile::new(PropertyContext::TypeMember(TypeKind::Enum), CAPABILITIES),
];

pub(crate) fn validate(lang: &Scala, property: PropertyIntent<'_>) -> Result<(), SigilStitchError> {
    super::validation_result(|errors| collect_validation_errors(lang, property, errors))
}

pub(crate) fn collect_validation_errors(
    lang: &Scala,
    intent: PropertyIntent<'_>,
    errors: &mut Vec<SigilStitchError>,
) {
    let property = intent.property();
    let invalid = |reason: &str, errors: &mut Vec<SigilStitchError>| {
        errors.push(SigilStitchError::InvalidProperty {
            language: lang.file_extension().to_string(),
            property_name: property.name().to_string(),
            context: intent.context(),
            reason: reason.to_string(),
        });
    };
    if !is_valid_identifier(property.name()) || property.name() == "_" {
        invalid("property name is not a valid Scala identifier", errors);
    }
    if let Some(setter) = property.setter() {
        if !is_valid_identifier(setter.param_name()) || setter.param_name() == "_" {
            invalid("setter parameter is not a valid Scala identifier", errors);
        }
        if property.property_type().is_empty() {
            invalid(
                "Scala setter methods require an explicit value type",
                errors,
            );
        }
    }
    if !matches!(
        property.modifiers().visibility,
        Visibility::Inherited | Visibility::Public | Visibility::Private | Visibility::Protected
    ) {
        invalid(
            "Scala accessors do not support crate- or superclass-scoped visibility",
            errors,
        );
    }
}

pub(crate) fn lower(
    lang: &Scala,
    intent: ValidatedProperty<'_>,
) -> Result<Vec<CodeBlock>, SigilStitchError> {
    let property = intent.property();
    let mut blocks = Vec::new();
    if let Some(body) = property.getter() {
        let mut block = CodeBlock::builder();
        emit_preamble(&mut block, lang, property)?;
        emit_prefix(&mut block, lang, property);
        block.add("def %L", lang.escape_field_name(property.name()));
        if !property.property_type().is_empty() {
            block.add(": %T", property.property_type().clone());
        }
        block.add(" =", ());
        emit_body(&mut block, body);
        blocks.push(block.build()?);
    }
    if let Some(setter) = property.setter() {
        let mut block = CodeBlock::builder();
        if property.getter().is_none() {
            emit_preamble(&mut block, lang, property)?;
        }
        emit_prefix(&mut block, lang, property);
        block.add(
            "def %L_=(%L: %T): Unit =",
            (
                lang.escape_field_name(property.name()),
                lang.escape_reserved(setter.param_name()),
                property.property_type().clone(),
            ),
        );
        emit_body(&mut block, setter.body());
        blocks.push(block.build()?);
    }
    Ok(blocks)
}

fn emit_prefix(block: &mut CodeBlockBuilder, lang: &Scala, property: &PropertySpec) {
    block.add(
        "%L",
        lang.render_visibility(property.modifiers().visibility, DeclarationContext::Member),
    );
}

fn emit_body(block: &mut CodeBlockBuilder, body: &CodeBlock) {
    block.add(" {", ());
    block.add_line();
    block.add("%>", ());
    block.add_code(body.clone());
    if !body.ends_with_newline_or_block_close() {
        block.add_line();
    }
    block.add("%<}", ());
    block.add_line();
}

fn emit_preamble(
    block: &mut CodeBlockBuilder,
    lang: &Scala,
    property: &PropertySpec,
) -> Result<(), SigilStitchError> {
    if !property.doc().is_empty() {
        let lines: Vec<&str> = property.doc().iter().map(String::as_str).collect();
        block.add("%L", lang.render_doc_comment(&lines));
        block.add_line();
    }
    for annotation in property.annotation_specs() {
        block.add_code(annotation.emit_with_syntax("@", "")?);
        block.add_line();
    }
    for annotation in property.annotations() {
        block.add_code(annotation.clone());
        block.add_line();
    }
    Ok(())
}
