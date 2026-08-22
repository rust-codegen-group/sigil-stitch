//! Haskell-owned data-constructor grammar.

#![deny(deprecated)]

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::lang::CodeLang;
use crate::lang::haskell::Haskell;
use crate::spec::enum_variant_spec::{ValidatedVariants, VariantIntent};
use crate::spec::modifiers::Visibility;

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
    let mut record_selectors = std::collections::HashMap::new();
    for variant in variants.variants() {
        let mut escaped_field_names = std::collections::HashMap::new();
        for field in variant.record_payload() {
            let escaped_field_name = lang.escape_field_name(field.name());
            if let Some(previous_name) = escaped_field_names
                .insert(escaped_field_name.clone(), field.name())
                .filter(|previous_name| *previous_name != field.name())
            {
                errors.push(SigilStitchError::InvalidVariantRecordField {
                    language: language.to_string(),
                    variant_name: variant.name().to_string(),
                    field_name: field.name().to_string(),
                    reason: format!(
                        "field name collides with {previous_name:?} after both escape as {escaped_field_name:?}"
                    ),
                });
            }

            if field.modifiers.visibility != Visibility::Inherited
                || field.modifiers.is_static
                || field.modifiers.is_readonly
                || field.initializer.is_some()
                || !field.doc.is_empty()
                || !field.annotations.is_empty()
                || !field.annotation_specs.is_empty()
                || field.tag.is_some()
                || field.is_optional
            {
                errors.push(SigilStitchError::InvalidVariantRecordField {
                    language: "hs".to_string(),
                    variant_name: variant.name().to_string(),
                    field_name: field.name().to_string(),
                    reason: "record constructor fields currently require a plain name and type"
                        .to_string(),
                });
            }

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
            for (field_index, field) in variant.record_payload().iter().enumerate() {
                if field_index > 0 {
                    block.add(", ", ());
                }
                block.add(
                    "%L :: %T",
                    (
                        lang.escape_field_name(field.name()),
                        field.field_type().clone(),
                    ),
                );
            }
            block.add(" }", ());
        }
        block.add_line();
    }
    block.build()
}
