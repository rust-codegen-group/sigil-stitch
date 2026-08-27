//! TypeScript-owned complete type-declaration grammar.

#![deny(deprecated)]

use crate::code_block::{Arg, CodeBlock};
use crate::error::SigilStitchError;
use crate::lang::RendererLang;
use crate::lang::typescript::TypeScript;
use crate::spec::modifiers::{TypeKind, Visibility};
use crate::spec::type_spec::{TypeIntent, ValidatedType};
use crate::spec::where_spec::{TypeParamSpec, WhereConstraint};
use crate::type_name::TypeName;

use super::common;

pub(crate) fn is_identifier(name: &str) -> bool {
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

pub(crate) fn validate(lang: &TypeScript, type_: TypeIntent<'_>) -> Result<(), SigilStitchError> {
    common::validate_declaration(
        type_,
        lang.file_extension(),
        is_identifier,
        &[Visibility::Inherited, Visibility::Public],
        &[TypeKind::Class, TypeKind::Struct],
    )?;
    if lang.reserved_words().contains(&type_.name()) {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: type_.name().to_string(),
            reason: "TypeScript reserves this declaration identifier".to_string(),
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
                reason: "TypeScript type declarations do not support context bounds".to_string(),
            });
        }
    }
    common::validate_constraint_subjects(type_, lang.file_extension(), type_.where_constraints())?;
    if matches!(type_.kind(), TypeKind::Class | TypeKind::Struct)
        && type_.nominal_super_types().len() > 1
    {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: type_.name().to_string(),
            reason: "TypeScript classes may extend at most one base class".to_string(),
        });
    }
    Ok(())
}

pub(crate) fn lower(
    lang: &TypeScript,
    type_: ValidatedType<'_>,
) -> Result<Vec<CodeBlock>, SigilStitchError> {
    if type_.kind() == TypeKind::TypeAlias {
        let mut block = CodeBlock::builder();
        preamble(&mut block, lang, &type_)?;
        let mut arguments = Vec::new();
        let params = type_parameters(&type_, &mut arguments);
        arguments.push(Arg::TypeName(
            type_.target_type().expect("validated target").clone(),
        ));
        let export = if type_.modifiers().visibility == Visibility::Public {
            "export "
        } else {
            ""
        };
        block.add(
            &format!("{export}type {}{params} = %T;", type_.name()),
            arguments,
        );
        block.add_line();
        return Ok(vec![block.build()?]);
    }
    let mut block = CodeBlock::builder();
    preamble(&mut block, lang, &type_)?;
    let mut format = String::new();
    if type_.modifiers().visibility == Visibility::Public {
        format.push_str("export ");
    }
    if type_.modifiers().is_abstract {
        format.push_str("abstract ");
    }
    format.push_str(match type_.kind() {
        TypeKind::Interface | TypeKind::Trait => "interface ",
        TypeKind::Enum => "enum ",
        TypeKind::Class | TypeKind::Struct => "class ",
        TypeKind::TypeAlias | TypeKind::Newtype => unreachable!(),
    });
    format.push_str(type_.name());
    let mut arguments = Vec::new();
    format.push_str(&type_parameters(&type_, &mut arguments));
    if !type_.nominal_super_types().is_empty() {
        format.push_str(" extends ");
        for (index, base) in type_.nominal_super_types().iter().enumerate() {
            if index > 0 {
                format.push_str(", ");
            }
            format.push_str("%T");
            arguments.push(Arg::TypeName(base.clone()));
        }
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
    let properties = if (variants || fields) && !type_.properties().is_empty() {
        block.add_line();
        common::emit_properties(&mut block, lang, &type_)?
    } else {
        common::emit_properties(&mut block, lang, &type_)?
    };
    if (variants || fields || properties) && !type_.methods().is_empty() {
        block.add_line();
    }
    common::emit_methods(&mut block, lang, &type_)?;
    common::emit_extra_members(&mut block, &type_);
    block.add("%<}", ());
    block.add_line();
    Ok(vec![block.build()?])
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
        let bounds = parameter_bounds(parameter, type_.where_constraints());
        if !bounds.is_empty() {
            format.push_str(" extends ");
            for (bound_index, bound) in bounds.into_iter().enumerate() {
                if bound_index > 0 {
                    format.push_str(" & ");
                }
                format.push_str("%T");
                arguments.push(Arg::TypeName(bound.clone()));
            }
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

fn preamble(
    block: &mut crate::code_block::CodeBlockBuilder,
    lang: &TypeScript,
    type_: &ValidatedType<'_>,
) -> Result<(), SigilStitchError> {
    common::emit_doc(block, lang, type_);
    common::emit_structured_annotations(block, type_, "@", "")?;
    common::emit_raw_annotations(block, type_);
    Ok(())
}
