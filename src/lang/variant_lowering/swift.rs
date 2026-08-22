//! Swift-owned enum-case grammar.

#![deny(deprecated)]

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::lang::swift::Swift;
use crate::spec::enum_variant_spec::{ValidatedVariants, VariantIntent};

use super::{
    collect_legacy_value_errors, emit_doc, emit_raw_annotations, emit_structured_annotations,
    reject_legacy_values,
};

pub(crate) fn validate(lang: &Swift, variants: VariantIntent<'_>) -> Result<(), SigilStitchError> {
    reject_legacy_values(crate::lang::RendererLang::file_extension(lang), &variants)
}

pub(crate) fn collect_validation_errors(
    lang: &Swift,
    variants: VariantIntent<'_>,
    errors: &mut Vec<SigilStitchError>,
) {
    collect_legacy_value_errors(
        crate::lang::RendererLang::file_extension(lang),
        &variants,
        errors,
    );
}

pub(crate) fn lower(
    lang: &Swift,
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
                block.add("%T", payload.clone());
            }
            block.add(")", ());
        }
        block.add_line();
    }
    block.build()
}
