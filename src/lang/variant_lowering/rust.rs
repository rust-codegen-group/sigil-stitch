//! Rust-owned enum-variant grammar.

#![deny(deprecated)]

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::lang::CodeLang;
use crate::lang::rust::Rust;
use crate::spec::enum_variant_spec::{ValidatedVariants, VariantIntent};
use crate::spec::modifiers::Visibility;
use crate::type_name::TypeName;

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
    variants: VariantIntent<'_>,
    errors: &mut Vec<SigilStitchError>,
) {
    for variant in variants.variants() {
        for field in variant.record_payload() {
            if field.modifiers.visibility != Visibility::Inherited
                || field.modifiers.is_static
                || field.modifiers.is_readonly
                || field.initializer.is_some()
                || field.tag.is_some()
            {
                errors.push(SigilStitchError::InvalidVariantRecordField {
                    language: "rs".to_string(),
                    variant_name: variant.name().to_string(),
                    field_name: field.name().to_string(),
                    reason: "Rust enum payload fields cannot carry visibility, static/readonly modifiers, initializers, or tags"
                        .to_string(),
                });
            }
        }
    }
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
            for field in variant.record_payload() {
                if !field.doc.is_empty() {
                    let lines: Vec<&str> = field.doc.iter().map(String::as_str).collect();
                    block.add("%L", lang.render_doc_comment(&lines));
                    block.add_line();
                }
                for annotation in &field.annotation_specs {
                    block.add_code(annotation.emit_with_syntax("#[", "]")?);
                    block.add_line();
                }
                for annotation in &field.annotations {
                    block.add_code(annotation.clone());
                    block.add_line();
                }
                let field_type = if field.is_optional {
                    TypeName::optional(field.field_type().clone())
                } else {
                    field.field_type().clone()
                };
                block.add(
                    "%L: %T,",
                    (lang.escape_field_name(field.name()), field_type),
                );
                block.add_line();
            }
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
