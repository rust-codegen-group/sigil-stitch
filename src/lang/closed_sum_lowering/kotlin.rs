//! Kotlin-owned ClosedSum declaration grammar.

#![deny(deprecated)]

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::lang::kotlin::Kotlin;
use crate::lang::{CodeLang, RendererLang, ValidatedClosedSum};
use crate::spec::closed_sum_spec::ClosedSumIntent;
use crate::spec::modifiers::{DeclarationContext, Visibility};

use super::{emit_annotations, emit_case_annotations, emit_case_doc, emit_doc};

pub(crate) fn validate(lang: &Kotlin, sum: ClosedSumIntent<'_>) -> Result<(), SigilStitchError> {
    if !matches!(
        sum.visibility(),
        Visibility::Inherited | Visibility::Public | Visibility::Private
    ) {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: sum.name().to_string(),
            reason: format!(
                "Kotlin does not support {:?} visibility for a top-level closed sum",
                sum.visibility()
            ),
        });
    }
    if !crate::lang::field_lowering::kotlin::is_valid_identifier(sum.name())
        || lang.reserved_words().contains(&sum.name())
    {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: sum.name().to_string(),
            reason: "Kotlin closed-sum names require a valid non-keyword identifier".to_string(),
        });
    }
    for case in sum.cases() {
        if !crate::lang::field_lowering::kotlin::is_valid_identifier(case.name())
            || lang.reserved_words().contains(&case.name())
            || case.name() == sum.name()
        {
            return Err(SigilStitchError::InvalidClosedSumCase {
                language: lang.file_extension().to_string(),
                type_name: sum.name().to_string(),
                case_name: case.name().to_string(),
                reason: "case name is not a valid distinct nested type name".to_string(),
            });
        }
    }
    Ok(())
}

pub(crate) fn lower(
    lang: &Kotlin,
    sum: ValidatedClosedSum<'_>,
) -> Result<Vec<CodeBlock>, SigilStitchError> {
    let mut block = CodeBlock::builder();
    emit_doc(&mut block, lang, sum.intent());
    emit_annotations(&mut block, sum.intent(), "@", "")?;
    block.add(
        &format!(
            "{}sealed class {} private constructor() {{",
            lang.render_visibility(sum.visibility(), DeclarationContext::TopLevel),
            sum.name()
        ),
        (),
    );
    block.add_line();
    block.add("%>", ());
    for (index, case) in sum.cases().enumerate() {
        if index > 0 {
            block.add_line();
        }
        emit_case_doc(&mut block, lang, case.intent());
        emit_case_annotations(&mut block, case.intent(), "@", "")?;
        if case.positional_payload().is_empty() && case.record_payload().is_none() {
            block.add(
                &format!("data object {} : {}()", case.name(), sum.name()),
                (),
            );
        } else {
            block.add(&format!("data class {}(", case.name()), ());
            if !case.positional_payload().is_empty() {
                for (payload_index, payload) in case.positional_payload().iter().enumerate() {
                    if payload_index > 0 {
                        block.add(", ", ());
                    }
                    block.add(&format!("val value{payload_index}: %T"), payload.clone());
                }
            } else if let Some(fields) = case.record_payload() {
                block.add_code(lang.lower_fields(fields.clone())?);
            }
            block.add(&format!(") : {}()", sum.name()), ());
        }
        block.add_line();
    }
    block.add("%<}", ());
    block.add_line();
    Ok(vec![block.build()?])
}
