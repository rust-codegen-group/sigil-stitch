//! Scala-owned enum-case grammar.

#![deny(deprecated)]

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::lang::scala::Scala;
use crate::spec::enum_variant_spec::{ValidatedVariants, VariantIntent};
use crate::spec::field_spec::{FieldSequenceIntent, FieldSpec};

use super::{
    collect_legacy_value_errors, emit_doc, emit_raw_annotations, emit_structured_annotations,
    reject_legacy_values,
};

pub(crate) fn validate(lang: &Scala, variants: VariantIntent<'_>) -> Result<(), SigilStitchError> {
    reject_legacy_values(crate::lang::RendererLang::file_extension(lang), &variants)
}

pub(crate) fn collect_validation_errors(
    lang: &Scala,
    variants: VariantIntent<'_>,
    errors: &mut Vec<SigilStitchError>,
) {
    collect_legacy_value_errors(
        crate::lang::RendererLang::file_extension(lang),
        &variants,
        errors,
    );
    if variants.is_closed_sum() {
        for variant in variants.variants() {
            if !crate::lang::type_lowering::scala::is_identifier(variant.name())
                || crate::lang::RendererLang::reserved_words(lang).contains(&variant.name())
            {
                errors.push(SigilStitchError::InvalidTypeDeclaration {
                    type_name: variants.owner_name().to_string(),
                    reason: format!(
                        "Scala closed-sum case {:?} is not a valid non-keyword identifier",
                        variant.name()
                    ),
                });
            }
        }
    }
}

pub(crate) fn lower(
    lang: &Scala,
    variants: ValidatedVariants<'_>,
) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    for variant in variants.variants() {
        emit_doc(&mut block, lang, variant);
        emit_structured_annotations(&mut block, variant, "@", "")?;
        emit_raw_annotations(&mut block, variant);
        block.add(&format!("case {}", variant.name()), ());
        if !variant.positional_payload().is_empty() {
            block.add("(", ());
            for (index, payload) in variant.positional_payload().iter().enumerate() {
                if index > 0 {
                    block.add(", ", ());
                }
                block.add(&format!("value{index}: %T"), payload.clone());
            }
            block.add(")", ());
        } else if !variant.record_payload().is_empty() {
            block.add("(", ());
            block.add_code(FieldSpec::lower_sequence(
                FieldSequenceIntent::closed_sum_record_payload(
                    variant.record_payload(),
                    variants.owner_name(),
                    variant.name(),
                ),
                lang,
            )?);
            block.add(")", ());
        }
        block.add_line();
    }
    block.build()
}
