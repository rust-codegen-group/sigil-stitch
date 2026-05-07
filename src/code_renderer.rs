use pretty::BoxDoc;

use crate::code_block::CodeBlock;
use crate::code_node::CodeNode;
use crate::error::SigilStitchError;
use crate::import::ImportGroup;
use crate::lang::CodeLang;

/// Pass 2 of the three-pass rendering model.
///
/// Renders CodeBlock content to a string with:
/// - Resolved import names (final lengths known)
/// - Column tracking for width-aware `%T` rendering
/// - `%W` soft break support via `pretty` crate
/// - Proper indentation via `%>` / `%<`
pub struct CodeRenderer<'a> {
    lang: &'a dyn CodeLang,
    imports: &'a ImportGroup,
    width: usize,
    output: String,
    indent_level: usize,
    current_column: usize,
    at_line_start: bool,
}

impl<'a> CodeRenderer<'a> {
    /// Create a new renderer with the given language, imports, and target width.
    pub fn new(lang: &'a dyn CodeLang, imports: &'a ImportGroup, width: usize) -> Self {
        Self {
            lang,
            imports,
            width,
            output: String::new(),
            indent_level: 0,
            current_column: 0,
            at_line_start: true,
        }
    }

    /// Render a CodeBlock to string.
    pub fn render(&mut self, block: &CodeBlock) -> Result<String, SigilStitchError> {
        self.render_nodes(&block.nodes)?;
        Ok(std::mem::take(&mut self.output))
    }

    fn render_nodes(&mut self, nodes: &[CodeNode]) -> Result<(), SigilStitchError> {
        if contains_soft_break(nodes) {
            self.render_nodes_pretty(nodes)
        } else {
            self.render_nodes_direct(nodes)
        }
    }

    fn resolve_type_doc(&self, tn: &crate::type_name::TypeName) -> BoxDoc<'static, ()> {
        let lang = self.lang;
        let resolve = |module: &str, name: &str| -> String {
            let resolved = self
                .imports
                .resolved_name(module, name)
                .unwrap_or(name)
                .to_string();
            lang.qualify_import_name(module, &resolved)
        };
        tn.to_doc_with_lang(&resolve, self.lang)
    }

    /// Direct string rendering (no SoftBreak in this segment).
    fn render_nodes_direct(&mut self, nodes: &[CodeNode]) -> Result<(), SigilStitchError> {
        for node in nodes {
            match node {
                CodeNode::Literal(text) => {
                    self.emit_possibly_multiline(text);
                }
                CodeNode::TypeRef(tn) => {
                    self.ensure_indent();
                    let remaining_width = self.width.saturating_sub(self.current_column);
                    let doc = self.resolve_type_doc(tn);
                    let mut buf = Vec::new();
                    doc.render(remaining_width, &mut buf).map_err(|e| {
                        SigilStitchError::Render {
                            context: "CodeRenderer::render_nodes_direct TypeRef".to_string(),
                            message: e.to_string(),
                        }
                    })?;
                    let rendered =
                        String::from_utf8(buf).map_err(|e| SigilStitchError::Render {
                            context: "CodeRenderer::render_nodes_direct TypeRef UTF-8".to_string(),
                            message: e.to_string(),
                        })?;
                    let lines: Vec<&str> = rendered.split('\n').collect();
                    for (i, line) in lines.iter().enumerate() {
                        if i > 0 {
                            self.emit_newline();
                            self.ensure_indent();
                            let padding = " ".repeat(self.current_column);
                            self.emit(&padding);
                        }
                        self.emit(line);
                    }
                }
                CodeNode::NameRef(name) => {
                    self.ensure_indent();
                    self.emit(name);
                }
                CodeNode::StringLit(s) => {
                    self.ensure_indent();
                    let rendered = self.lang.render_string_literal(s);
                    self.emit(&rendered);
                }
                CodeNode::InlineLiteral(s) => {
                    self.emit_possibly_multiline(s);
                }
                CodeNode::Nested(block) => {
                    self.render_nodes(&block.nodes)?;
                }
                CodeNode::Comment(text) => {
                    self.ensure_indent();
                    let prefix = self.lang.line_comment_prefix();
                    let suffix = self.lang.line_comment_suffix();
                    self.emit(&format!("{prefix} {text}{suffix}"));
                }
                CodeNode::SoftBreak => {
                    self.emit(" ");
                }
                CodeNode::Indent => {
                    self.indent_level += 1;
                }
                CodeNode::Dedent => {
                    self.indent_level = self.indent_level.saturating_sub(1);
                }
                CodeNode::StatementBegin => {
                    self.ensure_indent();
                }
                CodeNode::StatementEnd => {
                    if self.lang.block_syntax().uses_semicolons {
                        self.emit(";");
                    }
                }
                CodeNode::Newline => {
                    self.emit_newline();
                }
                CodeNode::BlockOpen => {
                    self.emit(self.lang.block_syntax().block_open);
                }
                CodeNode::BlockOpenOverride(s) => {
                    self.emit(s);
                }
                CodeNode::BlockClose => {
                    let close = self.lang.block_syntax().block_close;
                    if !close.is_empty() {
                        self.ensure_indent();
                        self.emit(close);
                        self.emit_newline();
                    }
                }
                CodeNode::BlockCloseTransition => {
                    let cfg = self.lang.block_syntax();
                    if !cfg.block_close.is_empty() && cfg.close_on_transition {
                        self.ensure_indent();
                        self.emit(cfg.block_close);
                        self.emit(" ");
                    }
                }
                CodeNode::Sequence(children) => {
                    self.render_nodes(children)?;
                }
            }
        }
        Ok(())
    }

    /// Render a segment containing SoftBreak using the pretty crate.
    fn render_nodes_pretty(&mut self, nodes: &[CodeNode]) -> Result<(), SigilStitchError> {
        let doc = self.nodes_to_doc(nodes);
        let remaining_width = self.width.saturating_sub(self.current_column);
        let mut buf = Vec::new();
        doc.render(remaining_width, &mut buf)
            .map_err(|e| SigilStitchError::Render {
                context: "CodeRenderer::render_nodes_pretty".to_string(),
                message: e.to_string(),
            })?;
        let rendered = String::from_utf8(buf).map_err(|e| SigilStitchError::Render {
            context: "CodeRenderer::render_nodes_pretty UTF-8".to_string(),
            message: e.to_string(),
        })?;

        let lines: Vec<&str> = rendered.split('\n').collect();
        for (i, line) in lines.iter().enumerate() {
            if i > 0 {
                self.emit_newline();
                self.ensure_indent();
            }
            self.emit(line);
        }
        Ok(())
    }

    fn nodes_to_doc(&self, nodes: &[CodeNode]) -> BoxDoc<'static, ()> {
        let mut doc = BoxDoc::nil();

        for node in nodes {
            let node_doc = match node {
                CodeNode::Literal(text) => BoxDoc::text(text.clone()),
                CodeNode::TypeRef(tn) => self.resolve_type_doc(tn),
                CodeNode::NameRef(name) => BoxDoc::text(name.clone()),
                CodeNode::StringLit(s) => BoxDoc::text(self.lang.render_string_literal(s)),
                CodeNode::InlineLiteral(s) => BoxDoc::text(s.clone()),
                CodeNode::Nested(block) => self.nodes_to_doc(&block.nodes),
                CodeNode::Comment(text) => {
                    let prefix = self.lang.line_comment_prefix();
                    let suffix = self.lang.line_comment_suffix();
                    BoxDoc::text(format!("{prefix} {text}{suffix}"))
                }
                CodeNode::SoftBreak => BoxDoc::softline(),
                CodeNode::Indent | CodeNode::Dedent => BoxDoc::nil(),
                CodeNode::StatementBegin => BoxDoc::nil(),
                CodeNode::StatementEnd => {
                    if self.lang.block_syntax().uses_semicolons {
                        BoxDoc::text(";")
                    } else {
                        BoxDoc::nil()
                    }
                }
                CodeNode::Newline => BoxDoc::hardline(),
                CodeNode::BlockOpen => {
                    BoxDoc::text(self.lang.block_syntax().block_open.to_string())
                }
                CodeNode::BlockOpenOverride(s) => BoxDoc::text(s.clone()),
                CodeNode::BlockClose => {
                    let close = self.lang.block_syntax().block_close;
                    if close.is_empty() {
                        BoxDoc::nil()
                    } else {
                        BoxDoc::text(close.to_string()).append(BoxDoc::hardline())
                    }
                }
                CodeNode::BlockCloseTransition => {
                    let cfg = self.lang.block_syntax();
                    if cfg.block_close.is_empty() || !cfg.close_on_transition {
                        BoxDoc::nil()
                    } else {
                        BoxDoc::text(format!("{} ", cfg.block_close))
                    }
                }
                CodeNode::Sequence(children) => self.nodes_to_doc(children),
            };
            doc = doc.append(node_doc);
        }

        doc.group()
    }

    /// Emit a literal string, re-indenting each line when it spans multiple
    /// lines. Single-line input follows the fast path identical to
    /// `ensure_indent` + `emit`.
    fn emit_possibly_multiline(&mut self, text: &str) {
        if !text.contains('\n') {
            self.ensure_indent();
            self.emit(text);
            return;
        }
        for (i, line) in text.split('\n').enumerate() {
            if i > 0 {
                self.emit_newline();
            }
            if !line.is_empty() {
                self.ensure_indent();
                self.emit(line);
            }
        }
    }

    fn ensure_indent(&mut self) {
        if self.at_line_start {
            let indent_str = self.lang.block_syntax().indent_unit;
            for _ in 0..self.indent_level {
                self.output.push_str(indent_str);
                self.current_column += indent_str.len();
            }
            self.at_line_start = false;
        }
    }

    fn emit(&mut self, text: &str) {
        self.output.push_str(text);
        if let Some(last_nl) = text.rfind('\n') {
            self.current_column = text.len() - last_nl - 1;
        } else {
            self.current_column += text.len();
        }
    }

    fn emit_newline(&mut self) {
        self.output.push('\n');
        self.current_column = 0;
        self.at_line_start = true;
    }
}

fn contains_soft_break(nodes: &[CodeNode]) -> bool {
    nodes.iter().any(|n| match n {
        CodeNode::SoftBreak => true,
        CodeNode::Sequence(children) => contains_soft_break(children),
        _ => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_block::CodeBlock;
    use crate::import::ImportGroup;
    use crate::lang::typescript::TypeScript;
    use crate::type_name::TypeName;

    fn render_block(block: &CodeBlock, width: usize) -> String {
        let ts = TypeScript::new();
        let imports = ImportGroup::new();
        let mut renderer = CodeRenderer::new(&ts, &imports, width);
        renderer.render(block).unwrap()
    }

    #[test]
    fn test_simple_statement() {
        let mut b = CodeBlock::builder();
        b.add_statement("const x = 42", ());
        let block = b.build().unwrap();
        let output = render_block(&block, 80);
        assert_eq!(output.trim(), "const x = 42;");
    }

    #[test]
    fn test_control_flow() {
        let mut b = CodeBlock::builder();
        b.begin_control_flow("if (x > 0)", ());
        b.add_statement("return x", ());
        b.end_control_flow();
        let block = b.build().unwrap();
        let output = render_block(&block, 80);
        assert!(output.contains("if (x > 0) {"));
        assert!(output.contains("  return x;"));
        assert!(output.contains("}"));
    }

    #[test]
    fn test_if_else() {
        let mut b = CodeBlock::builder();
        b.begin_control_flow("if (x > 0)", ());
        b.add_statement("return x", ());
        b.next_control_flow("else", ());
        b.add_statement("return 0", ());
        b.end_control_flow();
        let block = b.build().unwrap();
        let output = render_block(&block, 80);
        assert!(output.contains("if (x > 0) {"));
        assert!(output.contains("} else {"));
        assert!(output.contains("  return 0;"));
    }

    #[test]
    fn test_type_rendering() {
        let user = TypeName::importable("./models", "User");
        let imports = ImportGroup {
            entries: vec![crate::import::ImportEntry {
                module: "./models".to_string(),
                name: "User".to_string(),
                alias: None,
                is_type_only: true,
                is_side_effect: false,
                is_wildcard: false,
            }],
        };

        let mut b = CodeBlock::builder();
        b.add_statement("const u: %T = getUser()", (user,));
        let block = b.build().unwrap();

        let ts = TypeScript::new();
        let mut renderer = CodeRenderer::new(&ts, &imports, 80);
        let output = renderer.render(&block).unwrap();
        assert_eq!(output.trim(), "const u: User = getUser();");
    }

    #[test]
    fn test_string_literal() {
        let mut b = CodeBlock::builder();
        b.add_statement(
            "const x = %S",
            (crate::code_block::StringLitArg("hello".to_string()),),
        );
        let block = b.build().unwrap();
        let output = render_block(&block, 80);
        assert_eq!(output.trim(), "const x = 'hello';");
    }

    #[test]
    fn test_nested_indent() {
        let mut b = CodeBlock::builder();
        b.begin_control_flow("if (a)", ());
        b.begin_control_flow("if (b)", ());
        b.add_statement("return c", ());
        b.end_control_flow();
        b.end_control_flow();
        let block = b.build().unwrap();
        let output = render_block(&block, 80);
        assert!(output.contains("    return c;"));
    }

    #[test]
    fn test_comment() {
        let mut b = CodeBlock::builder();
        b.add_comment("This is a comment");
        let block = b.build().unwrap();
        let output = render_block(&block, 80);
        assert!(output.contains("// This is a comment"));
    }

    #[test]
    fn test_multiline_literal_via_percent_l_reindents_each_line() {
        let mut b = CodeBlock::builder();
        b.begin_control_flow("interface User", ());
        b.add("%L", "/**\n * The user's name.\n */".to_string());
        b.add_line();
        b.add_statement("name: string", ());
        b.end_control_flow();
        let block = b.build().unwrap();
        let output = render_block(&block, 80);

        assert!(
            output.contains("  /**"),
            "first line of doc should be indented, got:\n{output}"
        );
        assert!(
            output.contains("   * The user's name."),
            "middle line of doc should be indented (indent + ' * ...'), got:\n{output}"
        );
        assert!(
            output.contains("   */"),
            "closing line of doc should be indented, got:\n{output}"
        );
        assert!(
            !output.contains("\n * The user's name."),
            "middle line must not be flush-left, got:\n{output}"
        );
        assert!(
            !output.contains("\n */"),
            "closing line must not be flush-left, got:\n{output}"
        );
    }

    #[test]
    fn test_multiline_literal_direct_reindents_each_line() {
        let mut b = CodeBlock::builder();
        b.begin_control_flow("function f()", ());
        b.add("line1\nline2\nline3", ());
        b.add_line();
        b.end_control_flow();
        let block = b.build().unwrap();
        let output = render_block(&block, 80);

        assert!(
            output.contains("  line1"),
            "first literal line should be indented, got:\n{output}"
        );
        assert!(
            output.contains("  line2"),
            "second literal line should be indented, got:\n{output}"
        );
        assert!(
            output.contains("  line3"),
            "third literal line should be indented, got:\n{output}"
        );
        assert!(
            !output.contains("\nline2"),
            "line2 must not be flush-left, got:\n{output}"
        );
    }

    #[test]
    fn test_block_open_override() {
        let mut b = CodeBlock::builder();
        b.begin_control_flow_with_open("class Functor f", (), " where");
        b.add_statement("fmap :: a -> b", ());
        b.end_control_flow();
        let block = b.build().unwrap();
        let output = render_block(&block, 80);
        assert!(
            output.contains("class Functor f where"),
            "should use custom opener, got:\n{output}"
        );
        assert!(
            !output.contains(" {"),
            "should NOT contain default block_open, got:\n{output}"
        );
    }
}
