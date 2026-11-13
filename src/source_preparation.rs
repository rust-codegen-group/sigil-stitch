use crate::code_block::CodeBlock;
use crate::code_node::CodeNode;
use crate::error::SigilStitchError;
use crate::lang::RendererLang;
use crate::type_name::TypeName;
use crate::type_name_lowering::DiagnosticPath;
use crate::type_name_lowering::validation::{validate_lowered_block, validate_type_name};

pub(crate) struct SourcePreparer<'a> {
    lang: &'a dyn RendererLang,
}

impl<'a> SourcePreparer<'a> {
    pub(crate) fn new(lang: &'a dyn RendererLang) -> Self {
        Self { lang }
    }

    pub(crate) fn prepare_source_block(
        &mut self,
        block: &CodeBlock,
        path: DiagnosticPath,
    ) -> Result<CodeBlock, SigilStitchError> {
        let mut rewritten = block.rewrite_source(&path, |nodes| self.lang.rewrite_nodes(nodes))?;
        rewritten.nodes = self.prepare_nodes(rewritten.nodes, &path)?;
        Ok(rewritten)
    }

    pub(crate) fn prepare_metadata_type(
        &mut self,
        type_name: &TypeName,
        path: DiagnosticPath,
    ) -> Result<CodeBlock, SigilStitchError> {
        self.lower_type(type_name, &path)
    }

    fn prepare_nodes(
        &mut self,
        nodes: Vec<CodeNode>,
        path: &DiagnosticPath,
    ) -> Result<Vec<CodeNode>, SigilStitchError> {
        // Consume each source vector once. Float expressions are prepared for
        // type lowering, while validated type terminals are not lowered again.
        let mut prepared = Vec::with_capacity(nodes.len());
        for (index, node) in nodes.into_iter().enumerate() {
            match node {
                CodeNode::TypeRef(type_name) => {
                    let lowered = self.lower_type(&type_name, &path.node(index))?;
                    prepared.push(CodeNode::Nested(lowered));
                }
                CodeNode::FloatValue(value) => {
                    let mut lowered = crate::float_value::lower(&value, self.lang)?;
                    lowered.nodes =
                        self.prepare_nodes(lowered.nodes, &path.node(index).child("float_value"))?;
                    prepared.push(CodeNode::Nested(lowered));
                }
                CodeNode::Nested(mut block) => {
                    block.nodes = self.prepare_nodes(block.nodes, &path.nested(index))?;
                    prepared.push(CodeNode::Nested(block));
                }
                CodeNode::Sequence(children) => {
                    prepared.push(CodeNode::Sequence(
                        self.prepare_nodes(children, &path.sequence(index))?,
                    ));
                }
                other => prepared.push(other),
            }
        }
        Ok(prepared)
    }

    fn lower_type(
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
