//! Exact machine floating values, distinct from source spelling and singleton types.

use crate::code_block::{
    CodeBlock, validate_balanced_indent_markers, validate_no_unresolved_indent_markers,
};
use crate::code_node::CodeNode;
use crate::error::SigilStitchError;
use crate::lang::RendererLang;

/// A floating value identified by its format and original bit pattern.
///
/// Payloads are integer bits, not host floating values. Equality and hashing
/// distinguish the formats, signed zeros and different NaN encodings. The
/// selected language owns construction of an exactly matching value; unsupported
/// targets return an error instead of rounding or emitting raw bytes.
/// Pass the value to `%L` or macro `$L`; source preparation chooses the file or
/// renderer's language for construction, before collecting imports.
///
/// This is value intent, not a floating singleton type or a quantization API.
/// Derived Serde support permits same-version round-trips, not a versioned wire
/// protocol. Additional scalar formats may be added without treating shared-scale
/// blocks as scalar values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum FloatValue {
    /// One IEEE 754 binary32 value, including its exact NaN and signed-zero bits.
    Binary32(u32),
    /// One IEEE 754 binary64 value, including its exact NaN and signed-zero bits.
    Binary64(u64),
}

impl FloatValue {
    /// Capture the exact bits of a host binary32 value without numeric conversion.
    pub fn from_f32(value: f32) -> Self {
        Self::Binary32(value.to_bits())
    }

    /// Capture the exact bits of a host binary64 value without numeric conversion.
    pub fn from_f64(value: f64) -> Self {
        Self::Binary64(value.to_bits())
    }
}

pub(crate) fn lower(
    value: &FloatValue,
    language: &dyn RendererLang,
) -> Result<CodeBlock, SigilStitchError> {
    let block = language.lower_float_value(value)?;
    let invalid = |reason: String| SigilStitchError::InvalidFloatValueLowering {
        language: language.file_extension().to_string(),
        reason,
    };
    if !contains_source_content(&block.nodes) {
        return Err(invalid(
            "output must contain source content, not only layout".to_string(),
        ));
    }
    validate_balanced_indent_markers(&block.nodes).map_err(|error| invalid(error.to_string()))?;
    validate_no_unresolved_indent_markers(&block.nodes)
        .map_err(|error| invalid(error.to_string()))?;
    if contains_float_value(&block.nodes) {
        return Err(invalid(
            "output must not contain unresolved floating values".to_string(),
        ));
    }
    Ok(block)
}

fn contains_source_content(nodes: &[CodeNode]) -> bool {
    nodes.iter().any(|node| match node {
        CodeNode::Literal(text) | CodeNode::InlineLiteral(text) | CodeNode::NameRef(text) => {
            !text.trim().is_empty()
        }
        CodeNode::TypeRef(type_name) => !type_name.is_empty(),
        CodeNode::Nested(block) => contains_source_content(&block.nodes),
        CodeNode::Sequence(children) => contains_source_content(children),
        CodeNode::SoftBreak
        | CodeNode::Indent
        | CodeNode::Dedent
        | CodeNode::StatementBegin
        | CodeNode::StatementEnd
        | CodeNode::Newline => false,
        _ => true,
    })
}

fn contains_float_value(nodes: &[CodeNode]) -> bool {
    nodes.iter().any(|node| match node {
        CodeNode::FloatValue(_) => true,
        CodeNode::Nested(block) => contains_float_value(&block.nodes),
        CodeNode::Sequence(children) => contains_float_value(children),
        _ => false,
    })
}
