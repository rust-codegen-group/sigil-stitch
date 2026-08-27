//! Swift-owned complete type-declaration grammar.

#![deny(deprecated)]

use crate::code_block::{Arg, CodeBlock};
use crate::error::SigilStitchError;
use crate::lang::swift::Swift;
use crate::lang::{CodeLang, RendererLang};
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};
use crate::spec::type_spec::{TypeIntent, ValidatedType};

use super::common;

pub(crate) fn validate(lang: &Swift, type_: TypeIntent<'_>) -> Result<(), SigilStitchError> {
    common::validate_declaration(
        type_,
        lang.file_extension(),
        common::is_identifier,
        &[
            Visibility::Inherited,
            Visibility::Public,
            Visibility::Private,
            Visibility::PublicCrate,
        ],
        &[],
    )?;
    if lang.reserved_words().contains(&type_.name()) {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: type_.name().to_string(),
            reason: "Swift reserves this declaration identifier".to_string(),
        });
    }
    common::validate_ordinary_type_parameters(
        type_,
        lang.file_extension(),
        common::is_identifier,
        lang.reserved_words(),
    )?;
    for parameter in type_.type_params() {
        if !parameter.context_bounds().is_empty() {
            return Err(SigilStitchError::InvalidTypeParameter {
                type_name: type_.name().to_string(),
                parameter_name: parameter.name().to_string(),
                reason: "Swift type declarations do not support context bounds".to_string(),
            });
        }
    }
    common::validate_constraint_subjects(type_, lang.file_extension(), type_.where_constraints())?;
    if type_.kind() == TypeKind::Class && type_.nominal_super_types().len() > 1 {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: type_.name().to_string(),
            reason: "Swift classes may inherit from at most one superclass".to_string(),
        });
    }
    Ok(())
}

pub(crate) fn lower(
    lang: &Swift,
    type_: ValidatedType<'_>,
) -> Result<Vec<CodeBlock>, SigilStitchError> {
    let mut block = CodeBlock::builder();
    common::emit_doc(&mut block, lang, &type_);
    common::emit_structured_annotations(&mut block, &type_, "@", "")?;
    common::emit_raw_annotations(&mut block, &type_);
    let mut format = String::new();
    format.push_str(
        lang.render_visibility(type_.modifiers().visibility, DeclarationContext::TopLevel),
    );
    format.push_str(match type_.kind() {
        TypeKind::Class => "class ",
        TypeKind::Struct => "struct ",
        TypeKind::Interface | TypeKind::Trait => "protocol ",
        TypeKind::Enum => "enum ",
        TypeKind::TypeAlias | TypeKind::Newtype => unreachable!(),
    });
    format.push_str(type_.name());
    let mut arguments = Vec::new();
    format.push_str(&type_parameters(&type_, &mut arguments));
    let bases: Vec<_> = type_
        .nominal_super_types()
        .iter()
        .chain(type_.implemented_types())
        .collect();
    if !bases.is_empty() {
        format.push_str(": ");
        for (index, base) in bases.into_iter().enumerate() {
            if index > 0 {
                format.push_str(", ");
            }
            format.push_str("%T");
            arguments.push(Arg::TypeName(base.clone()));
        }
    }
    if !type_.where_constraints().is_empty() {
        format.push_str(" where ");
        for (constraint_index, constraint) in type_.where_constraints().iter().enumerate() {
            if constraint_index > 0 {
                format.push_str(", ");
            }
            format.push_str("%T: ");
            arguments.push(Arg::TypeName(constraint.subject().clone()));
            for (bound_index, bound) in constraint.bounds().iter().enumerate() {
                if bound_index > 0 {
                    format.push_str(" & ");
                }
                format.push_str("%T");
                arguments.push(Arg::TypeName(bound.clone()));
            }
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

fn type_parameters(type_: &ValidatedType<'_>, arguments: &mut Vec<Arg>) -> String {
    if type_.type_params().is_empty() {
        return String::new();
    }
    let mut format = String::from("<");
    for (index, parameter) in type_.type_params().iter().enumerate() {
        if index > 0 {
            format.push_str(", ");
        }
        format.push_str(parameter.name());
        if !parameter.bounds().is_empty() {
            format.push_str(": ");
            for (bound_index, bound) in parameter.bounds().iter().enumerate() {
                if bound_index > 0 {
                    format.push_str(" & ");
                }
                format.push_str("%T");
                arguments.push(Arg::TypeName(bound.clone()));
            }
        }
    }
    format.push('>');
    format
}
