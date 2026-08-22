//! Ruby-owned enum-like constant grammar.

#![deny(deprecated)]

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::lang::ruby::Ruby;
use crate::spec::enum_variant_spec::{ValidatedVariants, VariantIntent};

use super::{emit_doc, emit_raw_annotations};

pub(crate) fn validate(lang: &Ruby, variants: VariantIntent<'_>) -> Result<(), SigilStitchError> {
    let mut errors = Vec::new();
    collect_validation_errors(lang, variants, &mut errors);
    if let Some(error) = errors.into_iter().next() {
        return Err(error);
    }
    Ok(())
}

pub(crate) fn collect_validation_errors(
    lang: &Ruby,
    variants: VariantIntent<'_>,
    errors: &mut Vec<SigilStitchError>,
) {
    for variant in variants
        .variants()
        .iter()
        .filter(|variant| !variant.annotation_specs().is_empty())
    {
        errors.push(SigilStitchError::InvalidVariantAnnotation {
            language: crate::lang::RendererLang::file_extension(lang).to_string(),
            variant_name: variant.name().to_string(),
            reason: "Ruby has no declaration-metadata syntax; use an opaque annotation block only for target-specific Ruby code"
                .to_string(),
        });
    }
}

pub(crate) fn lower(
    lang: &Ruby,
    variants: ValidatedVariants<'_>,
) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    for variant in variants.variants() {
        emit_doc(&mut block, lang, variant);
        emit_raw_annotations(&mut block, variant);
        if let Some(discriminant) = variant.discriminant().or_else(|| variant.legacy_value()) {
            block.add(&format!("{} = %L", variant.name()), discriminant.clone());
        } else {
            block.add(&format!("{} = :{}", variant.name(), variant.name()), ());
        }
        block.add_line();
    }
    block.build()
}
