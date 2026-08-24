//! PHP-owned complete type-declaration grammar.

#![deny(deprecated)]

use crate::code_block::{Arg, CodeBlock};
use crate::error::SigilStitchError;
use crate::lang::RendererLang;
use crate::lang::php::Php;
use crate::spec::modifiers::{TypeKind, Visibility};
use crate::spec::type_spec::{TypeIntent, ValidatedType};

use super::common;

pub(crate) fn validate(lang: &Php, type_: TypeIntent<'_>) -> Result<(), SigilStitchError> {
    common::validate_declaration(
        type_,
        lang.file_extension(),
        common::is_identifier,
        &[Visibility::Inherited, Visibility::Public],
        &[TypeKind::Class, TypeKind::Struct],
    )?;
    if lang
        .reserved_words()
        .iter()
        .any(|word| word.eq_ignore_ascii_case(type_.name()))
    {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: type_.name().to_string(),
            reason: "PHP reserves this declaration identifier".to_string(),
        });
    }
    if matches!(type_.kind(), TypeKind::Class | TypeKind::Struct)
        && type_.nominal_super_types().len() > 1
    {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: type_.name().to_string(),
            reason: "PHP classes may extend at most one base class".to_string(),
        });
    }
    Ok(())
}

pub(crate) fn lower(
    lang: &Php,
    type_: ValidatedType<'_>,
) -> Result<Vec<CodeBlock>, SigilStitchError> {
    if type_.kind() == TypeKind::Newtype {
        let mut block = CodeBlock::builder();
        preamble(&mut block, lang, &type_)?;
        let target = type_.target_type().expect("validated target").clone();
        block.add(
            &format!(
                "class {} {{ public function __construct(private %T $value) {{}} }}",
                type_.name()
            ),
            target,
        );
        block.add_line();
        return Ok(vec![block.build()?]);
    }
    let mut block = CodeBlock::builder();
    preamble(&mut block, lang, &type_)?;
    let mut format = String::new();
    if type_.modifiers().is_abstract {
        format.push_str("abstract ");
    }
    format.push_str(match type_.kind() {
        TypeKind::Class | TypeKind::Struct => "class ",
        TypeKind::Interface => "interface ",
        TypeKind::Trait => "trait ",
        TypeKind::Enum => "enum ",
        TypeKind::TypeAlias | TypeKind::Newtype => unreachable!(),
    });
    format.push_str(type_.name());
    let mut arguments = Vec::new();
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

fn preamble(
    block: &mut crate::code_block::CodeBlockBuilder,
    lang: &Php,
    type_: &ValidatedType<'_>,
) -> Result<(), SigilStitchError> {
    common::emit_doc(block, lang, type_);
    common::emit_structured_annotations(block, type_, "#[", "]")?;
    common::emit_raw_annotations(block, type_);
    Ok(())
}
