//! Haskell-owned ClosedSum declaration grammar.

#![deny(deprecated)]

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::lang::haskell::Haskell;
use crate::lang::{CodeLang, RendererLang, ValidatedClosedSum};
use crate::spec::closed_sum_spec::ClosedSumIntent;
use crate::spec::modifiers::Visibility;

use super::{emit_case_doc, emit_doc};

pub(crate) fn validate(lang: &Haskell, sum: ClosedSumIntent<'_>) -> Result<(), SigilStitchError> {
    if sum.visibility() != Visibility::Inherited {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: sum.name().to_string(),
            reason: "Haskell type declarations do not support explicit visibility".to_string(),
        });
    }
    if !crate::lang::type_lowering::haskell::starts_uppercase(sum.name())
        || lang.reserved_words().contains(&sum.name())
    {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: sum.name().to_string(),
            reason: "Haskell closed-sum names require an uppercase non-keyword identifier"
                .to_string(),
        });
    }
    if sum.cases().is_empty() {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: sum.name().to_string(),
            reason: "Haskell closed sums require at least one constructor".to_string(),
        });
    }
    for parameter in sum.type_params() {
        if parameter.is_lifetime()
            || !crate::lang::type_lowering::haskell::starts_lowercase_identifier(parameter.name())
            || lang.reserved_words().contains(&parameter.name())
        {
            return Err(SigilStitchError::InvalidTypeParameter {
                type_name: sum.name().to_string(),
                parameter_name: parameter.name().to_string(),
                reason: "Haskell type variables require a lowercase non-keyword identifier"
                    .to_string(),
            });
        }
    }
    for constraint in sum.where_constraints() {
        let declared = constraint.parameter_subject_name().is_some_and(|subject| {
            sum.type_params()
                .iter()
                .any(|parameter| parameter.name() == subject)
        });
        if !declared {
            return Err(SigilStitchError::InvalidTypeParameter {
                type_name: sum.name().to_string(),
                parameter_name: format!("{:?}", constraint.subject()),
                reason: "Haskell declaration constraints must target a declared type variable"
                    .to_string(),
            });
        }
    }
    let mut record_selectors = std::collections::HashMap::new();
    for case in sum.cases() {
        if !crate::lang::type_lowering::haskell::starts_uppercase(case.name())
            || lang.reserved_words().contains(&case.name())
        {
            return Err(SigilStitchError::InvalidClosedSumCase {
                language: lang.file_extension().to_string(),
                type_name: sum.name().to_string(),
                case_name: case.name().to_string(),
                reason: "case name is not a valid data-constructor name".to_string(),
            });
        }
        if !case.annotations().is_empty() || !case.annotation_specs().is_empty() {
            return Err(SigilStitchError::InvalidClosedSumCase {
                language: lang.file_extension().to_string(),
                type_name: sum.name().to_string(),
                case_name: case.name().to_string(),
                reason: "Haskell cases do not support annotations".to_string(),
            });
        }
        for field in case.record_payload() {
            let selector = lang.escape_field_name(field.name());
            if let Some((previous_type, previous_case)) = record_selectors.get(&selector).copied() {
                if previous_case != case.name() && previous_type != field.field_type() {
                    return Err(SigilStitchError::InvalidClosedSumCase {
                        language: lang.file_extension().to_string(),
                        type_name: sum.name().to_string(),
                        case_name: case.name().to_string(),
                        reason: format!(
                            "record selector {:?} was already declared with a different type by case {previous_case:?}",
                            field.name()
                        ),
                    });
                }
            } else {
                record_selectors.insert(selector, (field.field_type(), case.name()));
            }
        }
    }
    Ok(())
}

pub(crate) fn lower(
    lang: &Haskell,
    sum: ValidatedClosedSum<'_>,
) -> Result<Vec<CodeBlock>, SigilStitchError> {
    let mut block = CodeBlock::builder();
    emit_doc(&mut block, lang, sum.intent());
    block.add("data ", ());
    emit_context(&mut block, &sum);
    block.add("%L", sum.name());
    for parameter in sum.type_params() {
        block.add(" %L", parameter.name());
    }
    block.add(" =", ());
    block.add_line();
    block.add("%>", ());
    for (index, case) in sum.cases().enumerate() {
        if index > 0 {
            block.add("| ", ());
        }
        emit_case_doc(&mut block, lang, case.intent());
        block.add("%L", case.name());
        if !case.positional_payload().is_empty() {
            for payload in case.positional_payload() {
                if crate::type_name_render::is_compound_type(payload) {
                    block.add(" (%T)", payload.clone());
                } else {
                    block.add(" %T", payload.clone());
                }
            }
        } else if let Some(fields) = case.record_payload() {
            block.add(" { ", ());
            block.add_code(lang.lower_fields(fields.clone())?);
            block.add(" }", ());
        }
        block.add_line();
    }
    block.add("%<", ());
    block.build().map(|block| vec![block])
}

fn emit_context(block: &mut crate::code_block::CodeBlockBuilder, sum: &ValidatedClosedSum<'_>) {
    let count = sum
        .type_params()
        .iter()
        .map(|parameter| parameter.bounds().len() + parameter.context_bounds().len())
        .sum::<usize>()
        + sum
            .where_constraints()
            .iter()
            .map(|constraint| constraint.bounds().len())
            .sum::<usize>();
    if count == 0 {
        return;
    }
    if count > 1 {
        block.add("(", ());
    }
    let mut index = 0;
    for parameter in sum.type_params() {
        for bound in parameter.bounds().iter().chain(parameter.context_bounds()) {
            if index > 0 {
                block.add(", ", ());
            }
            block.add("%T %L", (bound.clone(), parameter.name()));
            index += 1;
        }
    }
    for constraint in sum.where_constraints() {
        for bound in constraint.bounds() {
            if index > 0 {
                block.add(", ", ());
            }
            block.add("%T %T", (bound.clone(), constraint.subject().clone()));
            index += 1;
        }
    }
    if count > 1 {
        block.add(")", ());
    }
    block.add(" => ", ());
}
