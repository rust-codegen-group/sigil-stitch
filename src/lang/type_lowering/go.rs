//! Go-owned complete type-declaration grammar.

#![deny(deprecated)]

use crate::code_block::{Arg, CodeBlock};
use crate::error::SigilStitchError;
use crate::lang::RendererLang;
use crate::lang::go::Go;
use crate::spec::modifiers::{TypeKind, Visibility};
use crate::spec::type_spec::{TypeIntent, ValidatedType};
use crate::spec::where_spec::{TypeParamSpec, WhereConstraint};
use crate::type_name::TypeName;

use super::common;

pub(crate) fn validate(lang: &Go, type_: TypeIntent<'_>) -> Result<(), SigilStitchError> {
    common::validate_declaration(
        type_,
        lang.file_extension(),
        common::is_identifier,
        &[
            Visibility::Inherited,
            Visibility::Public,
            Visibility::Private,
        ],
        &[],
    )?;

    if lang.reserved_words().contains(&type_.name()) {
        return Err(invalid(type_, "Go reserves this declaration identifier"));
    }
    let exported = type_.name().chars().next().is_some_and(char::is_uppercase);
    match type_.modifiers().visibility {
        Visibility::Public if !exported => {
            return Err(invalid(
                type_,
                "public Go type names must begin with an uppercase letter",
            ));
        }
        Visibility::Private if exported => {
            return Err(invalid(
                type_,
                "private Go type names must begin with a lowercase letter",
            ));
        }
        _ => {}
    }

    for parameter in type_.type_params() {
        if parameter.is_lifetime()
            || !common::is_identifier(parameter.name())
            || lang.reserved_words().contains(&parameter.name())
        {
            return Err(SigilStitchError::InvalidTypeParameter {
                type_name: type_.name().to_string(),
                parameter_name: parameter.name().to_string(),
                reason: "Go type parameters require an ordinary non-keyword identifier".to_string(),
            });
        }
        if !parameter.context_bounds().is_empty() {
            return Err(SigilStitchError::InvalidTypeParameter {
                type_name: type_.name().to_string(),
                parameter_name: parameter.name().to_string(),
                reason: "Go type declarations do not support context bounds".to_string(),
            });
        }
    }
    if type_.kind() == TypeKind::Newtype
        && let Some(target) = type_.target_type()
        && let Some(target_name) = match target {
            TypeName::Primitive(name) | TypeName::Raw(name) => Some(name.trim()),
            _ => None,
        }
        && type_
            .type_params()
            .iter()
            .any(|parameter| parameter.name() == target_name)
    {
        return Err(invalid(
            type_,
            "Go newtypes cannot use a bare type parameter as their target type",
        ));
    }
    common::validate_constraint_subjects(type_, lang.file_extension(), type_.where_constraints())?;
    Ok(())
}

pub(crate) fn lower(
    lang: &Go,
    type_: ValidatedType<'_>,
) -> Result<Vec<CodeBlock>, SigilStitchError> {
    match type_.kind() {
        TypeKind::TypeAlias => return Ok(vec![lower_alias(lang, &type_)?]),
        TypeKind::Newtype => return Ok(vec![lower_newtype(lang, &type_)?]),
        _ => {}
    }

    let mut block = CodeBlock::builder();
    common::emit_doc(&mut block, lang, &type_);

    let kind = match type_.kind() {
        TypeKind::Struct | TypeKind::Class => "struct",
        TypeKind::Interface | TypeKind::Trait => "interface",
        TypeKind::Enum | TypeKind::TypeAlias | TypeKind::Newtype => unreachable!(),
    };
    let mut arguments = Vec::new();
    let parameters = type_parameters(&type_, &mut arguments);
    block.add(
        &format!("type {}{parameters} {kind} {{", type_.name()),
        arguments,
    );
    block.add_line();
    block.add("%>", ());

    for embedded in type_.embedded_types() {
        block.add("%T", embedded.clone());
        block.add_line();
    }

    match type_.kind() {
        TypeKind::Struct | TypeKind::Class => {
            common::emit_fields(&mut block, lang, &type_)?;
        }
        TypeKind::Interface | TypeKind::Trait => {
            common::emit_methods(&mut block, lang, &type_)?;
        }
        TypeKind::Enum | TypeKind::TypeAlias | TypeKind::Newtype => unreachable!(),
    }
    common::emit_extra_members(&mut block, &type_);
    block.add("%<}", ());
    block.add_line();
    Ok(vec![block.build()?])
}

fn lower_alias(lang: &Go, type_: &ValidatedType<'_>) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    common::emit_doc(&mut block, lang, type_);
    block.add(
        &format!("type {} = %T", type_.name()),
        type_
            .target_type()
            .expect("validated aliases have a target")
            .clone(),
    );
    block.add_line();
    block.build()
}

fn lower_newtype(lang: &Go, type_: &ValidatedType<'_>) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    common::emit_doc(&mut block, lang, type_);
    let mut arguments = Vec::new();
    let parameters = type_parameters(type_, &mut arguments);
    arguments.push(Arg::TypeName(
        type_
            .target_type()
            .expect("validated newtypes have a target")
            .clone(),
    ));
    block.add(&format!("type {}{parameters} %T", type_.name()), arguments);
    block.add_line();
    block.build()
}

fn type_parameters(type_: &ValidatedType<'_>, arguments: &mut Vec<Arg>) -> String {
    if type_.type_params().is_empty() {
        return String::new();
    }

    let mut format = String::from("[");
    for (index, parameter) in type_.type_params().iter().enumerate() {
        if index > 0 {
            format.push_str(", ");
        }
        format.push_str(parameter.name());
        format.push(' ');
        let bounds = parameter_bounds(parameter, type_.where_constraints());
        match bounds.as_slice() {
            [] => format.push_str("any"),
            [bound] => {
                format.push_str("%T");
                arguments.push(Arg::TypeName((*bound).clone()));
            }
            bounds => {
                format.push_str("interface { ");
                for (bound_index, bound) in bounds.iter().enumerate() {
                    if bound_index > 0 {
                        format.push_str("; ");
                    }
                    format.push_str("%T");
                    arguments.push(Arg::TypeName((*bound).clone()));
                }
                format.push_str(" }");
            }
        }
    }
    format.push(']');
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

fn invalid(type_: TypeIntent<'_>, reason: &str) -> SigilStitchError {
    SigilStitchError::InvalidTypeDeclaration {
        type_name: type_.name().to_string(),
        reason: reason.to_string(),
    }
}
