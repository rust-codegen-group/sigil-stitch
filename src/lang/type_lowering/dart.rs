//! Dart-owned complete type-declaration grammar.

#![deny(deprecated)]

use crate::code_block::{Arg, CodeBlock};
use crate::error::SigilStitchError;
use crate::lang::dart::Dart;
use crate::lang::{CodeLang, RendererLang};
use crate::spec::field_spec::{FieldSequenceIntent, FieldSpec};
use crate::spec::modifiers::{TypeKind, Visibility};
use crate::spec::type_spec::{TypeIntent, ValidatedType};
use crate::spec::where_spec::{TypeParamSpec, WhereConstraint};
use crate::type_name::TypeName;

use super::common;

pub(crate) fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|ch| ch == '_' || ch == '$' || unicode_ident::is_xid_start(ch))
        && chars.all(|ch| ch == '$' || unicode_ident::is_xid_continue(ch))
}

pub(crate) fn validate(lang: &Dart, type_: TypeIntent<'_>) -> Result<(), SigilStitchError> {
    common::validate_declaration(
        type_,
        lang.file_extension(),
        is_identifier,
        &[Visibility::Inherited],
        &[TypeKind::Class, TypeKind::Struct],
    )?;
    if lang.reserved_words().contains(&type_.name()) {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: type_.name().to_string(),
            reason: "Dart reserves this declaration identifier".to_string(),
        });
    }
    common::validate_ordinary_type_parameters(
        type_,
        lang.file_extension(),
        is_identifier,
        lang.reserved_words(),
    )?;
    for parameter in type_.type_params() {
        if !parameter.context_bounds().is_empty() {
            return Err(SigilStitchError::InvalidTypeParameter {
                type_name: type_.name().to_string(),
                parameter_name: parameter.name().to_string(),
                reason: "Dart type declarations do not support context bounds".to_string(),
            });
        }
        if parameter_bounds(parameter, type_.where_constraints()).len() > 1 {
            return Err(SigilStitchError::InvalidTypeParameter {
                type_name: type_.name().to_string(),
                parameter_name: parameter.name().to_string(),
                reason: "Dart type parameters accept at most one upper bound".to_string(),
            });
        }
    }
    common::validate_constraint_subjects(type_, lang.file_extension(), type_.where_constraints())?;
    if type_.nominal_super_types().len() > 1 {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: type_.name().to_string(),
            reason: "Dart declarations may extend at most one superclass".to_string(),
        });
    }
    if type_.kind() == TypeKind::Enum && type_.variants().is_empty() {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: type_.name().to_string(),
            reason: if type_.is_closed_sum() {
                "Dart closed sums require at least one generated case".to_string()
            } else {
                "Dart enum declarations require at least one value".to_string()
            },
        });
    }
    if type_.is_closed_sum()
        && (!type_.fields().is_empty()
            || !type_.properties().is_empty()
            || !type_.methods().is_empty()
            || !type_.embedded_types().is_empty()
            || !type_.extra_members().is_empty()
            || !type_.primary_constructor_parameters().is_empty()
            || !type_.nominal_super_types().is_empty()
            || !type_.implemented_types().is_empty())
    {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: type_.name().to_string(),
            reason: "Dart closed sums currently support only cases".to_string(),
        });
    }
    Ok(())
}

pub(crate) fn lower(
    lang: &Dart,
    type_: ValidatedType<'_>,
) -> Result<Vec<CodeBlock>, SigilStitchError> {
    if type_.is_closed_sum() {
        return lower_closed_sum(lang, &type_);
    }
    if type_.kind() == TypeKind::TypeAlias {
        let mut block = CodeBlock::builder();
        common::emit_doc(&mut block, lang, &type_);
        common::emit_structured_annotations(&mut block, &type_, "@", "")?;
        common::emit_raw_annotations(&mut block, &type_);
        let mut arguments = Vec::new();
        let params = type_parameters(&type_, &mut arguments);
        arguments.push(Arg::TypeName(
            type_.target_type().expect("validated target").clone(),
        ));
        block.add(
            &format!("typedef {}{params} = %T;", type_.name()),
            arguments,
        );
        block.add_line();
        return Ok(vec![block.build()?]);
    }
    let mut block = CodeBlock::builder();
    common::emit_doc(&mut block, lang, &type_);
    common::emit_structured_annotations(&mut block, &type_, "@", "")?;
    common::emit_raw_annotations(&mut block, &type_);
    let mut format = String::new();
    if type_.modifiers().is_abstract
        || matches!(type_.kind(), TypeKind::Interface | TypeKind::Trait)
    {
        format.push_str("abstract ");
    }
    format.push_str(if type_.kind() == TypeKind::Enum {
        "enum "
    } else {
        "class "
    });
    format.push_str(type_.name());
    let mut arguments = Vec::new();
    format.push_str(&type_parameters(&type_, &mut arguments));
    if let Some(base) = type_.nominal_super_types().first() {
        format.push_str(" extends %T");
        arguments.push(Arg::TypeName(base.clone()));
    }
    if !type_.implemented_types().is_empty() {
        format.push_str(" implements ");
        for (index, implemented) in type_.implemented_types().iter().enumerate() {
            if index > 0 {
                format.push_str(", ");
            }
            format.push_str("%T");
            arguments.push(Arg::TypeName(implemented.clone()));
        }
    }
    format.push_str(" {");
    block.add(&format, arguments);
    block.add_line();
    block.add("%>", ());
    let variants = common::emit_variants(&mut block, lang, &type_)?;
    let fields = common::emit_fields(&mut block, lang, &type_)?;
    if (variants || fields) && !type_.methods().is_empty() {
        block.add_line();
    }
    common::emit_methods(&mut block, lang, &type_)?;
    common::emit_extra_members(&mut block, &type_);
    block.add("%<}", ());
    block.add_line();
    Ok(vec![block.build()?])
}

fn lower_closed_sum(
    lang: &Dart,
    type_: &ValidatedType<'_>,
) -> Result<Vec<CodeBlock>, SigilStitchError> {
    let variants = type_
        .variants()
        .expect("validated closed sums retain their complete case set");
    let mut blocks = Vec::with_capacity(variants.variants().len() + 1);
    let mut root = CodeBlock::builder();
    common::emit_doc(&mut root, lang, type_);
    common::emit_structured_annotations(&mut root, type_, "@", "")?;
    common::emit_raw_annotations(&mut root, type_);
    root.add(&format!("sealed class {} {{", type_.name()), ());
    root.add_line();
    root.add("%>const ", ());
    root.add(&format!("{}._();", type_.name()), ());
    root.add_line();
    root.add("%<}", ());
    root.add_line();
    blocks.push(root.build()?);

    for variant in variants.variants() {
        let generated_name = format!("{}{}", type_.name(), variant.name());
        let mut block = CodeBlock::builder();
        crate::lang::variant_lowering::emit_doc(&mut block, lang, variant);
        crate::lang::variant_lowering::emit_structured_annotations(&mut block, variant, "@", "")?;
        crate::lang::variant_lowering::emit_raw_annotations(&mut block, variant);
        block.add(
            &format!("final class {generated_name} extends {} {{", type_.name()),
            (),
        );
        block.add_line();
        block.add("%>", ());
        if variant.positional_payload().is_empty() && variant.record_payload().is_empty() {
            block.add(&format!("const {generated_name}._() : super._();"), ());
            block.add_line();
            block.add(
                &format!("static const {generated_name} instance = {generated_name}._();"),
                (),
            );
            block.add_line();
        } else {
            block.add(&format!("const {generated_name}("), ());
            let field_count = if !variant.positional_payload().is_empty() {
                variant.positional_payload().len()
            } else {
                variant.record_payload().len()
            };
            for field_index in 0..field_count {
                if field_index > 0 {
                    block.add(", ", ());
                }
                let field_name = if !variant.positional_payload().is_empty() {
                    format!("value{field_index}")
                } else {
                    lang.escape_field_name(variant.record_payload()[field_index].name())
                };
                block.add("this.%L", field_name);
            }
            block.add(") : super._();", ());
            block.add_line();
            for (field_index, field_type) in variant.positional_payload().iter().enumerate() {
                block.add(&format!("final %T value{field_index};"), field_type.clone());
                block.add_line();
            }
            if !variant.record_payload().is_empty() {
                block.add_code(FieldSpec::lower_sequence(
                    FieldSequenceIntent::closed_sum_record_payload(
                        variant.record_payload(),
                        type_.name(),
                        variant.name(),
                    ),
                    lang,
                )?);
            }
        }
        block.add("%<}", ());
        block.add_line();
        blocks.push(block.build()?);
    }
    Ok(blocks)
}

fn type_parameters(type_: &ValidatedType<'_>, arguments: &mut Vec<Arg>) -> String {
    if type_.type_params().is_empty() {
        return String::new();
    }
    let mut format = String::from("<");
    for (index, parameter) in type_.type_params().iter().enumerate() {
        if index > 0 {
            format.push_str(", ");
        }
        format.push_str(parameter.name());
        if let Some(bound) = parameter_bounds(parameter, type_.where_constraints()).first() {
            format.push_str(" extends %T");
            arguments.push(Arg::TypeName((*bound).clone()));
        }
    }
    format.push('>');
    format
}

fn parameter_bounds<'a>(
    parameter: &'a TypeParamSpec,
    constraints: &'a [WhereConstraint],
) -> Vec<&'a TypeName> {
    let mut bounds = parameter.bounds().iter().collect::<Vec<_>>();
    for bound in constraints
        .iter()
        .filter(|constraint| constraint.parameter_subject_name() == Some(parameter.name()))
        .flat_map(WhereConstraint::bounds)
    {
        if !bounds.contains(&bound) {
            bounds.push(bound);
        }
    }
    bounds
}
