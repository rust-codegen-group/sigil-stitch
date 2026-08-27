//! Structured recovery for pre-0.6.8 declaration hooks that return strings.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::code_block::CodeBlock;
use crate::code_node::CodeNode;
use crate::error::SigilStitchError;
use crate::lang::CodeLang;
use crate::spec::where_spec::{TypeParamSpec, render_type_params_for};
use crate::type_name::TypeName;

const MARKER_PREFIX: &str = "__SIGIL_STITCH_LEGACY_TYPE_";
const MARKER_SUFFIX: &str = "__";

static NEXT_MARKER_SET: AtomicU64 = AtomicU64::new(0);

pub(crate) struct LegacyTypeMarkers {
    hook: &'static str,
    nonce: u64,
    types: Vec<TypeName>,
    tokens: Vec<String>,
}

impl LegacyTypeMarkers {
    pub(crate) fn new(hook: &'static str) -> Self {
        Self {
            hook,
            nonce: NEXT_MARKER_SET.fetch_add(1, Ordering::Relaxed),
            types: Vec::new(),
            tokens: Vec::new(),
        }
    }

    pub(crate) fn mark(&mut self, type_name: &TypeName) -> String {
        let index = self.types.len();
        let token = format!(
            "{MARKER_PREFIX}{:016x}_{index:08x}{MARKER_SUFFIX}",
            self.nonce
        );
        self.types.push(type_name.clone());
        self.tokens.push(token.clone());
        token
    }

    pub(crate) fn mark_type_params(&mut self, params: &[TypeParamSpec]) -> Vec<TypeParamSpec> {
        params
            .iter()
            .map(|param| TypeParamSpec {
                name: param.name.clone(),
                bounds: param
                    .bounds
                    .iter()
                    .map(|bound| TypeName::raw(&self.mark(bound)))
                    .collect(),
                kind: param.kind.clone(),
                is_lifetime: param.is_lifetime,
                context_bounds: param
                    .context_bounds
                    .iter()
                    .map(|bound| TypeName::raw(&self.mark(bound)))
                    .collect(),
            })
            .collect()
    }

    pub(crate) fn render_marked_type_params<L: CodeLang + ?Sized>(
        &mut self,
        params: &[TypeParamSpec],
        lang: &L,
    ) -> Result<String, SigilStitchError> {
        let marked = self.mark_type_params(params);
        let mut arguments = Vec::new();
        let format = render_type_params_for(&marked, lang, &mut arguments);
        let block = CodeBlock::of(&format, arguments)?;
        self.flatten_marker_block(&block)
    }

    pub(crate) fn recover(self, output: &str) -> Result<CodeBlock, SigilStitchError> {
        let mut nodes = Vec::new();
        let mut seen = vec![false; self.types.len()];
        let mut cursor = 0;

        while let Some(relative_start) = output[cursor..].find(MARKER_PREFIX) {
            let start = cursor + relative_start;
            Self::push_literal(&mut nodes, &output[cursor..start]);

            let token_tail = &output[start + MARKER_PREFIX.len()..];
            let Some(relative_end) = token_tail.find(MARKER_SUFFIX) else {
                return Err(self.error("contains a malformed semantic type marker"));
            };
            let end = start + MARKER_PREFIX.len() + relative_end + MARKER_SUFFIX.len();
            let token = &output[start..end];
            let Some(index) = self.tokens.iter().position(|known| known == token) else {
                return Err(self.error("contains an unknown or changed semantic type marker"));
            };

            nodes.push(CodeNode::TypeRef(self.types[index].clone()));
            seen[index] = true;
            cursor = end;
        }

        Self::push_literal(&mut nodes, &output[cursor..]);

        if seen.iter().any(|was_seen| !was_seen) {
            return Err(self.error("discarded or changed a semantic type marker"));
        }

        Ok(CodeBlock { nodes })
    }

    fn flatten_marker_block(&self, block: &CodeBlock) -> Result<String, SigilStitchError> {
        let mut output = String::new();
        for node in &block.nodes {
            match node {
                CodeNode::Literal(text) => output.push_str(text),
                CodeNode::TypeRef(TypeName::Raw(marker))
                    if self.tokens.iter().any(|known| known == marker) =>
                {
                    output.push_str(marker);
                }
                _ => {
                    return Err(self.error(
                        "could not preserve the legacy type-parameter format structurally",
                    ));
                }
            }
        }
        Ok(output)
    }

    fn push_literal(nodes: &mut Vec<CodeNode>, text: &str) {
        if !text.is_empty() {
            nodes.push(CodeNode::Literal(text.to_string()));
        }
    }

    fn error(&self, message: &str) -> SigilStitchError {
        SigilStitchError::Render {
            context: format!("0.6.8 compatibility hook {}", self.hook),
            message: message.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_flattening_rejects_non_marker_structure() {
        let markers = LegacyTypeMarkers::new("test hook");
        let block = CodeBlock {
            nodes: vec![CodeNode::StringLit("value".to_string())],
        };

        let error = markers.flatten_marker_block(&block).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("could not preserve the legacy type-parameter format structurally")
        );
    }
}
