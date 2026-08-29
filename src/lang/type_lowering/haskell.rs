//! Haskell-owned complete type-declaration grammar.

#![deny(deprecated)]

use crate::code_block::{CodeBlock, CodeBlockBuilder};
use crate::error::SigilStitchError;
use crate::lang::RendererLang;
use crate::lang::haskell::Haskell;
use crate::spec::modifiers::{TypeKind, Visibility};
use crate::spec::type_spec::{TypeIntent, ValidatedType};

use super::common;

pub(crate) fn validate(lang: &Haskell, type_: TypeIntent<'_>) -> Result<(), SigilStitchError> {
    common::validate_declaration(
        type_,
        lang.file_extension(),
        common::is_identifier,
        &[Visibility::Inherited],
        &[],
    )?;
    if !starts_uppercase(type_.name()) || lang.reserved_words().contains(&type_.name()) {
        return Err(invalid(
            type_,
            "Haskell type constructors and classes require an uppercase non-keyword name",
        ));
    }
    for parameter in type_.type_params() {
        if parameter.is_lifetime()
            || !starts_lowercase_identifier(parameter.name())
            || lang.reserved_words().contains(&parameter.name())
        {
            return Err(SigilStitchError::InvalidTypeParameter {
                type_name: type_.name().to_string(),
                parameter_name: parameter.name().to_string(),
                reason: "Haskell type variables require a lowercase non-keyword identifier"
                    .to_string(),
            });
        }
    }
    for constraint in type_.where_constraints() {
        let declared = constraint.parameter_subject_name().is_some_and(|subject| {
            type_
                .type_params()
                .iter()
                .any(|parameter| parameter.name() == subject)
        });
        if !declared {
            return Err(SigilStitchError::InvalidTypeParameter {
                type_name: type_.name().to_string(),
                parameter_name: format!("{:?}", constraint.subject()),
                reason: "Haskell declaration constraints must target a declared type variable"
                    .to_string(),
            });
        }
    }
    if matches!(type_.kind(), TypeKind::Struct | TypeKind::Class)
        && !type_.fields().is_empty()
        && !type_.variants().is_empty()
    {
        return Err(invalid(
            type_,
            "a Haskell data declaration cannot combine an implicit record constructor with explicit variants",
        ));
    }
    if type_.kind() == TypeKind::Enum
        && type_.variants().is_empty()
        && (type_.is_closed_sum() || type_.extra_members().is_empty())
    {
        return Err(invalid(
            type_,
            "a Haskell enum declaration requires at least one constructor",
        ));
    }
    Ok(())
}

pub(crate) fn lower(
    lang: &Haskell,
    type_: ValidatedType<'_>,
) -> Result<Vec<CodeBlock>, SigilStitchError> {
    let block = match type_.kind() {
        TypeKind::TypeAlias => lower_alias(lang, &type_)?,
        TypeKind::Newtype => lower_newtype(lang, &type_)?,
        TypeKind::Trait | TypeKind::Interface => lower_class(lang, &type_)?,
        TypeKind::Struct | TypeKind::Class | TypeKind::Enum => lower_data(lang, &type_)?,
    };
    Ok(vec![block])
}

fn lower_alias(lang: &Haskell, type_: &ValidatedType<'_>) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    common::emit_doc(&mut block, lang, type_);
    block.add("type ", ());
    emit_name_and_parameters(&mut block, type_);
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

fn lower_newtype(lang: &Haskell, type_: &ValidatedType<'_>) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    common::emit_doc(&mut block, lang, type_);
    block.add("newtype ", ());
    emit_context(&mut block, type_);
    emit_name_and_parameters(&mut block, type_);
    block.add(&format!(" = {} ", type_.name()), ());
    let target = type_
        .target_type()
        .expect("validated newtypes have a target");
    if crate::type_name_render::is_compound_type(target) {
        block.add("(%T)", target.clone());
    } else {
        block.add("%T", target.clone());
    }
    block.add_line();
    emit_indented_deriving(&mut block, type_);
    block.build()
}

fn lower_class(lang: &Haskell, type_: &ValidatedType<'_>) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    common::emit_doc(&mut block, lang, type_);
    block.add("class ", ());
    emit_context(&mut block, type_);
    emit_name_and_parameters(&mut block, type_);
    block.add(" where", ());
    block.add_line();
    block.add("%>", ());
    common::emit_methods(&mut block, lang, type_)?;
    common::emit_extra_members(&mut block, type_);
    block.add("%<", ());
    block.build()
}

fn lower_data(lang: &Haskell, type_: &ValidatedType<'_>) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    common::emit_doc(&mut block, lang, type_);
    block.add("data ", ());
    emit_context(&mut block, type_);
    emit_name_and_parameters(&mut block, type_);

    let has_rhs =
        type_.fields().is_some() || type_.variants().is_some() || !type_.extra_members().is_empty();
    if !has_rhs {
        block.add(&format!(" = {}", type_.name()), ());
        block.add_line();
        emit_indented_deriving(&mut block, type_);
        return block.build();
    }

    block.add(" =", ());
    block.add_line();
    block.add("%>", ());
    if type_.fields().is_some() {
        block.add(&format!("{} {{", type_.name()), ());
        block.add_line();
        block.add("%>", ());
        common::emit_fields(&mut block, lang, type_)?;
        block.add("%<}", ());
        block.add_line();
    } else {
        common::emit_variants(&mut block, lang, type_)?;
    }
    common::emit_extra_members(&mut block, type_);
    emit_deriving(&mut block, type_);
    block.add("%<", ());
    block.build()
}

fn emit_name_and_parameters(block: &mut CodeBlockBuilder, type_: &ValidatedType<'_>) {
    block.add("%L", type_.name());
    for parameter in type_.type_params() {
        block.add(" ", ());
        block.add("%L", parameter.name());
    }
}

fn emit_context(block: &mut CodeBlockBuilder, type_: &ValidatedType<'_>) {
    let count = type_
        .type_params()
        .iter()
        .map(|parameter| parameter.bounds().len() + parameter.context_bounds().len())
        .sum::<usize>()
        + type_
            .where_constraints()
            .iter()
            .map(|constraint| constraint.bounds().len())
            .sum::<usize>();
    if count == 0 {
        return;
    }
    if count > 1 {
        block.add("(", ());
    }
    let mut index = 0;
    for parameter in type_.type_params() {
        for bound in parameter.bounds().iter().chain(parameter.context_bounds()) {
            if index > 0 {
                block.add(", ", ());
            }
            block.add("%T ", bound.clone());
            block.add("%L", parameter.name());
            index += 1;
        }
    }
    for constraint in type_.where_constraints() {
        for bound in constraint.bounds() {
            if index > 0 {
                block.add(", ", ());
            }
            block.add("%T %T", (bound.clone(), constraint.subject().clone()));
            index += 1;
        }
    }
    if count > 1 {
        block.add(")", ());
    }
    block.add(" => ", ());
}

fn emit_indented_deriving(block: &mut CodeBlockBuilder, type_: &ValidatedType<'_>) {
    if type_.implemented_types().is_empty() {
        return;
    }
    block.add("%>", ());
    block.add("%>", ());
    emit_deriving(block, type_);
    block.add("%<", ());
    block.add("%<", ());
}

fn emit_deriving(block: &mut CodeBlockBuilder, type_: &ValidatedType<'_>) {
    if type_.implemented_types().is_empty() {
        return;
    }
    block.add("deriving (", ());
    for (index, implemented) in type_.implemented_types().iter().enumerate() {
        if index > 0 {
            block.add(", ", ());
        }
        block.add("%T", implemented.clone());
    }
    block.add(")", ());
    block.add_line();
}

pub(crate) fn starts_uppercase(name: &str) -> bool {
    name.chars().next().is_some_and(char::is_uppercase)
        && name
            .chars()
            .all(|character| character == '_' || character == '\'' || character.is_alphanumeric())
}

pub(crate) fn starts_lowercase_identifier(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|character| character.is_lowercase())
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
