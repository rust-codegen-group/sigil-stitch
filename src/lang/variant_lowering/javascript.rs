//! JavaScript-owned enum-like class-member grammar.

#![deny(deprecated)]

use crate::code_block::{CodeBlock, StringLitArg};
use crate::error::SigilStitchError;
use crate::lang::javascript::JavaScript;
use crate::spec::enum_variant_spec::ValidatedVariants;

use super::emit_doc;

pub(crate) fn lower(
    lang: &JavaScript,
    variants: ValidatedVariants<'_>,
) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    for variant in variants.variants() {
        emit_doc(&mut block, lang, variant);
        if let Some(discriminant) = variant.discriminant().or_else(|| variant.legacy_value()) {
            block.add(
                &format!("static {} = %L", variant.name()),
                discriminant.clone(),
            );
        } else {
            block.add(
                &format!("static {} = %S", variant.name()),
                StringLitArg(variant.name().to_string()),
            );
        }
        block.add_line();
    }
    block.build()
}
