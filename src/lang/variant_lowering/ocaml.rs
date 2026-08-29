//! OCaml-owned variant-constructor grammar.

#![deny(deprecated)]

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::lang::ocaml::OCaml;
use crate::spec::enum_variant_spec::{ValidatedVariants, VariantIntent};
use crate::spec::field_spec::{FieldSequenceIntent, FieldSpec};

use super::{collect_legacy_value_errors, emit_doc};

pub(crate) fn validate(lang: &OCaml, variants: VariantIntent<'_>) -> Result<(), SigilStitchError> {
    let mut errors = Vec::new();
    collect_validation_errors(lang, variants, &mut errors);
    if let Some(error) = errors.into_iter().next() {
        return Err(error);
    }
    Ok(())
}

pub(crate) fn collect_validation_errors(
    lang: &OCaml,
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
            let mut characters = variant.name().chars();
            let valid = characters.next().is_some_and(char::is_uppercase)
                && characters.all(|character| {
                    character == '_' || character == '\'' || character.is_alphanumeric()
                });
            if !valid || crate::lang::RendererLang::reserved_words(lang).contains(&variant.name()) {
                errors.push(SigilStitchError::InvalidTypeDeclaration {
                    type_name: variants.owner_name().to_string(),
                    reason: format!(
                        "OCaml closed-sum case {:?} is not a valid constructor name",
                        variant.name()
                    ),
                });
            }
            if !variant.annotations().is_empty() || !variant.annotation_specs().is_empty() {
                errors.push(SigilStitchError::InvalidTypeDeclaration {
                    type_name: variants.owner_name().to_string(),
                    reason: format!(
                        "OCaml closed-sum case {:?} does not support annotations",
                        variant.name()
                    ),
                });
            }
        }
    }
}

pub(crate) fn lower(
    lang: &OCaml,
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
            block.add(" of ", ());
            for (payload_index, payload) in variant.positional_payload().iter().enumerate() {
                if payload_index > 0 {
                    block.add(" * ", ());
                }
                if crate::type_name_render::is_compound_type(payload) {
                    block.add("(%T)", payload.clone());
                } else {
                    block.add("%T", payload.clone());
                }
            }
        } else if !variant.record_payload().is_empty() {
            block.add(" of { ", ());
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
