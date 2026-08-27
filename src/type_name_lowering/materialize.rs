use crate::code_block::CodeBlock;
use crate::code_node::CodeNode;
use crate::error::SigilStitchError;
use crate::lang::RendererLang;
use crate::type_name::TypeName;

use super::DiagnosticPath;
use super::validation::{validate_lowered_block, validate_rewritten_block, validate_type_name};

pub(crate) struct TypeNameMaterializer<'a> {
    lang: &'a dyn RendererLang,
}

impl<'a> TypeNameMaterializer<'a> {
    pub(crate) fn new(lang: &'a dyn RendererLang) -> Self {
        Self { lang }
    }

    pub(crate) fn prepare_source_block(
        &mut self,
        block: &CodeBlock,
        path: DiagnosticPath,
    ) -> Result<CodeBlock, SigilStitchError> {
        let mut rewritten = block.clone();
        self.lang.rewrite_nodes(&mut rewritten.nodes);
        validate_rewritten_block(&rewritten, &path)?;
        rewritten.nodes = self.materialize_nodes(rewritten.nodes, &path)?;
        Ok(rewritten)
    }

    pub(crate) fn prepare_metadata_type(
        &mut self,
        type_name: &TypeName,
        path: DiagnosticPath,
    ) -> Result<CodeBlock, SigilStitchError> {
        self.lower(type_name, &path)
    }

    fn materialize_nodes(
        &mut self,
        nodes: Vec<CodeNode>,
        path: &DiagnosticPath,
    ) -> Result<Vec<CodeNode>, SigilStitchError> {
        // Consume each vector once and replace every original TypeRef with one
        // validated lowered block; terminal leaves are never lowered again.
        let mut prepared = Vec::with_capacity(nodes.len());
        for (index, node) in nodes.into_iter().enumerate() {
            match node {
                CodeNode::TypeRef(type_name) => {
                    let lowered = self.lower(&type_name, &path.node(index))?;
                    prepared.push(CodeNode::Nested(lowered));
                }
                CodeNode::Nested(mut block) => {
                    block.nodes = self.materialize_nodes(block.nodes, &path.nested(index))?;
                    prepared.push(CodeNode::Nested(block));
                }
                CodeNode::Sequence(children) => {
                    prepared.push(CodeNode::Sequence(
                        self.materialize_nodes(children, &path.sequence(index))?,
                    ));
                }
                other => prepared.push(other),
            }
        }
        Ok(prepared)
    }

    fn lower(
        &mut self,
        type_name: &TypeName,
        path: &DiagnosticPath,
    ) -> Result<CodeBlock, SigilStitchError> {
        validate_type_name(type_name, path)?;
        let lowered = self
            .lang
            .lower_type_name(type_name)
            .map_err(|error| match error {
                SigilStitchError::UnsupportedTypeName {
                    language, reason, ..
                } => SigilStitchError::UnsupportedTypeName {
                    language,
                    context: path.to_string(),
                    reason,
                },
                other => other,
            })?;
        validate_lowered_block(&lowered, self.lang.file_extension(), path)?;
        Ok(lowered)
    }
}
