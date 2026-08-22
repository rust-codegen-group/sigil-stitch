//! Java-owned enum-constant grammar.

#![deny(deprecated)]

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::lang::java::Java;
use crate::spec::enum_variant_spec::{ValidatedVariants, VariantIntent};

use super::{emit_doc, emit_raw_annotations, emit_structured_annotations};

pub(crate) fn validate(lang: &Java, variants: VariantIntent<'_>) -> Result<(), SigilStitchError> {
    let mut errors = Vec::new();
    collect_validation_errors(lang, variants, &mut errors);
    if let Some(error) = errors.into_iter().next() {
        return Err(error);
    }
    Ok(())
}

pub(crate) fn collect_validation_errors(
    lang: &Java,
    variants: VariantIntent<'_>,
    errors: &mut Vec<SigilStitchError>,
) {
    if variants.has_opaque_members() {
        return;
    }

    for variant in variants.variants() {
        let has_arguments =
            !variant.constructor_arguments().is_empty() || variant.legacy_value().is_some();
        let argument_count = if variant.constructor_arguments().is_empty() {
            usize::from(variant.legacy_value().is_some())
        } else {
            variant.constructor_arguments().len()
        };
        if !variants.has_declared_constructor() {
            if has_arguments {
                errors.push(SigilStitchError::MissingVariantConstructor {
                    language: crate::lang::RendererLang::file_extension(lang).to_string(),
                    type_name: variants.owner_name().to_string(),
                    variant_name: variant.name().to_string(),
                });
            }
        } else if !variants.has_compatible_constructor(argument_count) {
            errors.push(SigilStitchError::IncompatibleVariantConstructorArguments {
                language: crate::lang::RendererLang::file_extension(lang).to_string(),
                type_name: variants.owner_name().to_string(),
                variant_name: variant.name().to_string(),
                argument_count,
            });
        }
    }
}

pub(crate) fn lower(
    lang: &Java,
    variants: ValidatedVariants<'_>,
) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    let count = variants.variants().len();
    for (index, variant) in variants.variants().iter().enumerate() {
        emit_doc(&mut block, lang, variant);
        emit_structured_annotations(&mut block, variant, "@", "")?;
        emit_raw_annotations(&mut block, variant);
        block.add("%L", variant.name());
        if !variant.constructor_arguments().is_empty() {
            block.add("(", ());
            for (argument_index, argument) in variant.constructor_arguments().iter().enumerate() {
                if argument_index > 0 {
                    block.add(", ", ());
                }
                block.add("%L", argument.clone());
            }
            block.add(")", ());
        } else if let Some(value) = variant.legacy_value() {
            block.add("(%L)", value.clone());
        }
        if index + 1 != count {
            block.add(",", ());
        } else if variants.has_following_members() {
            block.add(";", ());
        }
        block.add_line();
    }
    block.build()
}
