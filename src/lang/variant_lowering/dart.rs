//! Dart-owned enum-value grammar.

#![deny(deprecated)]

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::lang::dart::Dart;
use crate::spec::enum_variant_spec::{ValidatedVariants, VariantIntent};

use super::{
    collect_legacy_value_errors, emit_doc, emit_raw_annotations, emit_structured_annotations,
    reject_legacy_values,
};

pub(crate) fn validate(lang: &Dart, variants: VariantIntent<'_>) -> Result<(), SigilStitchError> {
    reject_legacy_values(crate::lang::RendererLang::file_extension(lang), &variants)
}

pub(crate) fn collect_validation_errors(
    lang: &Dart,
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
    lang: &Dart,
    variants: ValidatedVariants<'_>,
) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    let count = variants.variants().len();
    for (index, variant) in variants.variants().iter().enumerate() {
        emit_doc(&mut block, lang, variant);
        emit_structured_annotations(&mut block, variant, "@", "")?;
        emit_raw_annotations(&mut block, variant);
        block.add("%L", variant.name());
        if index + 1 != count {
            block.add(",", ());
        } else if variants.has_non_variant_members() {
            block.add(";", ());
        }
        block.add_line();
    }
    block.build()
}
