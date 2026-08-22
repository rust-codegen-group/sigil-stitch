//! Python-owned Enum member grammar.

#![deny(deprecated)]

use crate::code_block::{CodeBlock, StringLitArg};
use crate::error::SigilStitchError;
use crate::lang::python::Python;
use crate::spec::enum_variant_spec::ValidatedVariants;

pub(crate) fn lower(
    _lang: &Python,
    variants: ValidatedVariants<'_>,
) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    for variant in variants.variants() {
        for line in variant.doc() {
            if line.is_empty() {
                block.add("#", ());
            } else {
                block.add("# %L", line.as_str());
            }
            block.add_line();
        }
        if let Some(discriminant) = variant.discriminant().or_else(|| variant.legacy_value()) {
            block.add(&format!("{} = %L", variant.name()), discriminant.clone());
        } else {
            block.add(
                &format!("{} = %S", variant.name()),
                StringLitArg(variant.name().to_string()),
            );
        }
        block.add_line();
    }
    block.build()
}
