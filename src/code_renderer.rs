mod direct;
mod pretty;
#[cfg(test)]
mod tests;

use crate::code_block::CodeBlock;
use crate::code_node::{BlockIntent, CodeNode};
use crate::error::SigilStitchError;
use crate::import::ImportGroup;
use crate::lang::RendererLang;
use crate::type_name::TypeName;
use crate::type_name_lowering::{DiagnosticPath, TypeNameMaterializer};
use direct::DirectAdapter;
use pretty::PrettyAdapter;

/// Terminal renderer for fully prepared structured source.
///
/// The renderer owns only stable configuration. Each render call creates one
/// call-local output adapter and interprets every node through the same
/// semantic walker.
pub struct CodeRenderer<'a> {
    lang: &'a dyn RendererLang,
    imports: &'a ImportGroup,
    width: usize,
}

impl<'a> CodeRenderer<'a> {
    /// Create a new renderer with the given language, imports, and target width.
    pub fn new(lang: &'a dyn RendererLang, imports: &'a ImportGroup, width: usize) -> Self {
        Self {
            lang,
            imports,
            width,
        }
    }

    /// Render a CodeBlock to string.
    pub fn render(&mut self, block: &CodeBlock) -> Result<String, SigilStitchError> {
        let mut materializer = TypeNameMaterializer::new(self.lang);
        let prepared =
            materializer.prepare_source_block(block, DiagnosticPath::root("standalone"))?;
        self.render_prepared(&prepared)
    }

    pub(crate) fn render_prepared(&self, block: &CodeBlock) -> Result<String, SigilStitchError> {
        let nodes = &block.nodes;
        let indent_unit = self.lang.indent_unit();
        if contains_soft_break(nodes) {
            let mut adapter = PrettyAdapter::new(indent_unit, self.width);
            adapter.begin_group(LayoutGroup::IndependentBreaks)?;
            self.walk_nodes(nodes, &mut adapter)?;
            adapter.end_group()?;
            adapter.finish()
        } else {
            let mut adapter = DirectAdapter::new(indent_unit, self.width);
            adapter.begin_group(LayoutGroup::IndependentBreaks)?;
            self.walk_nodes(nodes, &mut adapter)?;
            adapter.end_group()?;
            Ok(adapter.finish())
        }
    }

    fn resolve_terminal_type(&self, type_name: &TypeName) -> Result<String, SigilStitchError> {
        match type_name {
            TypeName::Primitive(name) | TypeName::Raw(name) => Ok(name.clone()),
            TypeName::Importable {
                module,
                name,
                qualified: false,
                ..
            } => {
                let resolved = self
                    .imports
                    .resolved_name(module, name)
                    .unwrap_or(name)
                    .to_string();
                Ok(self.lang.qualify_import_reference(module, name, &resolved))
            }
            _ => Err(SigilStitchError::UnexpectedTypeReference {
                context: "prepared renderer tree".to_string(),
            }),
        }
    }

    fn resolve_comment(lang: &dyn RendererLang, text: &str) -> String {
        let prefix = lang.line_comment_prefix();
        let suffix = lang.line_comment_suffix();
        text.split('\n')
            .map(|line| {
                if line.is_empty() {
                    format!("{prefix}{suffix}")
                } else {
                    format!("{prefix} {line}{suffix}")
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[expect(deprecated, reason = "legacy block node compatibility bridge")]
    fn walk_nodes<A: RenderAdapter>(
        &self,
        nodes: &[CodeNode],
        adapter: &mut A,
    ) -> Result<(), SigilStitchError> {
        for node in nodes {
            match node {
                CodeNode::Literal(text) => adapter.structured_text(text)?,
                CodeNode::TypeRef(type_name) => {
                    adapter.structured_text(&self.resolve_terminal_type(type_name)?)?;
                }
                CodeNode::NameRef(name) => {
                    adapter.structured_text(&self.lang.escape_reserved(name))?;
                }
                CodeNode::StringLit(value) => {
                    adapter.opaque_text(&self.lang.render_string_literal(value))?;
                }
                CodeNode::VerbatimStr(value) => {
                    adapter.opaque_text(&self.lang.render_verbatim_string(value))?;
                }
                CodeNode::InlineLiteral(text) => adapter.structured_text(text)?,
                CodeNode::Nested(block) => {
                    adapter.begin_group(LayoutGroup::IndependentBreaks)?;
                    self.walk_nodes(&block.nodes, adapter)?;
                    adapter.end_group()?;
                }
                CodeNode::Comment(text) => {
                    adapter.structured_text(&Self::resolve_comment(self.lang, text))?;
                }
                CodeNode::Attribute(text) => {
                    adapter.structured_text(&self.lang.render_attribute(text))?;
                }
                CodeNode::SoftBreak => adapter.soft_break()?,
                CodeNode::Indent => adapter.indent()?,
                CodeNode::Dedent => adapter.dedent()?,
                CodeNode::StatementBegin => adapter.ensure_indent()?,
                CodeNode::StatementEnd => {
                    let suffix = self.lang.render_statement_end()?;
                    if !suffix.is_empty() {
                        adapter.structured_text(suffix)?;
                    }
                }
                CodeNode::Newline => adapter.hard_break()?,
                CodeNode::BlockOpen(condition) => {
                    let open = self
                        .lang
                        .render_block_open(BlockIntent::Generic, condition)?;
                    if !open.is_empty() {
                        adapter.structured_text(open)?;
                    }
                }
                CodeNode::BlockClose(condition) => {
                    let close = self
                        .lang
                        .render_block_close(BlockIntent::Generic, condition)?;
                    if !close.is_empty() {
                        adapter.structured_text(close)?;
                    }
                }
                CodeNode::BranchClose(condition) => {
                    let transition = self
                        .lang
                        .render_branch_transition(BlockIntent::Generic, condition)?;
                    if !transition.is_empty() {
                        adapter.structured_text(&transition)?;
                    }
                }
                CodeNode::BlockOpenIntent { condition, intent } => {
                    let open = self.lang.render_block_open(*intent, condition)?;
                    if !open.is_empty() {
                        adapter.structured_text(open)?;
                    }
                }
                CodeNode::BlockCloseIntent { condition, intent } => {
                    let close = self.lang.render_block_close(*intent, condition)?;
                    if !close.is_empty() {
                        adapter.structured_text(close)?;
                    }
                }
                CodeNode::BranchCloseIntent { condition, intent } => {
                    let transition = self.lang.render_branch_transition(*intent, condition)?;
                    if !transition.is_empty() {
                        adapter.structured_text(&transition)?;
                    }
                }
                CodeNode::Sequence(children) => {
                    adapter.begin_group(LayoutGroup::ConsistentBreaks)?;
                    self.walk_nodes(children, adapter)?;
                    adapter.end_group()?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum LayoutGroup {
    IndependentBreaks,
    ConsistentBreaks,
}

trait RenderAdapter {
    fn raw_text(&mut self, text: &str) -> Result<(), SigilStitchError>;
    fn ensure_indent(&mut self) -> Result<(), SigilStitchError>;
    fn hard_break(&mut self) -> Result<(), SigilStitchError>;
    fn soft_break(&mut self) -> Result<(), SigilStitchError>;
    fn indent(&mut self) -> Result<(), SigilStitchError>;
    fn dedent(&mut self) -> Result<(), SigilStitchError>;
    fn begin_group(&mut self, group: LayoutGroup) -> Result<(), SigilStitchError>;
    fn end_group(&mut self) -> Result<(), SigilStitchError>;

    fn structured_text(&mut self, text: &str) -> Result<(), SigilStitchError> {
        for (index, line) in text.split('\n').enumerate() {
            if index > 0 {
                self.hard_break()?;
            }
            if !line.is_empty() {
                self.ensure_indent()?;
                self.raw_text(line)?;
            }
        }
        Ok(())
    }

    fn opaque_text(&mut self, text: &str) -> Result<(), SigilStitchError> {
        for (index, line) in text.split('\n').enumerate() {
            if index > 0 {
                self.hard_break()?;
            }
            if !line.is_empty() {
                if index == 0 {
                    self.ensure_indent()?;
                }
                self.raw_text(line)?;
            }
        }
        Ok(())
    }
}

fn contains_soft_break(nodes: &[CodeNode]) -> bool {
    nodes.iter().any(|node| match node {
        CodeNode::SoftBreak => true,
        CodeNode::Nested(block) => contains_soft_break(&block.nodes),
        CodeNode::Sequence(children) => contains_soft_break(children),
        _ => false,
    })
}
