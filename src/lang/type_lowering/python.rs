//! Python-owned complete type-declaration grammar.

#![deny(deprecated)]

use crate::code_block::{CodeBlock, CodeBlockBuilder};
use crate::error::SigilStitchError;
use crate::lang::RendererLang;
use crate::lang::python::Python;
use crate::spec::modifiers::{TypeKind, Visibility};
use crate::spec::type_spec::{TypeIntent, ValidatedType};

use super::common;

pub(crate) fn validate(lang: &Python, type_: TypeIntent<'_>) -> Result<(), SigilStitchError> {
    common::validate_declaration(
        type_,
        lang.file_extension(),
        common::is_identifier,
        &[Visibility::Inherited],
        &[],
    )?;
    if lang.reserved_words().contains(&type_.name()) {
        return Err(invalid(
            type_,
            "Python reserves this declaration identifier",
        ));
    }
    for parameter in type_.type_params() {
        if parameter.is_lifetime()
            || !common::is_identifier(parameter.name())
            || lang.reserved_words().contains(&parameter.name())
        {
            return Err(SigilStitchError::InvalidTypeParameter {
                type_name: type_.name().to_string(),
                parameter_name: parameter.name().to_string(),
                reason: "Python type parameters require an ordinary non-keyword identifier"
                    .to_string(),
            });
        }
    }
    if matches!(type_.kind(), TypeKind::TypeAlias | TypeKind::Newtype) && !type_.doc().is_empty() {
        return Err(invalid(
            type_,
            "Python aliases and NewType declarations cannot attach a runtime docstring",
        ));
    }
    Ok(())
}

pub(crate) fn lower(
    lang: &Python,
    type_: ValidatedType<'_>,
) -> Result<Vec<CodeBlock>, SigilStitchError> {
    match type_.kind() {
        TypeKind::TypeAlias => return Ok(vec![lower_alias(&type_)?]),
        TypeKind::Newtype => return Ok(vec![lower_newtype(&type_)?]),
        _ => {}
    }

    let mut block = CodeBlock::builder();
    emit_decorators(&mut block, &type_)?;
    block.add(&format!("class {}", type_.name()), ());
    let bases = type_
        .nominal_super_types()
        .iter()
        .chain(type_.implemented_types());
    let mut count = 0;
    for base in bases {
        block.add(if count == 0 { "(" } else { ", " }, ());
        block.add("%T", base.clone());
        count += 1;
    }
    if count > 0 {
        block.add(")", ());
    }
    block.add(":", ());
    block.add_line();
    block.add("%>", ());

    let mut has_body = false;
    if !type_.doc().is_empty() {
        common::emit_doc(&mut block, lang, &type_);
        has_body = true;
    }
    if type_.variants().is_some() {
        common::emit_variants(&mut block, lang, &type_)?;
        has_body = true;
    }
    if type_.fields().is_some() {
        common::emit_fields(&mut block, lang, &type_)?;
        has_body = true;
    }
    if !type_.methods().is_empty() {
        if type_.fields().is_some() || type_.variants().is_some() {
            block.add_line();
        }
        common::emit_methods(&mut block, lang, &type_)?;
        has_body = true;
    }
    if common::emit_extra_members(&mut block, &type_) {
        has_body = true;
    }
    if !has_body {
        block.add("pass", ());
        block.add_line();
    }
    block.add("%<", ());
    Ok(vec![block.build()?])
}

fn lower_alias(type_: &ValidatedType<'_>) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    block.add("type ", ());
    block.add("%L", type_.name());
    if !type_.type_params().is_empty() {
        block.add("[", ());
        for (index, parameter) in type_.type_params().iter().enumerate() {
            if index > 0 {
                block.add(", ", ());
            }
            block.add("%L", parameter.name());
        }
        block.add("]", ());
    }
    block.add(
        " = %T",
        type_
            .target_type()
            .expect("validated aliases have a target")
            .clone(),
    );
    block.add_line();
    block.build()
}

fn lower_newtype(type_: &ValidatedType<'_>) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    block.add(
        &format!("{} = NewType(\"{}\", %T)", type_.name(), type_.name()),
        type_
            .target_type()
            .expect("validated newtypes have a target")
            .clone(),
    );
    block.add_line();
    block.build()
}

fn emit_decorators(
    block: &mut CodeBlockBuilder,
    type_: &ValidatedType<'_>,
) -> Result<(), SigilStitchError> {
    common::emit_structured_annotations(block, type_, "@", "")?;
    common::emit_raw_annotations(block, type_);
    Ok(())
}

fn invalid(type_: TypeIntent<'_>, reason: &str) -> SigilStitchError {
    SigilStitchError::InvalidTypeDeclaration {
        type_name: type_.name().to_string(),
        reason: reason.to_string(),
    }
}
