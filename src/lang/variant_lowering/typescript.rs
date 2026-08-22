//! TypeScript-owned enum-member grammar.

#![deny(deprecated)]

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::lang::typescript::TypeScript;
use crate::spec::enum_variant_spec::ValidatedVariants;

use super::emit_doc;

pub(crate) fn lower(
    lang: &TypeScript,
    variants: ValidatedVariants<'_>,
) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    for variant in variants.variants() {
        emit_doc(&mut block, lang, variant);
        block.add("%L", variant.name());
        if let Some(discriminant) = variant.discriminant().or_else(|| variant.legacy_value()) {
            block.add(" = %L", discriminant.clone());
        }
        block.add(",", ());
        block.add_line();
    }
    block.build()
}
