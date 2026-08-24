//! C++-owned complete type-declaration grammar.

#![deny(deprecated)]

use crate::code_block::{Arg, CodeBlock, CodeBlockBuilder};
use crate::error::SigilStitchError;
use crate::lang::RendererLang;
use crate::lang::cpp::Cpp;
use crate::spec::modifiers::{TypeKind, Visibility};
use crate::spec::type_spec::{TypeIntent, ValidatedType};

use super::common;

pub(crate) fn validate(lang: &Cpp, type_: TypeIntent<'_>) -> Result<(), SigilStitchError> {
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
            reason: "C++ reserves this declaration identifier".to_string(),
        });
    }
    common::validate_ordinary_type_parameters(
        type_,
        lang.file_extension(),
        common::is_identifier,
        lang.reserved_words(),
    )
}

pub(crate) fn lower(
    lang: &Cpp,
    type_: ValidatedType<'_>,
) -> Result<Vec<CodeBlock>, SigilStitchError> {
    if type_.kind() == TypeKind::TypeAlias {
        let mut block = CodeBlock::builder();
        preamble(&mut block, lang, &type_)?;
        emit_template_declaration(&mut block, &type_);
        let arguments = vec![Arg::TypeName(
            type_.target_type().expect("validated alias target").clone(),
        )];
        block.add(&format!("using {} = %T;", type_.name()), arguments);
        block.add_line();
        return Ok(vec![block.build()?]);
    }

    let mut block = CodeBlock::builder();
    preamble(&mut block, lang, &type_)?;
    emit_template_declaration(&mut block, &type_);
    let keyword = match type_.kind() {
        TypeKind::Struct => "struct",
        TypeKind::Enum => "enum class",
        TypeKind::Class | TypeKind::Interface | TypeKind::Trait => "class",
        TypeKind::TypeAlias | TypeKind::Newtype => unreachable!(),
    };
    let mut format = keyword.to_string();
    let mut arguments = Vec::new();
    for annotation in type_.annotation_specs() {
        format.push_str(" %L");
        arguments.push(Arg::Code(annotation.emit_with_syntax("[[", "]]")?));
    }
    format.push(' ');
    format.push_str(type_.name());
    for (index, super_type) in type_.nominal_super_types().iter().enumerate() {
        format.push_str(if index == 0 {
            " : public "
        } else {
            ", public "
        });
        format.push_str("%T");
        arguments.push(Arg::TypeName(super_type.clone()));
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
    block.add("%<};", ());
    block.add_line();
    Ok(vec![block.build()?])
}

fn emit_template_declaration(block: &mut CodeBlockBuilder, type_: &ValidatedType<'_>) {
    if type_.type_params().is_empty() {
        return;
    }
    block.add("template <", ());
    for (index, parameter) in type_.type_params().iter().enumerate() {
        if index > 0 {
            block.add(", ", ());
        }
        block.add("typename %L", parameter.name());
    }
    block.add(">", ());
    block.add_line();
}

fn preamble(
    block: &mut crate::code_block::CodeBlockBuilder,
    lang: &Cpp,
    type_: &ValidatedType<'_>,
) -> Result<(), SigilStitchError> {
    common::emit_doc(block, lang, type_);
    common::emit_raw_annotations(block, type_);
    Ok(())
}
