//! C-owned complete type-declaration grammar.

#![deny(deprecated)]

use crate::code_block::{Arg, CodeBlock};
use crate::error::SigilStitchError;
use crate::lang::RendererLang;
use crate::lang::c::C;
use crate::spec::modifiers::{TypeKind, Visibility};
use crate::spec::type_spec::{TypeIntent, ValidatedType};
use crate::type_name::TypeName;

use super::common;

pub(crate) fn validate(lang: &C, type_: TypeIntent<'_>) -> Result<(), SigilStitchError> {
    common::validate_declaration(
        type_,
        lang.file_extension(),
        common::is_identifier,
        &[Visibility::Inherited],
        &[],
    )?;
    if lang.reserved_words().contains(&type_.name()) {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: type_.name().to_string(),
            reason: "C reserves this declaration identifier".to_string(),
        });
    }
    if matches!(
        type_.kind(),
        TypeKind::Class | TypeKind::Struct | TypeKind::Interface | TypeKind::Trait
    ) && type_.fields().is_empty()
        && type_.extra_members().is_empty()
    {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: type_.name().to_string(),
            reason: "C does not permit an empty record declaration".to_string(),
        });
    }
    if type_.kind() == TypeKind::Enum && type_.variants().is_empty() {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: type_.name().to_string(),
            reason: "C does not permit an empty enum declaration".to_string(),
        });
    }
    Ok(())
}

pub(crate) fn lower(
    lang: &C,
    type_: ValidatedType<'_>,
) -> Result<Vec<CodeBlock>, SigilStitchError> {
    if type_.kind() == TypeKind::TypeAlias {
        return Ok(vec![lower_alias(lang, &type_)?]);
    }
    let mut block = CodeBlock::builder();
    common::emit_doc(&mut block, lang, &type_);
    common::emit_raw_annotations(&mut block, &type_);
    let keyword = if type_.kind() == TypeKind::Enum {
        "enum"
    } else {
        "struct"
    };
    let mut format = keyword.to_string();
    let mut arguments = Vec::new();
    for annotation in type_.annotation_specs() {
        format.push_str(" %L");
        arguments.push(Arg::Code(
            annotation.emit_with_syntax("__attribute__((", "))")?,
        ));
    }
    format.push(' ');
    format.push_str(type_.name());
    format.push_str(" {");
    block.add(&format, arguments);
    block.add_line();
    block.add("%>", ());
    if type_.kind() == TypeKind::Enum {
        common::emit_variants(&mut block, lang, &type_)?;
    } else {
        common::emit_fields(&mut block, lang, &type_)?;
    }
    common::emit_extra_members(&mut block, &type_);
    block.add("%<};", ());
    block.add_line();
    Ok(vec![block.build()?])
}

fn lower_alias(lang: &C, type_: &ValidatedType<'_>) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    common::emit_doc(&mut block, lang, type_);
    let target = type_
        .target_type()
        .expect("validated aliases have a target");
    match target {
        TypeName::Function {
            params,
            return_type,
        } => {
            let format = format!(
                "typedef %T (*{})({});",
                type_.name(),
                vec!["%T"; params.len()].join(", ")
            );
            let mut arguments = vec![Arg::TypeName((**return_type).clone())];
            arguments.extend(params.iter().cloned().map(Arg::TypeName));
            block.add(&format, arguments);
        }
        target => {
            block.add(&format!("typedef %T {};", type_.name()), target.clone());
        }
    }
    block.add_line();
    block.build()
}
