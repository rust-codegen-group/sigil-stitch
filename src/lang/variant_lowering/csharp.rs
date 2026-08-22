//! C#-owned enum-member grammar.

#![deny(deprecated)]

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::lang::csharp::CSharp;
use crate::spec::enum_variant_spec::ValidatedVariants;

use super::{emit_doc, emit_raw_annotations, emit_structured_annotations};

pub(crate) fn lower(
    lang: &CSharp,
    variants: ValidatedVariants<'_>,
) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    let count = variants.variants().len();
    for (index, variant) in variants.variants().iter().enumerate() {
        emit_doc(&mut block, lang, variant);
        emit_structured_annotations(&mut block, variant, "[", "]")?;
        emit_raw_annotations(&mut block, variant);
        block.add("%L", variant.name());
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
