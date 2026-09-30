//! Scala-owned ClosedSum declaration grammar.

#![deny(deprecated)]

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::lang::scala::Scala;
use crate::lang::{CodeLang, RendererLang, ValidatedClosedSum};
use crate::spec::closed_sum_spec::ClosedSumIntent;
use crate::spec::modifiers::{DeclarationContext, Visibility};

use super::{emit_annotations, emit_case_annotations, emit_case_doc, emit_doc};

pub(crate) fn validate(lang: &Scala, sum: ClosedSumIntent<'_>) -> Result<(), SigilStitchError> {
    if !matches!(
        sum.visibility(),
        Visibility::Inherited | Visibility::Public | Visibility::Private
    ) {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: sum.name().to_string(),
            reason: format!(
                "Scala does not support {:?} visibility for a top-level closed sum",
                sum.visibility()
            ),
        });
    }
    if !crate::lang::type_lowering::scala::is_identifier(sum.name())
        || lang.reserved_words().contains(&sum.name())
    {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: sum.name().to_string(),
            reason: "Scala closed-sum names require a valid non-keyword identifier".to_string(),
        });
    }
    if sum.cases().is_empty() {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: sum.name().to_string(),
            reason: "Scala closed sums require at least one case".to_string(),
        });
    }
    if !sum.type_params().is_empty() {
        return Err(SigilStitchError::InvalidTypeDeclaration { type_name: sum.name().to_string(), reason: "Scala closed sums with type parameters are unsupported until every case can preserve the root type arguments".to_string() });
    }
    if !sum.where_constraints().is_empty() {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: sum.name().to_string(),
            reason:
                "Scala closed sums do not support declaration constraints without generic cases"
                    .to_string(),
        });
    }
    for case in sum.cases() {
        if !crate::lang::type_lowering::scala::is_identifier(case.name())
            || lang.reserved_words().contains(&case.name())
        {
            return Err(SigilStitchError::InvalidClosedSumCase {
                language: lang.file_extension().to_string(),
                type_name: sum.name().to_string(),
                case_name: case.name().to_string(),
                reason: "case name is not a valid non-keyword identifier".to_string(),
            });
        }
    }
    Ok(())
}

pub(crate) fn lower(
    lang: &Scala,
    sum: ValidatedClosedSum<'_>,
) -> Result<Vec<CodeBlock>, SigilStitchError> {
    let mut block = CodeBlock::builder();
    emit_doc(&mut block, lang, sum.intent());
    emit_annotations(&mut block, sum.intent(), "@", "")?;
    block.add(
        &format!(
            "{}enum {} {{",
            lang.render_visibility(sum.visibility(), DeclarationContext::TopLevel),
            sum.name()
        ),
        (),
    );
    block.add_line();
    block.add("%>", ());
    for case in sum.cases() {
        emit_case_doc(&mut block, lang, case.intent());
        emit_case_annotations(&mut block, case.intent(), "@", "")?;
        block.add(&format!("case {}", case.name()), ());
        if !case.positional_payload().is_empty() {
            block.add("(", ());
            for (index, payload) in case.positional_payload().iter().enumerate() {
                if index > 0 {
                    block.add(", ", ());
                }
                block.add(&format!("value{index}: %T"), payload.clone());
            }
            block.add(")", ());
        } else if let Some(fields) = case.record_payload() {
            block.add("(", ());
            block.add_code(lang.lower_fields(fields.clone())?);
            block.add(")", ());
        }
        block.add_line();
    }
    block.add("%<}", ());
    block.add_line();
    Ok(vec![block.build()?])
}
