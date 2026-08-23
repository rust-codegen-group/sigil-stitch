//! Rust-owned enum-variant grammar.

#![deny(deprecated)]

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::lang::rust::Rust;
use crate::spec::enum_variant_spec::{ValidatedVariants, VariantIntent};
use crate::spec::field_spec::{FieldSequenceIntent, FieldSpec};

use super::{emit_doc, emit_raw_annotations, emit_structured_annotations};

pub(crate) fn validate(_lang: &Rust, variants: VariantIntent<'_>) -> Result<(), SigilStitchError> {
    let mut errors = Vec::new();
    collect_validation_errors(variants, &mut errors);
    if let Some(error) = errors.into_iter().next() {
        return Err(error);
    }
    Ok(())
}

pub(crate) fn collect_validation_errors(
    _variants: VariantIntent<'_>,
    _errors: &mut Vec<SigilStitchError>,
) {
}

pub(crate) fn lower(
    lang: &Rust,
    variants: ValidatedVariants<'_>,
) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    for variant in variants.variants() {
        emit_doc(&mut block, lang, variant);
        emit_structured_annotations(&mut block, variant, "#[", "]")?;
        emit_raw_annotations(&mut block, variant);
        block.add("%L", variant.name());
        if !variant.positional_payload().is_empty() {
            block.add("(", ());
            for (index, payload) in variant.positional_payload().iter().enumerate() {
                if index > 0 {
                    block.add(", ", ());
                }
                block.add("%T", payload.clone());
            }
            block.add(")", ());
        } else if !variant.record_payload().is_empty() {
            block.add(" {", ());
            block.add_line();
            block.add("%>", ());
            block.add_code(FieldSpec::lower_sequence(
                FieldSequenceIntent::variant_record_payload(
                    variant.record_payload(),
                    variants.owner_name(),
                    variants.owner_kind(),
                    variant.name(),
                ),
                lang,
            )?);
            block.add("%<}", ());
        } else if let Some(discriminant) = variant.discriminant().or_else(|| variant.legacy_value())
        {
            block.add(" = %L", discriminant.clone());
        }
        block.add(",", ());
        block.add_line();
    }
    block.build()
}
