//! C#-owned complete type-declaration grammar.

#![deny(deprecated)]

use crate::code_block::{Arg, CodeBlock};
use crate::error::SigilStitchError;
use crate::lang::csharp::CSharp;
use crate::lang::{CodeLang, RendererLang};
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};
use crate::spec::type_spec::{TypeIntent, ValidatedType};

use super::common;

pub(crate) fn validate(lang: &CSharp, type_: TypeIntent<'_>) -> Result<(), SigilStitchError> {
    common::validate_declaration(
        type_,
        lang.file_extension(),
        common::is_identifier,
        &[Visibility::Inherited, Visibility::Public],
        &[TypeKind::Class],
    )?;
    if lang.reserved_words().contains(&type_.name()) {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: type_.name().to_string(),
            reason: "C# reserves this declaration identifier".to_string(),
        });
    }
    common::validate_ordinary_type_parameters(
        type_,
        lang.file_extension(),
        common::is_identifier,
        lang.reserved_words(),
    )?;
    common::validate_constraint_subjects(type_, lang.file_extension(), type_.where_constraints())?;
    if type_.kind() == TypeKind::Class && type_.nominal_super_types().len() > 1 {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: type_.name().to_string(),
            reason: "C# classes may inherit from at most one base class".to_string(),
        });
    }
    Ok(())
}

pub(crate) fn lower(
    lang: &CSharp,
    type_: ValidatedType<'_>,
) -> Result<Vec<CodeBlock>, SigilStitchError> {
    let mut block = CodeBlock::builder();
    common::emit_doc(&mut block, lang, &type_);
    common::emit_structured_annotations(&mut block, &type_, "[", "]")?;
    common::emit_raw_annotations(&mut block, &type_);
    let mut format = String::new();
    format.push_str(
        lang.render_visibility(type_.modifiers().visibility, DeclarationContext::TopLevel),
    );
    if type_.modifiers().is_abstract {
        format.push_str("abstract ");
    }
    format.push_str(match type_.kind() {
        TypeKind::Class => "class ",
        TypeKind::Struct => "struct ",
        TypeKind::Interface | TypeKind::Trait => "interface ",
        TypeKind::Enum => "enum ",
        TypeKind::TypeAlias | TypeKind::Newtype => unreachable!(),
    });
    format.push_str(type_.name());
    let mut arguments = Vec::new();
    format.push_str(&declaration_type_parameters(&type_));
    for (index, base) in type_
        .nominal_super_types()
        .iter()
        .chain(type_.implemented_types())
        .enumerate()
    {
        format.push_str(if index == 0 { " : " } else { ", " });
        format.push_str("%T");
        arguments.push(Arg::TypeName(base.clone()));
    }
    let has_constraints = append_constraints(&mut format, &mut arguments, &type_);
    if !has_constraints {
        format.push_str(" {");
    } else {
        format.push_str("\n{");
    }
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

fn declaration_type_parameters(type_: &ValidatedType<'_>) -> String {
    if type_.type_params().is_empty() {
        return String::new();
    }
    format!(
        "<{}>",
        type_
            .type_params()
            .iter()
            .map(|parameter| parameter.name())
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn append_constraints(
    format: &mut String,
    arguments: &mut Vec<Arg>,
    type_: &ValidatedType<'_>,
) -> bool {
    let mut emitted = false;
    for parameter in type_.type_params() {
        let direct_bounds = parameter.bounds().iter().chain(parameter.context_bounds());
        let explicit_bounds = type_
            .where_constraints()
            .iter()
            .filter(|constraint| constraint.subject().simple_name() == Some(parameter.name()))
            .flat_map(|constraint| constraint.bounds());
        let bounds = direct_bounds.chain(explicit_bounds).collect::<Vec<_>>();
        if bounds.is_empty() {
            continue;
        }
        emitted = true;
        format.push_str("\n    where ");
        format.push_str(parameter.name());
        format.push_str(" : ");
        for (index, bound) in bounds.into_iter().enumerate() {
            if index > 0 {
                format.push_str(", ");
            }
            format.push_str("%T");
            arguments.push(Arg::TypeName(bound.clone()));
        }
    }
    emitted
}
