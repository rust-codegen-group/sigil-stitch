//! Rust-owned enum-variant grammar.

#![deny(deprecated)]

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::lang::rust::Rust;
use crate::spec::enum_variant_spec::{ValidatedVariants, VariantIntent};
use crate::spec::field_spec::{FieldSequenceIntent, FieldSpec};

use super::{emit_doc, emit_raw_annotations, emit_structured_annotations};

pub(crate) fn validate(lang: &Rust, variants: VariantIntent<'_>) -> Result<(), SigilStitchError> {
    let mut errors = Vec::new();
    collect_validation_errors(lang, variants, &mut errors);
    if let Some(error) = errors.into_iter().next() {
        return Err(error);
    }
    Ok(())
}

pub(crate) fn collect_validation_errors(
    lang: &Rust,
    variants: VariantIntent<'_>,
    errors: &mut Vec<SigilStitchError>,
) {
    if !variants.is_closed_sum() {
        return;
    }
    for variant in variants.variants() {
        if !is_identifier(variant.name())
            || matches!(variant.name(), "self" | "Self" | "super" | "crate")
            || crate::lang::RendererLang::reserved_words(lang).contains(&variant.name())
        {
            errors.push(SigilStitchError::InvalidTypeDeclaration {
                type_name: variants.owner_name().to_string(),
                reason: format!(
                    "Rust closed-sum case {:?} is not a valid non-keyword identifier",
                    variant.name()
                ),
            });
        }
    }
}

fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|character| character == '_' || unicode_ident::is_xid_start(character))
        && chars.all(unicode_ident::is_xid_continue)
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
                if variants.is_closed_sum() {
                    FieldSequenceIntent::closed_sum_record_payload(
                        variant.record_payload(),
                        variants.owner_name(),
                        variant.name(),
                    )
                } else {
                    FieldSequenceIntent::variant_record_payload(
                        variant.record_payload(),
                        variants.owner_name(),
                        variants.owner_kind(),
                        variant.name(),
                    )
                },
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
