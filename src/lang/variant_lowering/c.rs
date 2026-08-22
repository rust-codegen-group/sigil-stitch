//! C-owned enum enumerator grammar.

#![deny(deprecated)]

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::lang::c::C;
use crate::spec::enum_variant_spec::ValidatedVariants;

use super::emit_doc;

pub(crate) fn lower(
    lang: &C,
    variants: ValidatedVariants<'_>,
) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    let count = variants.variants().len();
    for (index, variant) in variants.variants().iter().enumerate() {
        emit_doc(&mut block, lang, variant);
        block.add("%L", variant.name());
        for annotation in variant.annotations() {
            block.add(" ", ());
            block.add_code(annotation.clone());
        }
        for annotation in variant.annotation_specs() {
            block.add(" ", ());
            block.add_code(annotation.emit_with_syntax("__attribute__((", "))")?);
        }
        if let Some(discriminant) = variant.discriminant().or_else(|| variant.legacy_value()) {
            block.add(" = %L", discriminant.clone());
        }
        if index + 1 != count {
            block.add(",", ());
        }
        block.add_line();
    }
    block.build()
}
