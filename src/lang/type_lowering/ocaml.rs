//! OCaml-owned complete type-declaration grammar.

#![deny(deprecated)]

use crate::code_block::{CodeBlock, CodeBlockBuilder};
use crate::error::SigilStitchError;
use crate::lang::RendererLang;
use crate::lang::ocaml::OCaml;
use crate::spec::modifiers::{TypeKind, Visibility};
use crate::spec::type_spec::{TypeIntent, ValidatedType};

use super::common;

pub(crate) fn validate(lang: &OCaml, type_: TypeIntent<'_>) -> Result<(), SigilStitchError> {
    common::validate_declaration(
        type_,
        lang.file_extension(),
        common::is_identifier,
        &[Visibility::Inherited],
        &[],
    )?;
    if type_.name() == "_"
        || !is_lowercase_identifier(type_.name())
        || lang.reserved_words().contains(&type_.name())
    {
        return Err(invalid(
            type_,
            "OCaml type declarations require a lowercase non-keyword identifier",
        ));
    }
    for parameter in type_.type_params() {
        let name = parameter
            .name()
            .strip_prefix('\'')
            .unwrap_or(parameter.name());
        if parameter.is_lifetime()
            || name == "_"
            || !is_lowercase_identifier(name)
            || lang.reserved_words().contains(&name)
        {
            return Err(SigilStitchError::InvalidTypeParameter {
                type_name: type_.name().to_string(),
                parameter_name: parameter.name().to_string(),
                reason: "OCaml type parameters require a lowercase type-variable identifier"
                    .to_string(),
            });
        }
    }
    if matches!(type_.kind(), TypeKind::Struct | TypeKind::Class)
        && type_.fields().is_empty()
        && type_.extra_members().is_empty()
    {
        return Err(invalid(type_, "OCaml does not permit an empty record type"));
    }
    if !type_.is_closed_sum()
        && type_.kind() == TypeKind::Enum
        && type_.variants().is_empty()
        && type_.extra_members().is_empty()
    {
        return Err(invalid(
            type_,
            "OCaml does not permit an empty variant type",
        ));
    }
    Ok(())
}

pub(crate) fn lower(
    lang: &OCaml,
    type_: ValidatedType<'_>,
) -> Result<Vec<CodeBlock>, SigilStitchError> {
    let mut block = CodeBlock::builder();
    common::emit_doc(&mut block, lang, &type_);
    block.add("type ", ());
    emit_parameters(&mut block, &type_);
    block.add("%L =", type_.name());

    match type_.kind() {
        TypeKind::TypeAlias => {
            block.add(
                " %T",
                type_
                    .target_type()
                    .expect("validated aliases have a target")
                    .clone(),
            );
            block.add_line();
        }
        TypeKind::Struct | TypeKind::Class => {
            block.add_line();
            block.add("%>", ());
            block.add("{", ());
            block.add_line();
            block.add("%>", ());
            common::emit_fields(&mut block, lang, &type_)?;
            common::emit_extra_members(&mut block, &type_);
            block.add("%<}", ());
            block.add_line();
            block.add("%<", ());
        }
        TypeKind::Enum => {
            if type_.is_closed_sum()
                && type_
                    .variants()
                    .is_some_and(|variants| variants.variants().is_empty())
            {
                block.add(" |", ());
                block.add_line();
                return Ok(vec![block.build()?]);
            }
            block.add_line();
            block.add("%>", ());
            common::emit_variants(&mut block, lang, &type_)?;
            common::emit_extra_members(&mut block, &type_);
            block.add("%<", ());
        }
        TypeKind::Interface | TypeKind::Trait | TypeKind::Newtype => unreachable!(),
    }
    Ok(vec![block.build()?])
}

fn emit_parameters(block: &mut CodeBlockBuilder, type_: &ValidatedType<'_>) {
    match type_.type_params() {
        [] => {}
        [parameter] => {
            block.add("%L ", ocaml_parameter(parameter.name()));
        }
        parameters => {
            block.add("(", ());
            for (index, parameter) in parameters.iter().enumerate() {
                if index > 0 {
                    block.add(", ", ());
                }
                block.add("%L", ocaml_parameter(parameter.name()));
            }
            block.add(") ", ());
        }
    }
}

fn ocaml_parameter(name: &str) -> String {
    if name.starts_with('\'') {
        name.to_string()
    } else {
        format!("'{name}")
    }
}

fn is_lowercase_identifier(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|character| character == '_' || character.is_lowercase())
        && name
            .chars()
            .all(|character| character == '_' || character == '\'' || character.is_alphanumeric())
}

fn invalid(type_: TypeIntent<'_>, reason: &str) -> SigilStitchError {
    SigilStitchError::InvalidTypeDeclaration {
        type_name: type_.name().to_string(),
        reason: reason.to_string(),
    }
}
