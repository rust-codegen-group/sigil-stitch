//! Haskell-owned data-constructor grammar.

#![deny(deprecated)]

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::lang::CodeLang;
use crate::lang::haskell::Haskell;
use crate::spec::enum_variant_spec::{ValidatedVariants, VariantIntent};
use crate::spec::field_spec::{FieldSequenceIntent, FieldSpec};

use super::{collect_legacy_value_errors, emit_doc};

pub(crate) fn validate(
    lang: &Haskell,
    variants: VariantIntent<'_>,
) -> Result<(), SigilStitchError> {
    let mut errors = Vec::new();
    collect_validation_errors(lang, variants, &mut errors);
    if let Some(error) = errors.into_iter().next() {
        return Err(error);
    }
    Ok(())
}

pub(crate) fn collect_validation_errors(
    lang: &Haskell,
    variants: VariantIntent<'_>,
    errors: &mut Vec<SigilStitchError>,
) {
    let language = crate::lang::RendererLang::file_extension(lang);
    collect_legacy_value_errors(language, &variants, errors);
    if variants.is_closed_sum() {
        for variant in variants.variants() {
            if !crate::lang::type_lowering::haskell::starts_uppercase(variant.name())
                || crate::lang::RendererLang::reserved_words(lang).contains(&variant.name())
            {
                errors.push(SigilStitchError::InvalidTypeDeclaration {
                    type_name: variants.owner_name().to_string(),
                    reason: format!(
                        "Haskell closed-sum case {:?} is not a valid data-constructor name",
                        variant.name()
                    ),
                });
            }
            if !variant.annotations().is_empty() || !variant.annotation_specs().is_empty() {
                errors.push(SigilStitchError::InvalidTypeDeclaration {
                    type_name: variants.owner_name().to_string(),
                    reason: format!(
                        "Haskell closed-sum case {:?} does not support annotations",
                        variant.name()
                    ),
                });
            }
        }
    }
    let mut record_selectors = std::collections::HashMap::new();
    for variant in variants.variants() {
        for field in variant.record_payload() {
            let escaped_field_name = lang.escape_field_name(field.name());
            if let Some((previous_type, previous_variant)) =
                record_selectors.get(&escaped_field_name).copied()
            {
                if previous_variant != variant.name() && previous_type != field.field_type() {
                    errors.push(SigilStitchError::InvalidVariantRecordField {
                        language: language.to_string(),
                        variant_name: variant.name().to_string(),
                        field_name: field.name().to_string(),
                        reason: format!(
                            "record selector was already declared with a different type by variant {previous_variant:?}"
                        ),
                    });
                }
            } else {
                record_selectors.insert(escaped_field_name, (field.field_type(), variant.name()));
            }
        }
    }
}

pub(crate) fn lower(
    lang: &Haskell,
    variants: ValidatedVariants<'_>,
) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    for (index, variant) in variants.variants().iter().enumerate() {
        emit_doc(&mut block, lang, variant);
        if index > 0 {
            block.add("| ", ());
        }
        block.add("%L", variant.name());
        if !variant.positional_payload().is_empty() {
            for payload in variant.positional_payload() {
                if crate::type_name_render::is_compound_type(payload) {
                    block.add(" (%T)", payload.clone());
                } else {
                    block.add(" %T", payload.clone());
                }
            }
        } else if !variant.record_payload().is_empty() {
            block.add(" { ", ());
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
            block.add(" }", ());
        }
        block.add_line();
    }
    block.build()
}
