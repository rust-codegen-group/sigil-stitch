//! JavaScript-owned complete type-declaration grammar.

#![deny(deprecated)]

use crate::code_block::{Arg, CodeBlock};
use crate::error::SigilStitchError;
use crate::lang::RendererLang;
use crate::lang::javascript::JavaScript;
use crate::spec::modifiers::Visibility;
use crate::spec::type_spec::{TypeIntent, ValidatedType};

use super::common;

fn is_identifier(name: &str) -> bool {
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

pub(crate) fn validate(lang: &JavaScript, type_: TypeIntent<'_>) -> Result<(), SigilStitchError> {
    common::validate_declaration(
        type_,
        lang.file_extension(),
        is_identifier,
        &[Visibility::Inherited, Visibility::Public],
        &[],
    )?;
    if lang.reserved_words().contains(&type_.name()) {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: type_.name().to_string(),
            reason: "JavaScript reserves this declaration identifier".to_string(),
        });
    }
    if type_.nominal_super_types().len() > 1 {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: type_.name().to_string(),
            reason: "JavaScript classes may extend at most one base class".to_string(),
        });
    }
    Ok(())
}

pub(crate) fn lower(
    lang: &JavaScript,
    type_: ValidatedType<'_>,
) -> Result<Vec<CodeBlock>, SigilStitchError> {
    let mut block = CodeBlock::builder();
    common::emit_doc(&mut block, lang, &type_);
    common::emit_structured_annotations(&mut block, &type_, "@", "")?;
    common::emit_raw_annotations(&mut block, &type_);
    let mut format = String::new();
    if type_.modifiers().visibility == Visibility::Public {
        format.push_str("export ");
    }
    format.push_str("class ");
    format.push_str(type_.name());
    let mut arguments = Vec::new();
    if let Some(base) = type_.nominal_super_types().first() {
        format.push_str(" extends %T");
        arguments.push(Arg::TypeName(base.clone()));
    }
    format.push_str(" {");
    block.add(&format, arguments);
    block.add_line();
    block.add("%>", ());
    let variants = common::emit_variants(&mut block, lang, &type_)?;
    let fields = common::emit_fields(&mut block, lang, &type_)?;
    let properties = if variants || fields {
        if !type_.properties().is_empty() {
            block.add_line();
        }
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
