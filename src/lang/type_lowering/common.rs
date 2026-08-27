//! Policy-free helpers used by complete language-local type lowerers.

use crate::code_block::{Arg, CodeBlockBuilder};
use crate::error::SigilStitchError;
use crate::lang::CodeLang;
use crate::spec::modifiers::{TypeKind, Visibility};
use crate::spec::type_spec::{TypeIntent, ValidatedType};
use crate::spec::where_spec::{WhereConstraint, render_type_params_for};

pub(super) fn validate_declaration(
    type_: TypeIntent<'_>,
    language: &str,
    is_valid_identifier: fn(&str) -> bool,
    visibilities: &[Visibility],
    abstract_kinds: &[TypeKind],
) -> Result<(), SigilStitchError> {
    if !is_valid_identifier(type_.name()) {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: type_.name().to_string(),
            reason: format!("{language} requires a valid declaration identifier"),
        });
    }
    if !visibilities.contains(&type_.modifiers().visibility) {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: type_.name().to_string(),
            reason: format!(
                "{language} does not support {:?} visibility for this type declaration",
                type_.modifiers().visibility
            ),
        });
    }
    if type_.modifiers().is_abstract && !abstract_kinds.contains(&type_.kind()) {
        return Err(SigilStitchError::InvalidAbstractType {
            language: language.to_string(),
            kind: type_.kind(),
            type_name: type_.name().to_string(),
        });
    }
    Ok(())
}

pub(super) fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|first| first == '_' || unicode_ident::is_xid_start(first))
        && chars.all(unicode_ident::is_xid_continue)
}

/// Validate the ordinary (non-lifetime) parameter shape shared by languages
/// whose target grammar uses identifier-named type parameters.
pub(super) fn validate_ordinary_type_parameters(
    type_: TypeIntent<'_>,
    language: &str,
    is_valid_identifier: fn(&str) -> bool,
    reserved_words: &[&str],
) -> Result<(), SigilStitchError> {
    for parameter in type_.type_params() {
        if parameter.is_lifetime()
            || !is_valid_identifier(parameter.name())
            || reserved_words.contains(&parameter.name())
        {
            return Err(SigilStitchError::InvalidTypeParameter {
                type_name: type_.name().to_string(),
                parameter_name: parameter.name().to_string(),
                reason: format!(
                    "{language} type parameters require an ordinary non-keyword identifier"
                ),
            });
        }
    }
    Ok(())
}

/// Validate explicit constraints for a grammar that attaches every subject to
/// one of the declaration's named type parameters.
pub(super) fn validate_constraint_subjects(
    type_: TypeIntent<'_>,
    language: &str,
    constraints: &[WhereConstraint],
) -> Result<(), SigilStitchError> {
    for constraint in constraints {
        let Some(subject) = constraint.subject().simple_name() else {
            return Err(SigilStitchError::InvalidTypeParameter {
                type_name: type_.name().to_string(),
                parameter_name: format!("{:?}", constraint.subject()),
                reason: format!(
                    "{language} declaration constraints must target a declared type parameter"
                ),
            });
        };
        if !type_
            .type_params()
            .iter()
            .any(|parameter| parameter.name() == subject)
        {
            return Err(SigilStitchError::InvalidTypeParameter {
                type_name: type_.name().to_string(),
                parameter_name: subject.to_string(),
                reason: format!(
                    "{language} declaration constraints must target a declared type parameter"
                ),
            });
        }
    }
    Ok(())
}

pub(super) fn emit_doc<L: CodeLang + ?Sized>(
    block: &mut CodeBlockBuilder,
    lang: &L,
    type_: &ValidatedType<'_>,
) {
    if type_.doc().is_empty() {
        return;
    }
    let lines: Vec<&str> = type_.doc().iter().map(String::as_str).collect();
    block.add("%L", lang.render_doc_comment(&lines));
    block.add_line();
}

pub(super) fn emit_structured_annotations(
    block: &mut CodeBlockBuilder,
    type_: &ValidatedType<'_>,
    prefix: &str,
    suffix: &str,
) -> Result<(), SigilStitchError> {
    for annotation in type_.annotation_specs() {
        block.add_code(annotation.emit_with_syntax(prefix, suffix)?);
        block.add_line();
    }
    Ok(())
}

pub(super) fn emit_raw_annotations(block: &mut CodeBlockBuilder, type_: &ValidatedType<'_>) {
    for annotation in type_.annotations() {
        block.add_code(annotation.clone());
        block.add_line();
    }
}

pub(super) fn type_params<L: CodeLang + ?Sized>(
    lang: &L,
    type_: &ValidatedType<'_>,
    arguments: &mut Vec<Arg>,
) -> String {
    render_type_params_for(type_.type_params(), lang, arguments)
}

pub(super) fn emit_fields<L: CodeLang + ?Sized>(
    block: &mut CodeBlockBuilder,
    lang: &L,
    type_: &ValidatedType<'_>,
) -> Result<bool, SigilStitchError> {
    if let Some(fields) = type_.fields() {
        block.add_code(lang.lower_fields(fields.clone())?);
        Ok(true)
    } else {
        Ok(false)
    }
}

pub(super) fn emit_properties<L: CodeLang + ?Sized>(
    block: &mut CodeBlockBuilder,
    lang: &L,
    type_: &ValidatedType<'_>,
) -> Result<bool, SigilStitchError> {
    for (index, property) in type_.properties().iter().enumerate() {
        if index > 0 {
            block.add_line();
        }
        for property_block in lang.lower_property(property.clone())? {
            block.add_code(property_block);
        }
    }
    Ok(!type_.properties().is_empty())
}

pub(super) fn emit_methods<L: CodeLang + ?Sized>(
    block: &mut CodeBlockBuilder,
    lang: &L,
    type_: &ValidatedType<'_>,
) -> Result<bool, SigilStitchError> {
    for (index, method) in type_.methods().iter().enumerate() {
        if index > 0 {
            block.add_line();
        }
        block.add_code(lang.lower_function(*method)?);
    }
    Ok(!type_.methods().is_empty())
}

pub(super) fn emit_variants<L: CodeLang + ?Sized>(
    block: &mut CodeBlockBuilder,
    lang: &L,
    type_: &ValidatedType<'_>,
) -> Result<bool, SigilStitchError> {
    if let Some(variants) = type_.variants() {
        block.add_code(lang.lower_variants(variants.clone())?);
        Ok(true)
    } else {
        Ok(false)
    }
}

pub(super) fn emit_extra_members(block: &mut CodeBlockBuilder, type_: &ValidatedType<'_>) -> bool {
    for member in type_.extra_members() {
        block.add_code(member.clone());
    }
    !type_.extra_members().is_empty()
}
