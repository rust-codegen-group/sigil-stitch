//! Kotlin-owned computed-property grammar.

#![deny(deprecated)]

use crate::code_block::{CodeBlock, CodeBlockBuilder};
use crate::error::SigilStitchError;
use crate::lang::capability::{PropertyCapability, PropertyCapabilityProfile, PropertyContext};
use crate::lang::field_lowering::kotlin::is_valid_identifier;
use crate::lang::kotlin::Kotlin;
use crate::lang::{CodeLang, RendererLang};
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};
use crate::spec::property_spec::{PropertyIntent, PropertySpec, ValidatedProperty};

const CAPABILITIES: &[PropertyCapability] = &[
    PropertyCapability::ExplicitType,
    PropertyCapability::ReadAccessor,
    PropertyCapability::WriteAccessor,
    PropertyCapability::Attributes,
];
const REQUIRED: &[PropertyCapability] = &[
    PropertyCapability::ExplicitType,
    PropertyCapability::ReadAccessor,
];

pub(crate) const PROFILES: &[PropertyCapabilityProfile] = &[
    PropertyCapabilityProfile::new(
        PropertyContext::Direct(DeclarationContext::Member),
        CAPABILITIES,
    )
    .with_required_capabilities(REQUIRED),
    PropertyCapabilityProfile::new(
        PropertyContext::Direct(DeclarationContext::InterfaceMember),
        CAPABILITIES,
    )
    .with_required_capabilities(REQUIRED),
    PropertyCapabilityProfile::new(PropertyContext::TypeMember(TypeKind::Class), CAPABILITIES)
        .with_required_capabilities(REQUIRED),
    PropertyCapabilityProfile::new(PropertyContext::TypeMember(TypeKind::Struct), CAPABILITIES)
        .with_required_capabilities(REQUIRED),
    PropertyCapabilityProfile::new(
        PropertyContext::TypeMember(TypeKind::Interface),
        CAPABILITIES,
    )
    .with_required_capabilities(REQUIRED),
    PropertyCapabilityProfile::new(PropertyContext::TypeMember(TypeKind::Trait), CAPABILITIES)
        .with_required_capabilities(REQUIRED),
    PropertyCapabilityProfile::new(PropertyContext::TypeMember(TypeKind::Enum), CAPABILITIES)
        .with_required_capabilities(REQUIRED),
];

pub(crate) fn validate(
    lang: &Kotlin,
    property: PropertyIntent<'_>,
) -> Result<(), SigilStitchError> {
    super::validation_result(|errors| collect_validation_errors(lang, property, errors))
}

pub(crate) fn collect_validation_errors(
    lang: &Kotlin,
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
        invalid("property name is not a valid Kotlin identifier", errors);
    }
    if let Some(setter) = property.setter()
        && (!is_valid_identifier(setter.param_name()) || setter.param_name() == "_")
    {
        invalid("setter parameter is not a valid Kotlin identifier", errors);
    }
    let contract = matches!(
        intent.context(),
        PropertyContext::Direct(DeclarationContext::InterfaceMember)
            | PropertyContext::TypeMember(TypeKind::Interface | TypeKind::Trait)
    );
    let visibility_is_valid = if contract {
        matches!(
            property.modifiers().visibility,
            Visibility::Inherited | Visibility::Public
        )
    } else {
        property.modifiers().visibility != Visibility::PublicSuper
    };
    if !visibility_is_valid {
        invalid(
            "Kotlin does not support this property visibility in the selected context",
            errors,
        );
    }
}

pub(crate) fn lower(
    lang: &Kotlin,
    intent: ValidatedProperty<'_>,
) -> Result<Vec<CodeBlock>, SigilStitchError> {
    let property = intent.property();
    let mut block = CodeBlock::builder();
    emit_preamble(&mut block, lang, property)?;
    let context = match intent.context() {
        PropertyContext::Direct(context) => context,
        PropertyContext::TypeMember(TypeKind::Interface | TypeKind::Trait) => {
            DeclarationContext::InterfaceMember
        }
        PropertyContext::TypeMember(_) => DeclarationContext::Member,
    };
    block.add("%L", visibility(property.modifiers().visibility, context));
    block.add(
        if property.setter().is_some() {
            "var "
        } else {
            "val "
        },
        (),
    );
    block.add(
        "%L: %T",
        (
            lang.escape_field_name(property.name()),
            property.property_type().clone(),
        ),
    );
    block.add_line();
    block.add("%>", ());
    if let Some(body) = property.getter() {
        block.add("get()", ());
        emit_body(&mut block, body);
    }
    if let Some(setter) = property.setter() {
        block.add("set(%L)", lang.escape_reserved(setter.param_name()));
        emit_body(&mut block, setter.body());
    }
    block.add("%<", ());
    Ok(vec![block.build()?])
}

fn visibility(visibility: Visibility, context: DeclarationContext) -> &'static str {
    if context == DeclarationContext::InterfaceMember {
        return "";
    }
    match visibility {
        Visibility::Inherited | Visibility::Public => "",
        Visibility::Private => "private ",
        Visibility::Protected => "protected ",
        Visibility::PublicCrate => "internal ",
        Visibility::PublicSuper => unreachable!(),
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
    lang: &Kotlin,
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
