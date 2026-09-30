//! Dart-owned ClosedSum declaration grammar.

#![deny(deprecated)]

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::lang::dart::Dart;
use crate::lang::{CodeLang, RendererLang, ValidatedClosedSum};
use crate::spec::closed_sum_spec::ClosedSumIntent;
use crate::spec::modifiers::Visibility;

use super::{emit_annotations, emit_case_annotations, emit_case_doc, emit_doc};

pub(crate) fn validate(lang: &Dart, sum: ClosedSumIntent<'_>) -> Result<(), SigilStitchError> {
    if sum.visibility() != Visibility::Inherited {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: sum.name().to_string(),
            reason: "Dart type declarations do not support explicit visibility".to_string(),
        });
    }
    if !crate::lang::type_lowering::dart::is_identifier(sum.name())
        || lang.reserved_words().contains(&sum.name())
    {
        return Err(SigilStitchError::InvalidTypeDeclaration {
            type_name: sum.name().to_string(),
            reason: "Dart closed-sum names require a valid non-keyword identifier".to_string(),
        });
    }
    for case in sum.cases() {
        let generated = format!("{}{}", sum.name(), case.name());
        if !crate::lang::type_lowering::dart::is_identifier(case.name())
            || lang.reserved_words().contains(&case.name())
            || !crate::lang::type_lowering::dart::is_identifier(&generated)
            || lang.reserved_words().contains(&generated.as_str())
        {
            return Err(SigilStitchError::InvalidClosedSumCase {
                language: lang.file_extension().to_string(),
                type_name: sum.name().to_string(),
                case_name: case.name().to_string(),
                reason: "case does not produce a valid root-qualified type name".to_string(),
            });
        }
    }
    Ok(())
}

pub(crate) fn lower(
    lang: &Dart,
    sum: ValidatedClosedSum<'_>,
) -> Result<Vec<CodeBlock>, SigilStitchError> {
    let mut blocks = Vec::with_capacity(sum.cases().count() + 1);
    let mut root = CodeBlock::builder();
    emit_doc(&mut root, lang, sum.intent());
    emit_annotations(&mut root, sum.intent(), "@", "")?;
    root.add(&format!("sealed class {} {{", sum.name()), ());
    root.add_line();
    root.add("%>const ", ());
    root.add(&format!("{}._();", sum.name()), ());
    root.add_line();
    root.add("%<}", ());
    root.add_line();
    blocks.push(root.build()?);

    for case in sum.cases() {
        let generated = format!("{}{}", sum.name(), case.name());
        let mut block = CodeBlock::builder();
        emit_case_doc(&mut block, lang, case.intent());
        emit_case_annotations(&mut block, case.intent(), "@", "")?;
        block.add(
            &format!("final class {generated} extends {} {{", sum.name()),
            (),
        );
        block.add_line();
        block.add("%>", ());
        if case.positional_payload().is_empty() && case.record_payload().is_none() {
            block.add(&format!("const {generated}._() : super._();"), ());
            block.add_line();
            block.add(
                &format!("static const {generated} instance = {generated}._();"),
                (),
            );
            block.add_line();
        } else {
            block.add(&format!("const {generated}("), ());
            let count = if !case.positional_payload().is_empty() {
                case.positional_payload().len()
            } else {
                case.record_payload()
                    .map_or(0, |fields| fields.fields().len())
            };
            for index in 0..count {
                if index > 0 {
                    block.add(", ", ());
                }
                let field_name = if !case.positional_payload().is_empty() {
                    format!("value{index}")
                } else {
                    lang.escape_field_name(case.record_payload().unwrap().fields()[index].name())
                };
                block.add("this.%L", field_name);
            }
            block.add(") : super._();", ());
            block.add_line();
            for (index, ty) in case.positional_payload().iter().enumerate() {
                block.add(&format!("final %T value{index};"), ty.clone());
                block.add_line();
            }
            if let Some(fields) = case.record_payload() {
                block.add_code(lang.lower_fields(fields.clone())?);
            }
        }
        block.add("%<}", ());
        block.add_line();
        blocks.push(block.build()?);
    }
    Ok(blocks)
}
