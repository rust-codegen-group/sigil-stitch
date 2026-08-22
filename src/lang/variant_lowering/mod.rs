//! Enum-variant sequence lowering.
//!
//! The compatibility module is the sole interpreter of the pre-0.6.8 shared
//! enum configuration. Built-in adapters use language-local modules added
//! alongside this one.

mod compatibility;

pub(crate) mod c;
pub(crate) mod cpp;
pub(crate) mod csharp;
pub(crate) mod dart;
pub(crate) mod haskell;
pub(crate) mod java;
pub(crate) mod javascript;
pub(crate) mod kotlin;
pub(crate) mod ocaml;
pub(crate) mod php;
pub(crate) mod python;
pub(crate) mod ruby;
pub(crate) mod rust;
pub(crate) mod scala;
pub(crate) mod swift;
pub(crate) mod typescript;

pub(crate) use compatibility::{lower as lower_compatibility, lower_legacy_into};

use crate::code_block::CodeBlockBuilder;
use crate::error::SigilStitchError;
use crate::lang::CodeLang;
use crate::spec::enum_variant_spec::{EnumVariantSpec, VariantIntent};

pub(crate) fn variants_precede_fields<L: CodeLang + ?Sized>(lang: &L, inline: bool) -> bool {
    if lang.capabilities().variant_validation_is_permissive() {
        inline && compatibility::variants_before_fields(lang)
    } else {
        true
    }
}

pub(crate) fn emit_doc<L: CodeLang + ?Sized>(
    block: &mut CodeBlockBuilder,
    lang: &L,
    variant: &EnumVariantSpec,
) {
    if variant.doc().is_empty() {
        return;
    }
    let lines: Vec<&str> = variant.doc().iter().map(String::as_str).collect();
    block.add("%L", lang.render_doc_comment(&lines));
    block.add_line();
}

pub(crate) fn emit_structured_annotations(
    block: &mut CodeBlockBuilder,
    variant: &EnumVariantSpec,
    prefix: &str,
    suffix: &str,
) -> Result<(), SigilStitchError> {
    for annotation in variant.annotation_specs() {
        block.add_code(annotation.emit_with_syntax(prefix, suffix)?);
        block.add_line();
    }
    Ok(())
}

pub(crate) fn emit_raw_annotations(block: &mut CodeBlockBuilder, variant: &EnumVariantSpec) {
    for annotation in variant.annotations() {
        block.add_code(annotation.clone());
        block.add_line();
    }
}

pub(crate) fn reject_legacy_values(
    language: &str,
    variants: &VariantIntent<'_>,
) -> Result<(), SigilStitchError> {
    if let Some(variant) = variants
        .variants()
        .iter()
        .find(|variant| variant.legacy_value().is_some())
    {
        return Err(SigilStitchError::UnsupportedLegacyVariantValue {
            language: language.to_string(),
            variant_name: variant.name().to_string(),
        });
    }
    Ok(())
}

pub(crate) fn collect_legacy_value_errors(
    language: &str,
    variants: &VariantIntent<'_>,
    errors: &mut Vec<SigilStitchError>,
) {
    for variant in variants
        .variants()
        .iter()
        .filter(|variant| variant.legacy_value().is_some())
    {
        errors.push(SigilStitchError::UnsupportedLegacyVariantValue {
            language: language.to_string(),
            variant_name: variant.name().to_string(),
        });
    }
}
