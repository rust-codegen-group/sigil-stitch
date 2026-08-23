//! C-owned field grammar.

#![deny(deprecated)]

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::lang::c::C;
use crate::lang::capability::{FieldCapability, FieldCapabilityProfile, FieldContext};
use crate::lang::{CodeLang, RendererLang};
use crate::spec::field_spec::{FieldSequenceIntent, ValidatedFields};
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};

use super::{
    collect_escaped_name_collisions, collect_invalid_identifiers, emit_annotations, emit_doc,
};

const CAPABILITIES: &[FieldCapability] = &[
    FieldCapability::ExplicitType,
    FieldCapability::Attributes,
    FieldCapability::ReadOnly,
];
const TOP_LEVEL_CAPABILITIES: &[FieldCapability] = &[
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
        TOP_LEVEL_CAPABILITIES,
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
    FieldCapabilityProfile::new(FieldContext::TypeMember(TypeKind::Struct), CAPABILITIES)
        .with_required_capabilities(REQUIRED),
    FieldCapabilityProfile::new(FieldContext::TypeMember(TypeKind::Class), CAPABILITIES)
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

fn requires_interleaved_declarator(field_type: &crate::type_name::TypeName) -> bool {
    use crate::type_name::TypeName;

    match field_type {
        TypeName::Array(_)
        | TypeName::ReadonlyArray(_)
        | TypeName::Slice(_)
        | TypeName::Function { .. } => true,
        TypeName::Generic { base, params } => {
            requires_interleaved_declarator(base)
                || params.iter().any(requires_interleaved_declarator)
        }
        TypeName::Union(types)
        | TypeName::Intersection(types)
        | TypeName::Tuple(types)
        | TypeName::ImplTrait { bounds: types }
        | TypeName::DynTrait { bounds: types } => types.iter().any(requires_interleaved_declarator),
        TypeName::Pointer(inner)
        | TypeName::Optional(inner)
        | TypeName::Reference { inner, .. } => requires_interleaved_declarator(inner),
        TypeName::Map { key, value } => {
            requires_interleaved_declarator(key) || requires_interleaved_declarator(value)
        }
        TypeName::AssociatedType {
            base, qualifier, ..
        } => {
            requires_interleaved_declarator(base)
                || qualifier
                    .as_deref()
                    .is_some_and(requires_interleaved_declarator)
        }
        TypeName::Wildcard {
            upper_bound,
            lower_bound,
        } => {
            upper_bound
                .as_deref()
                .is_some_and(requires_interleaved_declarator)
                || lower_bound
                    .as_deref()
                    .is_some_and(requires_interleaved_declarator)
        }
        TypeName::Importable { .. } | TypeName::Primitive(_) | TypeName::Raw(_) => false,
    }
}

pub(crate) fn validate(lang: &C, fields: FieldSequenceIntent<'_>) -> Result<(), SigilStitchError> {
    super::validation_result(|errors| collect_validation_errors(lang, fields, errors))
}

pub(crate) fn collect_validation_errors(
    lang: &C,
    fields: FieldSequenceIntent<'_>,
    errors: &mut Vec<SigilStitchError>,
) {
    collect_invalid_identifiers(lang, fields, is_valid_identifier, errors);
    collect_escaped_name_collisions(lang, fields, errors);
    for field in fields.fields() {
        if requires_interleaved_declarator(field.field_type()) {
            errors.push(SigilStitchError::InvalidField {
                language: lang.file_extension().to_string(),
                field_name: field.name().to_string(),
                context: fields.context(),
                reason: "this C declarator shape requires the field name inside the rendered type"
                    .to_string(),
            });
        }
        if field.modifiers().visibility != Visibility::Inherited {
            errors.push(SigilStitchError::InvalidField {
                language: lang.file_extension().to_string(),
                field_name: field.name().to_string(),
                context: fields.context(),
                reason: "C fields do not have declaration visibility modifiers".to_string(),
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

pub(crate) fn lower(lang: &C, fields: ValidatedFields<'_>) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    let top_level = fields.context() == FieldContext::Direct(DeclarationContext::TopLevel);
    for field in fields.fields() {
        emit_doc(&mut block, lang, field);
        emit_annotations(&mut block, field, "__attribute__((", "))")?;
        if top_level && field.modifiers().is_static {
            block.add("static ", ());
        }
        let const_after_type = field.modifiers().is_readonly
            && matches!(
                field.field_type(),
                crate::type_name::TypeName::Pointer(_)
                    | crate::type_name::TypeName::Reference { .. }
                    | crate::type_name::TypeName::Optional(_)
            );
        if field.modifiers().is_readonly && !const_after_type {
            block.add("const ", ());
        }
        block.add("%T", field.field_type().clone());
        if const_after_type {
            block.add(" const", ());
        }
        block.add(" ", ());
        block.add("%L", lang.escape_field_name(field.name()));
        if top_level && let Some(initializer) = field.initializer() {
            block.add(" = %L", initializer.clone());
        }
        block.add(";", ());
        block.add_line();
    }
    block.build()
}
