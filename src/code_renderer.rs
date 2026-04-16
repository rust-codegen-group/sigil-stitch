use pretty::RcDoc;

use crate::code_block::{Arg, CodeBlock, FormatPart};
use crate::import::ImportGroup;
use crate::lang::CodeLang;

/// Pass 2 of the three-pass rendering model.
///
/// Renders CodeBlock content to a string with:
/// - Resolved import names (final lengths known)
/// - Column tracking for width-aware `%T` rendering
/// - `%W` soft break support via `pretty` crate
/// - Proper indentation via `%>` / `%<`
pub struct CodeRenderer<'a, L: CodeLang> {
    lang: &'a L,
    imports: &'a ImportGroup,
    width: usize,
    output: String,
    indent_level: usize,
    current_column: usize,
    at_line_start: bool,
}

impl<'a, L: CodeLang> CodeRenderer<'a, L> {
    /// Create a new renderer with the given language, imports, and target width.
    pub fn new(lang: &'a L, imports: &'a ImportGroup, width: usize) -> Self {
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
    pub fn render(&mut self, block: &CodeBlock<L>) -> String {
        let mut arg_index = 0;
        self.render_parts(&block.parts, &block.args, &mut arg_index);
        std::mem::take(&mut self.output)
    }

    fn render_parts(&mut self, parts: &[FormatPart], args: &[Arg<L>], arg_index: &mut usize) {
        // Check if this sequence of parts contains any %W (soft breaks).
        // If so, we build the whole thing as an RcDoc and let pretty decide.
        let has_wrap = parts.iter().any(|p| matches!(p, FormatPart::Wrap));

        if has_wrap {
            self.render_with_pretty(parts, args, arg_index);
        } else {
            self.render_direct(parts, args, arg_index);
        }
    }

    /// Direct string rendering (no %W in this segment).
    fn render_direct(&mut self, parts: &[FormatPart], args: &[Arg<L>], arg_index: &mut usize) {
        for part in parts {
            match part {
                FormatPart::Literal(text) => {
                    if let Some(comment_text) = text.strip_prefix("__COMMENT__") {
                        self.ensure_indent();
                        let prefix = self.lang.line_comment_prefix();
                        self.emit(&format!("{prefix} {comment_text}"));
                    } else {
                        self.ensure_indent();
                        self.emit(text);
                    }
                }
                FormatPart::Type => {
                    let arg = &args[*arg_index];
                    *arg_index += 1;
                    if let Arg::TypeName(tn) = arg {
                        self.ensure_indent();
                        let remaining_width = self.width.saturating_sub(self.current_column);
                        let lang = self.lang;
                        let resolve = |module: &str, name: &str| -> String {
                            let resolved = self
                                .imports
                                .resolved_name(module, name)
                                .unwrap_or(name)
                                .to_string();
                            lang.qualify_import_name(module, &resolved)
                        };
                        let doc = tn.to_doc_with_lang(&resolve, self.lang);
                        let mut buf = Vec::new();
                        doc.render(remaining_width, &mut buf).unwrap();
                        let rendered = String::from_utf8(buf).unwrap();
                        // Handle multi-line output: indent continuation lines.
                        let lines: Vec<&str> = rendered.split('\n').collect();
                        for (i, line) in lines.iter().enumerate() {
                            if i > 0 {
                                self.emit_newline();
                                self.ensure_indent();
                                // Add continuation indent to align with first line.
                                let padding = " ".repeat(self.current_column);
                                self.emit(&padding);
                            }
                            self.emit(line);
                        }
                    }
                }
                FormatPart::Name => {
                    let arg = &args[*arg_index];
                    *arg_index += 1;
                    if let Arg::Name(name) = arg {
                        self.ensure_indent();
                        self.emit(name);
                    }
                }
                FormatPart::StringLit => {
                    let arg = &args[*arg_index];
                    *arg_index += 1;
                    if let Arg::StringLit(s) = arg {
                        self.ensure_indent();
                        let rendered = self.lang.render_string_literal(s);
                        self.emit(&rendered);
                    }
                }
                FormatPart::Literal_ => {
                    let arg = &args[*arg_index];
                    *arg_index += 1;
                    match arg {
                        Arg::Literal(s) => {
                            self.ensure_indent();
                            self.emit(s);
                        }
                        Arg::Code(block) => {
                            let mut inner_idx = 0;
                            self.render_parts(&block.parts, &block.args, &mut inner_idx);
                        }
                        _ => {}
                    }
                }
                FormatPart::Wrap => {
                    // Should not reach here in direct mode, but just emit a space.
                    self.emit(" ");
                }
                FormatPart::Indent => {
                    self.indent_level += 1;
                }
                FormatPart::Dedent => {
                    self.indent_level = self.indent_level.saturating_sub(1);
                }
                FormatPart::StatementBegin => {
                    self.ensure_indent();
                }
                FormatPart::StatementEnd => {
                    if self.lang.uses_semicolons() {
                        self.emit(";");
                    }
                }
                FormatPart::Newline => {
                    self.emit_newline();
                }
                FormatPart::BlockOpen => {
                    self.emit(self.lang.block_open());
                }
                FormatPart::BlockClose => {
                    let close = self.lang.block_close();
                    if !close.is_empty() {
                        self.ensure_indent();
                        self.emit(close);
                    }
                }
                FormatPart::BlockCloseTransition => {
                    let close = self.lang.block_close();
                    if !close.is_empty() {
                        self.ensure_indent();
                        self.emit(close);
                        self.emit(" ");
                    }
                }
            }
        }
    }

    /// Render a segment containing %W using the pretty crate.
    fn render_with_pretty(&mut self, parts: &[FormatPart], args: &[Arg<L>], arg_index: &mut usize) {
        // Build an RcDoc from the parts, using softline() for %W.
        let doc = self.build_doc_from_parts(parts, args, arg_index);
        let remaining_width = self.width.saturating_sub(self.current_column);
        let mut buf = Vec::new();
        doc.render(remaining_width, &mut buf).unwrap();
        let rendered = String::from_utf8(buf).unwrap();

        let lines: Vec<&str> = rendered.split('\n').collect();
        for (i, line) in lines.iter().enumerate() {
            if i > 0 {
                self.emit_newline();
                self.ensure_indent();
            }
            self.emit(line);
        }
    }

    fn build_doc_from_parts(
        &self,
        parts: &[FormatPart],
        args: &[Arg<L>],
        arg_index: &mut usize,
    ) -> RcDoc<'static, ()> {
        let mut doc = RcDoc::nil();

        for part in parts {
            let part_doc = match part {
                FormatPart::Literal(text) => {
                    if let Some(comment_text) = text.strip_prefix("__COMMENT__") {
                        let prefix = self.lang.line_comment_prefix();
                        RcDoc::text(format!("{prefix} {comment_text}"))
                    } else {
                        RcDoc::text(text.clone())
                    }
                }
                FormatPart::Type => {
                    let arg = &args[*arg_index];
                    *arg_index += 1;
                    if let Arg::TypeName(tn) = arg {
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
                    } else {
                        RcDoc::nil()
                    }
                }
                FormatPart::Name => {
                    let arg = &args[*arg_index];
                    *arg_index += 1;
                    if let Arg::Name(name) = arg {
                        RcDoc::text(name.clone())
                    } else {
                        RcDoc::nil()
                    }
                }
                FormatPart::StringLit => {
                    let arg = &args[*arg_index];
                    *arg_index += 1;
                    if let Arg::StringLit(s) = arg {
                        RcDoc::text(self.lang.render_string_literal(s))
                    } else {
                        RcDoc::nil()
                    }
                }
                FormatPart::Literal_ => {
                    let arg = &args[*arg_index];
                    *arg_index += 1;
                    match arg {
                        Arg::Literal(s) => RcDoc::text(s.clone()),
                        Arg::Code(block) => {
                            let mut inner_idx = 0;
                            self.build_doc_from_parts(&block.parts, &block.args, &mut inner_idx)
                        }
                        _ => RcDoc::nil(),
                    }
                }
                FormatPart::Wrap => RcDoc::softline(),
                FormatPart::Indent | FormatPart::Dedent => {
                    // Indent/dedent in pretty mode is handled by nest().
                    RcDoc::nil()
                }
                FormatPart::StatementBegin => RcDoc::nil(),
                FormatPart::StatementEnd => {
                    if self.lang.uses_semicolons() {
                        RcDoc::text(";")
                    } else {
                        RcDoc::nil()
                    }
                }
                FormatPart::Newline => RcDoc::hardline(),
                FormatPart::BlockOpen => RcDoc::text(self.lang.block_open().to_string()),
                FormatPart::BlockClose => {
                    let close = self.lang.block_close();
                    if close.is_empty() {
                        RcDoc::nil()
                    } else {
                        RcDoc::text(close.to_string())
                    }
                }
                FormatPart::BlockCloseTransition => {
                    let close = self.lang.block_close();
                    if close.is_empty() {
                        RcDoc::nil()
                    } else {
                        RcDoc::text(format!("{} ", close))
                    }
                }
            };
            doc = doc.append(part_doc);
        }

        doc.group()
    }

    fn ensure_indent(&mut self) {
        if self.at_line_start {
            let indent_str = self.lang.indent_unit();
            for _ in 0..self.indent_level {
                self.output.push_str(indent_str);
                self.current_column += indent_str.len();
            }
            self.at_line_start = false;
        }
    }

    fn emit(&mut self, text: &str) {
        self.output.push_str(text);
        // Update column tracking. Only count from last newline.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_block::CodeBlock;
    use crate::import::ImportGroup;
    use crate::lang::typescript::TypeScript;
    use crate::type_name::TypeName;

    fn render_block(block: &CodeBlock<TypeScript>, width: usize) -> String {
        let ts = TypeScript::new();
        let imports = ImportGroup::new();
        let mut renderer = CodeRenderer::new(&ts, &imports, width);
        renderer.render(block)
    }

    #[test]
    fn test_simple_statement() {
        let mut b = CodeBlock::<TypeScript>::builder();
        b.add_statement("const x = 42", ());
        let block = b.build().unwrap();
        let output = render_block(&block, 80);
        assert_eq!(output.trim(), "const x = 42;");
    }

    #[test]
    fn test_control_flow() {
        let mut b = CodeBlock::<TypeScript>::builder();
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
        let mut b = CodeBlock::<TypeScript>::builder();
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
        let user = TypeName::<TypeScript>::importable("./models", "User");
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

        let mut b = CodeBlock::<TypeScript>::builder();
        b.add_statement("const u: %T = getUser()", (user,));
        let block = b.build().unwrap();

        let ts = TypeScript::new();
        let mut renderer = CodeRenderer::new(&ts, &imports, 80);
        let output = renderer.render(&block);
        assert_eq!(output.trim(), "const u: User = getUser();");
    }

    #[test]
    fn test_string_literal() {
        let mut b = CodeBlock::<TypeScript>::builder();
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
        let mut b = CodeBlock::<TypeScript>::builder();
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
        let mut b = CodeBlock::<TypeScript>::builder();
        b.add_comment("This is a comment");
        let block = b.build().unwrap();
        let output = render_block(&block, 80);
        assert!(output.contains("// This is a comment"));
    }
}
