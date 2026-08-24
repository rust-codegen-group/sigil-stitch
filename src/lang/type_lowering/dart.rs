//! Dart-owned complete type-declaration grammar.

#![deny(deprecated)]

use crate::code_block::{Arg, CodeBlock};
use crate::error::SigilStitchError;
use crate::lang::RendererLang;
use crate::lang::dart::Dart;
use crate::spec::modifiers::{TypeKind, Visibility};
use crate::spec::type_spec::{TypeIntent, ValidatedType};

use super::common;

fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|ch| ch == '_' || ch == '$' || unicode_ident::is_xid_start(ch))
        && chars.all(|ch| ch == '$' || unicode_ident::is_xid_continue(ch))
}

pub(crate) fn validate(lang: &Dart, type_: TypeIntent<'_>) -> Result<(), SigilStitchError> {
    common::validate_declaration(
        type_,
        lang.file_extension(),
        is_identifier,
        &[Visibility::Inherited],
        &[TypeKind::Class, TypeKind::Struct],
    )?;
    if lang.reserved_words().contains(&type_.name()) {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: type_.name().to_string(),
            reason: "Dart reserves this declaration identifier".to_string(),
        });
    }
    common::validate_ordinary_type_parameters(
        type_,
        lang.file_extension(),
        is_identifier,
        lang.reserved_words(),
    )?;
    for parameter in type_.type_params() {
        if parameter.bounds().len() > 1 || !parameter.context_bounds().is_empty() {
            return Err(SigilStitchError::InvalidTypeParameter {
                type_name: type_.name().to_string(),
                parameter_name: parameter.name().to_string(),
                reason: "Dart type parameters accept at most one direct upper bound".to_string(),
            });
        }
    }
    if !type_.where_constraints().is_empty() {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: type_.name().to_string(),
            reason: "Dart type constraints must be attached directly to a declared type parameter"
                .to_string(),
        });
    }
    if type_.nominal_super_types().len() > 1 {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: type_.name().to_string(),
            reason: "Dart declarations may extend at most one superclass".to_string(),
        });
    }
    if type_.kind() == TypeKind::Enum && type_.variants().is_empty() {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: type_.name().to_string(),
            reason: "Dart enum declarations require at least one value".to_string(),
        });
    }
    Ok(())
}

pub(crate) fn lower(
    lang: &Dart,
    type_: ValidatedType<'_>,
) -> Result<Vec<CodeBlock>, SigilStitchError> {
    if type_.kind() == TypeKind::TypeAlias {
        let mut block = CodeBlock::builder();
        common::emit_doc(&mut block, lang, &type_);
        common::emit_structured_annotations(&mut block, &type_, "@", "")?;
        common::emit_raw_annotations(&mut block, &type_);
        let mut arguments = Vec::new();
        let params = common::type_params(lang, &type_, &mut arguments);
        arguments.push(Arg::TypeName(
            type_.target_type().expect("validated target").clone(),
        ));
        block.add(
            &format!("typedef {}{params} = %T;", type_.name()),
            arguments,
        );
        block.add_line();
        return Ok(vec![block.build()?]);
    }
    let mut block = CodeBlock::builder();
    common::emit_doc(&mut block, lang, &type_);
    common::emit_structured_annotations(&mut block, &type_, "@", "")?;
    common::emit_raw_annotations(&mut block, &type_);
    let mut format = String::new();
    if type_.modifiers().is_abstract
        || matches!(type_.kind(), TypeKind::Interface | TypeKind::Trait)
    {
        format.push_str("abstract ");
    }
    format.push_str(if type_.kind() == TypeKind::Enum {
        "enum "
    } else {
        "class "
    });
    format.push_str(type_.name());
    let mut arguments = Vec::new();
    format.push_str(&common::type_params(lang, &type_, &mut arguments));
    if let Some(base) = type_.nominal_super_types().first() {
        format.push_str(" extends %T");
        arguments.push(Arg::TypeName(base.clone()));
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
    if (variants || fields) && !type_.methods().is_empty() {
        block.add_line();
    }
    common::emit_methods(&mut block, lang, &type_)?;
    common::emit_extra_members(&mut block, &type_);
    block.add("%<}", ());
    block.add_line();
    Ok(vec![block.build()?])
}
