//! PHP-owned enum-case grammar.

#![deny(deprecated)]

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::lang::php::Php;
use crate::spec::enum_variant_spec::{ValidatedVariants, VariantIntent};

use super::{
    collect_legacy_value_errors, emit_doc, emit_raw_annotations, emit_structured_annotations,
    reject_legacy_values,
};

pub(crate) fn validate(lang: &Php, variants: VariantIntent<'_>) -> Result<(), SigilStitchError> {
    reject_legacy_values(crate::lang::RendererLang::file_extension(lang), &variants)
}

pub(crate) fn collect_validation_errors(
    lang: &Php,
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
    lang: &Php,
    variants: ValidatedVariants<'_>,
) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    for variant in variants.variants() {
        emit_doc(&mut block, lang, variant);
        emit_structured_annotations(&mut block, variant, "#[", "]")?;
        emit_raw_annotations(&mut block, variant);
        block.add(&format!("case {};", variant.name()), ());
        block.add_line();
    }
    block.build()
}
