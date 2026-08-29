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
    if variants.is_closed_sum() {
        for variant in variants.variants() {
            let generated_name = format!("{}{}", variants.owner_name(), variant.name());
            if !crate::lang::type_lowering::dart::is_identifier(variant.name())
                || crate::lang::RendererLang::reserved_words(lang).contains(&variant.name())
                || !crate::lang::type_lowering::dart::is_identifier(&generated_name)
                || crate::lang::RendererLang::reserved_words(lang)
                    .contains(&generated_name.as_str())
                || generated_name == variants.owner_name()
            {
                errors.push(SigilStitchError::InvalidTypeDeclaration {
                    type_name: variants.owner_name().to_string(),
                    reason: format!(
                        "Dart closed-sum case {:?} does not produce a valid distinct root-qualified type name",
                        variant.name()
                    ),
                });
            }
        }
    }
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
