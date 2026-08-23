//! C++-owned data-member grammar.

#![deny(deprecated)]

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::lang::capability::{FieldCapability, FieldCapabilityProfile, FieldContext};
use crate::lang::cpp::Cpp;
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
        FieldContext::Direct(DeclarationContext::TopLevel),
        CAPABILITIES,
    )
    .with_required_capabilities(REQUIRED),
    FieldCapabilityProfile::new(
        FieldContext::Direct(DeclarationContext::Member),
        CAPABILITIES,
    )
    .with_required_capabilities(REQUIRED),
    FieldCapabilityProfile::new(
        FieldContext::Direct(DeclarationContext::InterfaceMember),
        CAPABILITIES,
    )
    .with_required_capabilities(REQUIRED),
    FieldCapabilityProfile::new(FieldContext::TypeMember(TypeKind::Class), CAPABILITIES)
        .with_required_capabilities(REQUIRED),
    FieldCapabilityProfile::new(FieldContext::TypeMember(TypeKind::Struct), CAPABILITIES)
        .with_required_capabilities(REQUIRED),
    FieldCapabilityProfile::new(FieldContext::TypeMember(TypeKind::Interface), CAPABILITIES)
        .with_required_capabilities(REQUIRED),
    FieldCapabilityProfile::new(FieldContext::TypeMember(TypeKind::Trait), CAPABILITIES)
        .with_required_capabilities(REQUIRED),
];

fn is_valid_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|ch| ch == '_' || unicode_ident::is_xid_start(ch))
        && chars.all(unicode_ident::is_xid_continue)
}

pub(crate) fn validate(
    lang: &Cpp,
    fields: FieldSequenceIntent<'_>,
) -> Result<(), SigilStitchError> {
    super::validation_result(|errors| collect_validation_errors(lang, fields, errors))
}

pub(crate) fn collect_validation_errors(
    lang: &Cpp,
    fields: FieldSequenceIntent<'_>,
    errors: &mut Vec<SigilStitchError>,
) {
    collect_invalid_identifiers(lang, fields, is_valid_identifier, errors);
    collect_escaped_name_collisions(lang, fields, errors);
    for field in fields.fields() {
        if fields.context() == FieldContext::Direct(DeclarationContext::TopLevel)
            && field.modifiers().visibility != Visibility::Inherited
        {
            errors.push(SigilStitchError::InvalidField {
                language: lang.file_extension().to_string(),
                field_name: field.name().to_string(),
                context: fields.context(),
                reason: "C++ namespace-scope fields do not use access-section visibility"
                    .to_string(),
            });
        }
        if matches!(
            field.modifiers().visibility,
            Visibility::PublicCrate | Visibility::PublicSuper
        ) {
            errors.push(SigilStitchError::InvalidField {
                language: lang.file_extension().to_string(),
                field_name: field.name().to_string(),
                context: fields.context(),
                reason: "C++ data members do not support crate- or superclass-scoped visibility"
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
    }
}

fn default_visibility(context: FieldContext) -> Option<Visibility> {
    match context {
        FieldContext::TypeMember(TypeKind::Struct) => Some(Visibility::Public),
        FieldContext::TypeMember(TypeKind::Class | TypeKind::Interface | TypeKind::Trait) => {
            Some(Visibility::Private)
        }
        _ => None,
    }
}

fn emit_access_section(
    block: &mut crate::code_block::CodeBlockBuilder,
    visibility: Visibility,
    owner_aware: bool,
) {
    if owner_aware {
        block.add("%<", ());
    }
    block.add(
        match visibility {
            Visibility::Public => "public:",
            Visibility::Private | Visibility::Inherited => "private:",
            Visibility::Protected => "protected:",
            Visibility::PublicCrate | Visibility::PublicSuper => unreachable!(),
        },
        (),
    );
    block.add_line();
    if owner_aware {
        block.add("%>", ());
    }
}

pub(crate) fn lower(
    lang: &Cpp,
    fields: ValidatedFields<'_>,
) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    let default = default_visibility(fields.context());
    let mut current = default;
    for field in fields.fields() {
        let desired = match field.modifiers().visibility {
            Visibility::Inherited => default,
            explicit => Some(explicit),
        };
        if desired != current {
            emit_access_section(
                &mut block,
                desired.unwrap_or(Visibility::Private),
                default.is_some(),
            );
            current = desired;
        }
        emit_doc(&mut block, lang, field);
        emit_annotations(&mut block, field, "[[", "]]")?;
        if fields.context() != FieldContext::Direct(DeclarationContext::TopLevel)
            && field.modifiers().is_static
            && field.initializer().is_some()
        {
            block.add("inline ", ());
        }
        if field.modifiers().is_static {
            block.add("static ", ());
        }
        let const_after_type = field.modifiers().is_readonly
            && matches!(field.field_type(), crate::type_name::TypeName::Pointer(_));
        let reference_is_binding_readonly = field.modifiers().is_readonly
            && matches!(
                field.field_type(),
                crate::type_name::TypeName::Reference { .. }
            );
        if field.modifiers().is_readonly && !const_after_type && !reference_is_binding_readonly {
            block.add("const ", ());
        }
        block.add("%T", field.field_type().clone());
        if const_after_type {
            block.add(" const", ());
        }
        block.add(" %L", lang.escape_field_name(field.name()));
        if let Some(initializer) = field.initializer() {
            block.add(" = %L", initializer.clone());
        }
        block.add(";", ());
        block.add_line();
    }
    if let Some(default) = default
        && current != Some(default)
    {
        emit_access_section(&mut block, default, true);
    }
    block.build()
}
