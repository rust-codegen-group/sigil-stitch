//! Ruby-owned complete type-declaration grammar.

#![deny(deprecated)]

use crate::code_block::{Arg, CodeBlock};
use crate::error::SigilStitchError;
use crate::lang::RendererLang;
use crate::lang::ruby::Ruby;
use crate::spec::modifiers::{TypeKind, Visibility};
use crate::spec::type_spec::{TypeIntent, ValidatedType};

use super::common;

pub(crate) fn validate(lang: &Ruby, type_: TypeIntent<'_>) -> Result<(), SigilStitchError> {
    common::validate_declaration(
        type_,
        lang.file_extension(),
        common::is_identifier,
        &[Visibility::Inherited],
        &[],
    )?;
    if !is_constant_name(type_.name()) || lang.reserved_words().contains(&type_.name()) {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: type_.name().to_string(),
            reason: "Ruby class and module declarations require a constant name".to_string(),
        });
    }
    if !type_.annotation_specs().is_empty() {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: type_.name().to_string(),
            reason: "Ruby has no structured declaration annotation syntax; use an opaque target-local annotation block"
                .to_string(),
        });
    }
    if type_.nominal_super_types().len() > 1 {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: type_.name().to_string(),
            reason: "Ruby classes may inherit from at most one superclass".to_string(),
        });
    }
    Ok(())
}

pub(crate) fn lower(
    lang: &Ruby,
    type_: ValidatedType<'_>,
) -> Result<Vec<CodeBlock>, SigilStitchError> {
    let mut block = CodeBlock::builder();
    common::emit_doc(&mut block, lang, &type_);
    common::emit_raw_annotations(&mut block, &type_);
    let keyword = if matches!(type_.kind(), TypeKind::Interface | TypeKind::Trait) {
        "module"
    } else {
        "class"
    };
    let mut format = format!("{keyword} {}", type_.name());
    let mut arguments = Vec::new();
    if let Some(base) = type_.nominal_super_types().first() {
        format.push_str(" < %T");
        arguments.push(Arg::TypeName(base.clone()));
    }
    block.add(&format, arguments);
    block.add_line();
    block.add("%>", ());
    let variants = common::emit_variants(&mut block, lang, &type_)?;
    if variants && !type_.methods().is_empty() {
        block.add_line();
    }
    common::emit_methods(&mut block, lang, &type_)?;
    common::emit_extra_members(&mut block, &type_);
    block.add("%<end", ());
    block.add_line();
    Ok(vec![block.build()?])
}

fn is_constant_name(name: &str) -> bool {
    name.chars().next().is_some_and(char::is_uppercase)
        && name
            .chars()
            .all(|character| character == '_' || unicode_ident::is_xid_continue(character))
}
