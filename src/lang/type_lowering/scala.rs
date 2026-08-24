//! Scala-owned complete type-declaration grammar.

#![deny(deprecated)]

use crate::code_block::{Arg, CodeBlock, CodeBlockBuilder};
use crate::error::SigilStitchError;
use crate::lang::scala::Scala;
use crate::lang::{CodeLang, RendererLang};
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};
use crate::spec::parameter_spec::ParameterSpec;
use crate::spec::type_spec::{TypeIntent, ValidatedType};

use super::common;

pub(crate) fn validate(lang: &Scala, type_: TypeIntent<'_>) -> Result<(), SigilStitchError> {
    common::validate_declaration(
        type_,
        lang.file_extension(),
        is_identifier,
        &[
            Visibility::Inherited,
            Visibility::Public,
            Visibility::Private,
        ],
        &[TypeKind::Class],
    )?;
    if lang.reserved_words().contains(&type_.name()) {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: type_.name().to_string(),
            reason: "Scala reserves this declaration identifier".to_string(),
        });
    }
    common::validate_ordinary_type_parameters(
        type_,
        lang.file_extension(),
        is_identifier,
        lang.reserved_words(),
    )?;
    if let Some(parameter) = crate::lang::scala::invalid_raw_type_parameter(type_.type_params()) {
        return Err(SigilStitchError::InvalidTypeParameter {
            type_name: type_.name().to_string(),
            parameter_name: parameter.name().to_string(),
            reason:
                "Scala higher-kinded parameter syntax must be a non-empty balanced bracket suffix"
                    .to_string(),
        });
    }
    let mut constructor_names = std::collections::HashSet::new();
    for parameter in type_.primary_constructor_parameters() {
        if !is_identifier(parameter.name())
            || parameter.name() == "_"
            || lang.reserved_words().contains(&parameter.name())
            || parameter.param_type().is_empty()
            || parameter.is_variadic()
        {
            return Err(SigilStitchError::InvalidTypeDeclaration {
                type_name: type_.name().to_string(),
                reason: format!(
                    "invalid Scala primary-constructor parameter {:?}",
                    parameter.name()
                ),
            });
        }
        if !constructor_names.insert(parameter.name()) {
            return Err(SigilStitchError::InvalidTypeDeclaration {
                type_name: type_.name().to_string(),
                reason: format!(
                    "duplicate Scala primary-constructor parameter {:?}",
                    parameter.name()
                ),
            });
        }
        if parameter.is_property() && parameter.is_mutable_property() {
            return Err(SigilStitchError::InvalidTypeDeclaration {
                type_name: type_.name().to_string(),
                reason: format!(
                    "primary-constructor parameter {:?} cannot be both val and var",
                    parameter.name()
                ),
            });
        }
        if parameter.default_value().is_some_and(CodeBlock::is_empty) {
            return Err(SigilStitchError::InvalidTypeDeclaration {
                type_name: type_.name().to_string(),
                reason: format!(
                    "Scala primary-constructor parameter {:?} has an empty default value",
                    parameter.name()
                ),
            });
        }
    }
    if !type_.where_constraints().is_empty() {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: type_.name().to_string(),
            reason: "Scala type constraints must be attached directly to a declared type parameter"
                .to_string(),
        });
    }
    if matches!(type_.kind(), TypeKind::Class | TypeKind::Struct)
        && type_.nominal_super_types().len() > 1
    {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: type_.name().to_string(),
            reason: "Scala classes may inherit from at most one superclass".to_string(),
        });
    }
    Ok(())
}

pub(crate) fn lower(
    lang: &Scala,
    type_: ValidatedType<'_>,
) -> Result<Vec<CodeBlock>, SigilStitchError> {
    if type_.kind() == TypeKind::TypeAlias {
        let mut block = CodeBlock::builder();
        preamble(&mut block, lang, &type_)?;
        let mut arguments = Vec::new();
        let params = common::type_params(lang, &type_, &mut arguments);
        arguments.push(Arg::TypeName(
            type_.target_type().expect("validated target").clone(),
        ));
        block.add(
            &format!(
                "{}type {}{params} = %T",
                lang.render_visibility(type_.modifiers().visibility, DeclarationContext::TopLevel),
                type_.name()
            ),
            arguments,
        );
        block.add_line();
        return Ok(vec![block.build()?]);
    }
    if type_.kind() == TypeKind::Newtype {
        let mut block = CodeBlock::builder();
        preamble(&mut block, lang, &type_)?;
        let mut arguments = Vec::new();
        let params = common::type_params(lang, &type_, &mut arguments);
        arguments.push(Arg::TypeName(
            type_.target_type().expect("validated target").clone(),
        ));
        block.add(
            &format!(
                "{}class {}{params}(val value: %T)",
                lang.render_visibility(type_.modifiers().visibility, DeclarationContext::TopLevel),
                type_.name()
            ),
            arguments,
        );
        block.add_line();
        return Ok(vec![block.build()?]);
    }
    let mut block = CodeBlock::builder();
    preamble(&mut block, lang, &type_)?;
    let mut format = String::new();
    format.push_str(
        lang.render_visibility(type_.modifiers().visibility, DeclarationContext::TopLevel),
    );
    if type_.modifiers().is_abstract {
        format.push_str("abstract ");
    }
    format.push_str(match type_.kind() {
        TypeKind::Class => "class ",
        TypeKind::Struct => "case class ",
        TypeKind::Interface | TypeKind::Trait => "trait ",
        TypeKind::Enum => "enum ",
        TypeKind::TypeAlias | TypeKind::Newtype => unreachable!(),
    });
    format.push_str(type_.name());
    let mut arguments = Vec::new();
    format.push_str(&common::type_params(lang, &type_, &mut arguments));
    if !type_.primary_constructor_parameters().is_empty() {
        format.push_str("(%L)");
        arguments.push(Arg::Code(primary_constructor(
            type_.primary_constructor_parameters(),
        )?));
    } else if type_.kind() == TypeKind::Struct {
        format.push_str("()");
    }
    let bases: Vec<_> = type_
        .nominal_super_types()
        .iter()
        .chain(type_.implemented_types())
        .collect();
    for (index, base) in bases.into_iter().enumerate() {
        format.push_str(if index == 0 { " extends " } else { " with " });
        format.push_str("%T");
        arguments.push(Arg::TypeName(base.clone()));
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
    block: &mut CodeBlockBuilder,
    lang: &Scala,
    type_: &ValidatedType<'_>,
) -> Result<(), SigilStitchError> {
    common::emit_doc(block, lang, type_);
    common::emit_structured_annotations(block, type_, "@", "")?;
    common::emit_raw_annotations(block, type_);
    Ok(())
}

fn primary_constructor(parameters: &[ParameterSpec]) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    for (index, parameter) in parameters.iter().enumerate() {
        if index > 0 {
            block.add(",%W", ());
        }
        if parameter.is_mutable_property() {
            block.add("var ", ());
        } else if parameter.is_property() {
            block.add("val ", ());
        }
        block.add(
            &format!("{}: %T", parameter.name()),
            parameter.param_type().clone(),
        );
        if let Some(default) = parameter.default_value() {
            block.add(" = %L", default.clone());
        }
    }
    block.build()
}

fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|character| {
        character == '_' || character == '$' || unicode_ident::is_xid_start(character)
    }) && chars.all(|character| character == '$' || unicode_ident::is_xid_continue(character))
}
