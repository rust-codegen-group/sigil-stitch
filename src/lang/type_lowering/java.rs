//! Java-owned complete type-declaration grammar.

#![deny(deprecated)]

use crate::code_block::{Arg, CodeBlock};
use crate::error::SigilStitchError;
use crate::lang::java::Java;
use crate::lang::{CodeLang, RendererLang};
use crate::spec::field_spec::{FieldSequenceIntent, FieldSpec};
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};
use crate::spec::type_spec::{TypeIntent, ValidatedType};
use crate::spec::where_spec::{TypeParamSpec, WhereConstraint};
use crate::type_name::TypeName;

use super::common;

pub(crate) fn is_identifier(name: &str) -> bool {
    use unicode_general_category::{GeneralCategory, get_general_category};

    fn is_start(ch: char) -> bool {
        use GeneralCategory::*;
        matches!(
            get_general_category(ch),
            UppercaseLetter
                | LowercaseLetter
                | TitlecaseLetter
                | ModifierLetter
                | OtherLetter
                | LetterNumber
                | CurrencySymbol
                | ConnectorPunctuation
        )
    }

    fn is_continue(ch: char) -> bool {
        use GeneralCategory::*;
        is_start(ch)
            || matches!(
                get_general_category(ch),
                NonspacingMark | SpacingMark | DecimalNumber | Format
            )
    }

    let mut chars = name.chars();
    chars.next().is_some_and(|ch| ch == '$' || is_start(ch))
        && chars.all(|ch| ch == '$' || is_continue(ch))
}

pub(crate) fn validate(lang: &Java, type_: TypeIntent<'_>) -> Result<(), SigilStitchError> {
    common::validate_declaration(
        type_,
        lang.file_extension(),
        is_identifier,
        &[Visibility::Inherited, Visibility::Public],
        &[
            TypeKind::Class,
            TypeKind::Struct,
            TypeKind::Interface,
            TypeKind::Trait,
        ],
    )?;
    if lang.reserved_words().contains(&type_.name()) {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: type_.name().to_string(),
            reason: "Java reserves this declaration identifier".to_string(),
        });
    }
    common::validate_ordinary_type_parameters(
        type_,
        lang.file_extension(),
        is_identifier,
        lang.reserved_words(),
    )?;
    for parameter in type_.type_params() {
        if !parameter.context_bounds().is_empty() {
            return Err(SigilStitchError::InvalidTypeParameter {
                type_name: type_.name().to_string(),
                parameter_name: parameter.name().to_string(),
                reason: "Java type declarations do not support context bounds".to_string(),
            });
        }
    }
    common::validate_constraint_subjects(type_, lang.file_extension(), type_.where_constraints())?;
    if let Some((parameter_name, first, second)) = crate::lang::java::conflicting_bound_erasures(
        type_.type_params(),
        type_.where_constraints(),
    ) {
        return Err(SigilStitchError::InvalidTypeParameter {
            type_name: type_.name().to_string(),
            parameter_name: parameter_name.to_string(),
            reason: format!(
                "Java bounds {first:?} and {second:?} have the same erased type; distinct bounds require pairwise-distinct erasures"
            ),
        });
    }
    if matches!(type_.kind(), TypeKind::Class | TypeKind::Struct)
        && type_.nominal_super_types().len() > 1
    {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: type_.name().to_string(),
            reason: "Java classes may extend at most one superclass".to_string(),
        });
    }
    if type_.is_closed_sum() {
        if type_.variants().is_empty() {
            return Err(SigilStitchError::InvalidTypeDeclaration {
                type_name: type_.name().to_string(),
                reason: "Java sealed closed sums require at least one permitted case".to_string(),
            });
        }
        if !type_.fields().is_empty()
            || !type_.properties().is_empty()
            || !type_.methods().is_empty()
            || !type_.embedded_types().is_empty()
            || !type_.extra_members().is_empty()
            || !type_.primary_constructor_parameters().is_empty()
            || !type_.nominal_super_types().is_empty()
            || !type_.implemented_types().is_empty()
        {
            return Err(SigilStitchError::InvalidTypeDeclaration {
                type_name: type_.name().to_string(),
                reason: "Java closed sums currently support only root documentation, annotations, and cases"
                    .to_string(),
            });
        }
    }
    Ok(())
}

pub(crate) fn lower(
    lang: &Java,
    type_: ValidatedType<'_>,
) -> Result<Vec<CodeBlock>, SigilStitchError> {
    if type_.is_closed_sum() {
        return Ok(vec![lower_closed_sum(lang, &type_)?]);
    }
    let mut block = CodeBlock::builder();
    common::emit_doc(&mut block, lang, &type_);
    common::emit_structured_annotations(&mut block, &type_, "@", "")?;
    common::emit_raw_annotations(&mut block, &type_);
    let mut format = String::new();
    format.push_str(
        lang.render_visibility(type_.modifiers().visibility, DeclarationContext::TopLevel),
    );
    if type_.modifiers().is_abstract {
        format.push_str("abstract ");
    }
    format.push_str(match type_.kind() {
        TypeKind::Class | TypeKind::Struct => "class ",
        TypeKind::Interface | TypeKind::Trait => "interface ",
        TypeKind::Enum => "enum ",
        TypeKind::TypeAlias | TypeKind::Newtype => unreachable!(),
    });
    format.push_str(type_.name());
    let mut arguments = Vec::new();
    format.push_str(&type_parameters(&type_, &mut arguments));
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
    if variants
        && (type_.fields().is_some()
            || !type_.properties().is_empty()
            || !type_.methods().is_empty()
            || !type_.extra_members().is_empty())
    {
        block.add_line();
    }
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

fn lower_closed_sum(lang: &Java, type_: &ValidatedType<'_>) -> Result<CodeBlock, SigilStitchError> {
    let variants = type_
        .variants()
        .expect("validated closed sums retain their complete case set");
    let mut block = CodeBlock::builder();
    common::emit_doc(&mut block, lang, type_);
    common::emit_structured_annotations(&mut block, type_, "@", "")?;
    common::emit_raw_annotations(&mut block, type_);
    block.add(
        &format!(
            "{}sealed interface {} {{",
            lang.render_visibility(type_.modifiers().visibility, DeclarationContext::TopLevel),
            type_.name()
        ),
        (),
    );
    block.add_line();
    block.add("%>", ());
    for (index, variant) in variants.variants().iter().enumerate() {
        if index > 0 {
            block.add_line();
        }
        crate::lang::variant_lowering::emit_doc(&mut block, lang, variant);
        crate::lang::variant_lowering::emit_structured_annotations(&mut block, variant, "@", "")?;
        crate::lang::variant_lowering::emit_raw_annotations(&mut block, variant);
        if variant.positional_payload().is_empty() && variant.record_payload().is_empty() {
            block.add(
                &format!(
                    "enum {} implements {} {{ INSTANCE }}",
                    variant.name(),
                    type_.name()
                ),
                (),
            );
        } else {
            block.add(&format!("record {}(", variant.name()), ());
            if !variant.positional_payload().is_empty() {
                for (payload_index, payload) in variant.positional_payload().iter().enumerate() {
                    if payload_index > 0 {
                        block.add(", ", ());
                    }
                    block.add(&format!("%T value{payload_index}"), payload.clone());
                }
            } else {
                block.add_code(FieldSpec::lower_sequence(
                    FieldSequenceIntent::closed_sum_record_payload(
                        variant.record_payload(),
                        type_.name(),
                        variant.name(),
                    ),
                    lang,
                )?);
            }
            block.add(&format!(") implements {} {{}}", type_.name()), ());
        }
        block.add_line();
    }
    block.add("%<}", ());
    block.add_line();
    block.build()
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
        let bounds = parameter_bounds(parameter, type_.where_constraints());
        if !bounds.is_empty() {
            format.push_str(" extends ");
            for (bound_index, bound) in bounds.into_iter().enumerate() {
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

fn parameter_bounds<'a>(
    parameter: &'a TypeParamSpec,
    constraints: &'a [WhereConstraint],
) -> Vec<&'a TypeName> {
    let explicit_bounds = constraints
        .iter()
        .filter(|constraint| constraint.parameter_subject_name() == Some(parameter.name()))
        .flat_map(WhereConstraint::bounds);
    crate::lang::java::deduplicated_bounds(parameter.bounds().iter().chain(explicit_bounds))
}
