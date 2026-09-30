//! Rust-owned complete type-declaration grammar.

#![deny(deprecated)]

use crate::code_block::{Arg, CodeBlock, CodeBlockBuilder};
use crate::error::SigilStitchError;
use crate::lang::rust::Rust;
use crate::lang::{CodeLang, RendererLang};
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};
use crate::spec::type_spec::{TypeIntent, ValidatedType};
use crate::spec::where_spec::WhereConstraint;

use super::common;

pub(crate) fn validate(lang: &Rust, type_: TypeIntent<'_>) -> Result<(), SigilStitchError> {
    common::validate_declaration(
        type_,
        lang.file_extension(),
        common::is_identifier,
        &[
            Visibility::Inherited,
            Visibility::Public,
            Visibility::Private,
            Visibility::PublicCrate,
            Visibility::PublicSuper,
        ],
        &[],
    )?;
    if matches!(type_.name(), "self" | "Self" | "super" | "crate")
        || lang.reserved_words().contains(&type_.name())
    {
        return Err(invalid(
            type_,
            "Rust reserves this identifier in type position",
        ));
    }
    for parameter in type_.type_params() {
        if parameter.is_lifetime() {
            if !crate::lang::rust::is_valid_lifetime_parameter_name(parameter.name()) {
                return Err(invalid_parameter(
                    type_,
                    parameter.name(),
                    "Rust lifetime parameters require a valid non-keyword declared name",
                ));
            }
            if parameter.bounds().iter().any(|bound| {
                !crate::lang::rust::is_valid_lifetime_bound(bound, type_.type_params())
            }) {
                return Err(invalid_parameter(
                    type_,
                    parameter.name(),
                    "Rust lifetime parameters accept only declared lifetime or 'static bounds",
                ));
            }
        } else if !common::is_identifier(parameter.name())
            || parameter.name().starts_with('\'')
            || lang.reserved_words().contains(&parameter.name())
        {
            return Err(invalid_parameter(
                type_,
                parameter.name(),
                "Rust type parameters require an ordinary non-keyword identifier",
            ));
        }
        if !parameter.context_bounds().is_empty() {
            return Err(invalid_parameter(
                type_,
                parameter.name(),
                "Rust type declarations do not support Scala-style context bounds",
            ));
        }
    }
    for constraint in type_.where_constraints() {
        let subject =
            match crate::lang::rust::lifetime_constraint_subject_name(constraint.subject()) {
                Ok(Some(subject)) => subject,
                Ok(None) => continue,
                Err(()) => {
                    return Err(invalid_parameter(
                        type_,
                        &format!("{:?}", constraint.subject()),
                        "Rust lifetime constraints must use an unparameterized lifetime subject",
                    ));
                }
            };
        if !type_.type_params().iter().any(|parameter| {
            parameter.is_lifetime()
                && parameter.name() == subject
                && crate::lang::rust::is_valid_lifetime_parameter_name(parameter.name())
        }) {
            return Err(invalid_parameter(
                type_,
                subject,
                "Rust lifetime constraints must target a declared lifetime",
            ));
        }
        if constraint
            .bounds()
            .iter()
            .any(|bound| !crate::lang::rust::is_valid_lifetime_bound(bound, type_.type_params()))
        {
            return Err(invalid_parameter(
                type_,
                subject,
                "Rust lifetime constraints accept only declared lifetime or 'static bounds",
            ));
        }
    }
    Ok(())
}

pub(crate) fn lower(
    lang: &Rust,
    type_: ValidatedType<'_>,
) -> Result<Vec<CodeBlock>, SigilStitchError> {
    match type_.kind() {
        TypeKind::TypeAlias => return Ok(vec![lower_alias(lang, &type_)?]),
        TypeKind::Newtype => return Ok(vec![lower_newtype(lang, &type_)?]),
        _ => {}
    }

    let mut blocks = vec![lower_declaration(lang, &type_)?];
    if !matches!(type_.kind(), TypeKind::Trait | TypeKind::Interface)
        && (!type_.properties().is_empty() || !type_.methods().is_empty())
    {
        blocks.push(lower_impl(lang, &type_)?);
    }
    Ok(blocks)
}

fn lower_declaration(
    lang: &Rust,
    type_: &ValidatedType<'_>,
) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    preamble(&mut block, lang, type_)?;
    let keyword = match type_.kind() {
        TypeKind::Struct | TypeKind::Class => "struct",
        TypeKind::Trait | TypeKind::Interface => "trait",
        TypeKind::Enum => "enum",
        TypeKind::TypeAlias | TypeKind::Newtype => unreachable!(),
    };
    let mut arguments = Vec::new();
    let parameters = type_parameters(type_, &mut arguments);
    block.add(
        &format!(
            "{}{keyword} {}{parameters}",
            lang.render_visibility(type_.modifiers().visibility, DeclarationContext::TopLevel),
            lang.escape_reserved(type_.name())
        ),
        arguments,
    );
    append_where_and_open(&mut block, type_.where_constraints());
    block.add("%>", ());

    match type_.kind() {
        TypeKind::Struct | TypeKind::Class => {
            common::emit_fields(&mut block, lang, type_)?;
        }
        TypeKind::Trait | TypeKind::Interface => {
            common::emit_methods(&mut block, lang, type_)?;
        }
        TypeKind::Enum => {
            common::emit_variants(&mut block, lang, type_)?;
        }
        TypeKind::TypeAlias | TypeKind::Newtype => unreachable!(),
    }
    common::emit_extra_members(&mut block, type_);
    block.add("%<}", ());
    block.add_line();
    block.build()
}

fn lower_impl(lang: &Rust, type_: &ValidatedType<'_>) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    let mut arguments = Vec::new();
    let parameters = type_parameters(type_, &mut arguments);
    let arguments_for_type = bare_type_parameters(type_);
    block.add(
        &format!(
            "impl{parameters} {}{arguments_for_type}",
            lang.escape_reserved(type_.name())
        ),
        arguments,
    );
    append_where_and_open(&mut block, type_.where_constraints());
    block.add("%>", ());
    common::emit_properties(&mut block, lang, type_)?;
    if !type_.properties().is_empty() && !type_.methods().is_empty() {
        block.add_line();
    }
    common::emit_methods(&mut block, lang, type_)?;
    block.add("%<}", ());
    block.add_line();
    block.build()
}

fn lower_alias(lang: &Rust, type_: &ValidatedType<'_>) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    preamble(&mut block, lang, type_)?;
    let mut arguments = Vec::new();
    let parameters = type_parameters(type_, &mut arguments);
    arguments.push(Arg::TypeName(
        type_
            .target_type()
            .expect("validated aliases have a target")
            .clone(),
    ));
    block.add(
        &format!(
            "{}type {}{parameters} = %T;",
            lang.render_visibility(type_.modifiers().visibility, DeclarationContext::TopLevel),
            lang.escape_reserved(type_.name())
        ),
        arguments,
    );
    block.add_line();
    block.build()
}

fn lower_newtype(lang: &Rust, type_: &ValidatedType<'_>) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    preamble(&mut block, lang, type_)?;
    let mut arguments = Vec::new();
    let parameters = type_parameters(type_, &mut arguments);
    arguments.push(Arg::TypeName(
        type_
            .target_type()
            .expect("validated newtypes have a target")
            .clone(),
    ));
    block.add(
        &format!(
            "{}struct {}{parameters}(%T)",
            lang.render_visibility(type_.modifiers().visibility, DeclarationContext::TopLevel),
            lang.escape_reserved(type_.name())
        ),
        arguments,
    );
    if type_.where_constraints().is_empty() {
        block.add(";", ());
        block.add_line();
    } else {
        append_where_constraints(&mut block, type_.where_constraints());
        block.add(";", ());
        block.add_line();
    }
    block.build()
}

fn type_parameters(type_: &ValidatedType<'_>, arguments: &mut Vec<Arg>) -> String {
    if type_.type_params().is_empty() {
        return String::new();
    }
    let mut format = String::from("<");
    let mut first = true;
    for parameter in type_
        .type_params()
        .iter()
        .filter(|parameter| parameter.is_lifetime())
        .chain(
            type_
                .type_params()
                .iter()
                .filter(|parameter| !parameter.is_lifetime()),
        )
    {
        if !first {
            format.push_str(", ");
        }
        first = false;
        format.push_str(parameter.name());
        if !parameter.bounds().is_empty() {
            format.push_str(": ");
            for (bound_index, bound) in parameter.bounds().iter().enumerate() {
                if bound_index > 0 {
                    format.push_str(" + ");
                }
                format.push_str("%T");
                arguments.push(Arg::TypeName(bound.clone()));
            }
        }
    }
    format.push('>');
    format
}

fn preamble(
    block: &mut CodeBlockBuilder,
    lang: &Rust,
    type_: &ValidatedType<'_>,
) -> Result<(), SigilStitchError> {
    common::emit_doc(block, lang, type_);
    common::emit_structured_annotations(block, type_, "#[", "]")?;
    common::emit_raw_annotations(block, type_);
    Ok(())
}

fn append_where_and_open(block: &mut CodeBlockBuilder, constraints: &[WhereConstraint]) {
    if constraints.is_empty() {
        block.add(" {", ());
        block.add_line();
        return;
    }
    append_where_constraints(block, constraints);
    block.add("{", ());
    block.add_line();
}

fn append_where_constraints(block: &mut CodeBlockBuilder, constraints: &[WhereConstraint]) {
    block.add_line();
    block.add("where", ());
    block.add_line();
    block.add("%>", ());
    for constraint in constraints {
        block.add("%T: ", constraint.subject().clone());
        for (index, bound) in constraint.bounds().iter().enumerate() {
            if index > 0 {
                block.add(" + ", ());
            }
            block.add("%T", bound.clone());
        }
        block.add(",", ());
        block.add_line();
    }
    block.add("%<", ());
}

fn bare_type_parameters(type_: &ValidatedType<'_>) -> String {
    if type_.type_params().is_empty() {
        return String::new();
    }
    let names = type_
        .type_params()
        .iter()
        .filter(|parameter| parameter.is_lifetime())
        .chain(
            type_
                .type_params()
                .iter()
                .filter(|parameter| !parameter.is_lifetime()),
        )
        .map(|parameter| parameter.name())
        .collect::<Vec<_>>()
        .join(", ");
    format!("<{names}>")
}

fn invalid(type_: TypeIntent<'_>, reason: &str) -> SigilStitchError {
    SigilStitchError::InvalidTypeDeclaration {
        type_name: type_.name().to_string(),
        reason: reason.to_string(),
    }
}

fn invalid_parameter(type_: TypeIntent<'_>, parameter: &str, reason: &str) -> SigilStitchError {
    SigilStitchError::InvalidTypeParameter {
        type_name: type_.name().to_string(),
        parameter_name: parameter.to_string(),
        reason: reason.to_string(),
    }
}
