//! JavaScript-owned computed-property grammar.

#![deny(deprecated)]

use crate::code_block::{CodeBlock, CodeBlockBuilder};
use crate::error::SigilStitchError;
use crate::lang::capability::{PropertyCapability, PropertyCapabilityProfile, PropertyContext};
use crate::lang::field_lowering::javascript::{is_valid_property_name, property_key};
use crate::lang::javascript::JavaScript;
use crate::lang::{CodeLang, RendererLang};
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};
use crate::spec::property_spec::{PropertyIntent, PropertySpec, ValidatedProperty};

const CAPABILITIES: &[PropertyCapability] = &[
    PropertyCapability::ReadAccessor,
    PropertyCapability::WriteAccessor,
    PropertyCapability::Attributes,
    PropertyCapability::StaticProperty,
];

pub(crate) const PROFILES: &[PropertyCapabilityProfile] = &[
    PropertyCapabilityProfile::new(
        PropertyContext::Direct(DeclarationContext::Member),
        CAPABILITIES,
    ),
    PropertyCapabilityProfile::new(
        PropertyContext::Direct(DeclarationContext::InterfaceMember),
        CAPABILITIES,
    ),
    PropertyCapabilityProfile::new(PropertyContext::TypeMember(TypeKind::Class), CAPABILITIES),
    PropertyCapabilityProfile::new(PropertyContext::TypeMember(TypeKind::Struct), CAPABILITIES),
    PropertyCapabilityProfile::new(
        PropertyContext::TypeMember(TypeKind::Interface),
        CAPABILITIES,
    ),
    PropertyCapabilityProfile::new(PropertyContext::TypeMember(TypeKind::Trait), CAPABILITIES),
    PropertyCapabilityProfile::new(PropertyContext::TypeMember(TypeKind::Enum), CAPABILITIES),
];

pub(crate) fn validate(
    lang: &JavaScript,
    property: PropertyIntent<'_>,
) -> Result<(), SigilStitchError> {
    super::validation_result(|errors| collect_validation_errors(lang, property, errors))
}

pub(crate) fn collect_validation_errors(
    lang: &JavaScript,
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
    if !is_valid_property_name(property.name()) {
        invalid(
            "property name is not a valid JavaScript property name",
            errors,
        );
    }
    let private_name = property.name().starts_with('#');
    let valid_visibility = match property.modifiers().visibility {
        Visibility::Inherited => true,
        Visibility::Public => !private_name,
        Visibility::Private => private_name,
        Visibility::Protected | Visibility::PublicCrate | Visibility::PublicSuper => false,
    };
    if !valid_visibility {
        invalid(
            "JavaScript private accessors must use a #name and other member visibility is implicit",
            errors,
        );
    }
    if property_key(property.name()).as_deref() == Some("constructor") {
        invalid("JavaScript accessors cannot be named constructor", errors);
    }
    if property.modifiers().is_static
        && !private_name
        && property_key(property.name()).as_deref() == Some("prototype")
    {
        invalid(
            "JavaScript static accessors cannot be named prototype",
            errors,
        );
    }
    if let Some(setter) = property.setter()
        && !is_valid_parameter_name(setter.param_name())
    {
        invalid(
            "setter parameter is not a valid JavaScript binding identifier",
            errors,
        );
    }
}

pub(crate) fn lower(
    lang: &JavaScript,
    intent: ValidatedProperty<'_>,
) -> Result<Vec<CodeBlock>, SigilStitchError> {
    let property = intent.property();
    let mut blocks = Vec::new();
    if let Some(body) = property.getter() {
        let mut block = CodeBlock::builder();
        emit_preamble(&mut block, lang, property)?;
        emit_prefix(&mut block, property);
        block.add("get %L()", property.name());
        emit_body(&mut block, body);
        blocks.push(block.build()?);
    }
    if let Some(setter) = property.setter() {
        let mut block = CodeBlock::builder();
        if property.getter().is_none() {
            emit_preamble(&mut block, lang, property)?;
        }
        emit_prefix(&mut block, property);
        block.add(
            "set %L(%L)",
            (property.name(), lang.escape_reserved(setter.param_name())),
        );
        emit_body(&mut block, setter.body());
        blocks.push(block.build()?);
    }
    Ok(blocks)
}

fn is_valid_parameter_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|ch| ch == '_' || ch == '$' || unicode_id_start::is_id_start(ch))
        && chars.all(|ch| {
            ch == '$'
                || ch == '\u{200c}'
                || ch == '\u{200d}'
                || unicode_id_start::is_id_continue(ch)
        })
}

fn emit_prefix(block: &mut CodeBlockBuilder, property: &PropertySpec) {
    if property.modifiers().is_static {
        block.add("static ", ());
    }
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
    lang: &JavaScript,
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
