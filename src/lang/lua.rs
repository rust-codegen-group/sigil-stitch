//! Lua language implementation.
//!
//! Lua characteristics:
//! - Dynamically typed (no type annotations)
//! - `function` keyword, `end` for block close
//! - `--` line comments, `---` doc comments
//! - `then`/`do` after control flow conditions
//! - `local` variable declarations
//! - `require("module")` imports
//! - 2-space indent by convention

use crate::code_node::BlockIntent;
use crate::import::ImportGroup;
use crate::lang::config::{
    BlockSyntaxConfig, EnumAndAnnotationConfig, FunctionSyntaxConfig, GenericSyntaxConfig,
    TypeDeclSyntaxConfig, TypePresentationConfig,
};
use crate::lang::{CodeLang, RendererLang};
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};

fn lua_block_open_for_intent(intent: BlockIntent, condition: &str) -> Option<&'static str> {
    let t = condition.trim();
    match intent {
        BlockIntent::If | BlockIntent::ElseIf => {
            if t.ends_with(" then") {
                Some("")
            } else {
                Some(" then")
            }
        }
        BlockIntent::For | BlockIntent::While => {
            if t.ends_with(" do") {
                Some("")
            } else {
                Some(" do")
            }
        }
        BlockIntent::Else => Some(""),
        _ => None,
    }
}

/// Lua language implementation.
#[derive(Debug, Clone)]
pub struct Lua {
    /// Indent with this string (default: `"  "` — 2 spaces by convention).
    pub indent: String,
    /// File extension (default: `"lua"`).
    pub extension: String,
}

impl Default for Lua {
    fn default() -> Self {
        Self {
            indent: "  ".to_string(),
            extension: "lua".to_string(),
        }
    }
}

fn lua_method_call_follows(chars: &[char], start: usize) -> bool {
    let mut index = start;
    while index < chars.len()
        && (chars[index].is_alphanumeric() || chars[index] == '_' || chars[index] == '.')
    {
        index += 1;
    }
    matches!(chars.get(index), Some('(') | Some('{'))
}

impl Lua {
    /// Create a new Lua language instance with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Rewrite `: ` to `:` for method calls (Lua `obj:method()` syntax).
    #[allow(clippy::ptr_arg)]
    fn rewrite_method_colon(nodes: &mut Vec<crate::code_node::CodeNode>) {
        use crate::code_node::CodeNode;
        for node in nodes.iter_mut() {
            if let CodeNode::Literal(s) | CodeNode::InlineLiteral(s) = node {
                let mut result = String::with_capacity(s.len());
                let chars: Vec<char> = s.chars().collect();
                let mut i = 0;
                while i < chars.len() {
                    if chars[i] == ':'
                        && i > 0
                        && i + 1 < chars.len()
                        && chars[i - 1].is_alphanumeric()
                        && chars[i + 1] == ' '
                        && i + 2 < chars.len()
                        && (chars[i + 2].is_alphanumeric() || chars[i + 2] == '_')
                        && lua_method_call_follows(&chars, i + 2)
                    {
                        // Skip the space after ':'
                        result.push(':');
                        i += 2; // skip ':' and ' '
                    } else {
                        result.push(chars[i]);
                        i += 1;
                    }
                }
                *s = result;
            }
        }
    }
}

/// Lua keywords (Lua 5.x).
/// Contextual keyword `self` intentionally not included — treat as regular ident.
const LUA_RESERVED: &[&str] = &[
    "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "goto", "if", "in",
    "local", "nil", "not", "or", "repeat", "return", "then", "true", "until", "while",
];

impl RendererLang for Lua {
    fn file_extension(&self) -> &str {
        &self.extension
    }

    fn line_comment_prefix(&self) -> &str {
        "--"
    }

    fn render_string_literal(&self, s: &str) -> String {
        let escaped = s
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\t', "\\t")
            .replace('\r', "\\r");
        format!("\"{}\"", escaped)
    }

    fn reserved_words(&self) -> &[&str] {
        LUA_RESERVED
    }

    fn module_separator(&self) -> Option<&str> {
        Some(".")
    }

    // ── Config accessors ──

    fn block_syntax(&self) -> BlockSyntaxConfig<'_> {
        BlockSyntaxConfig {
            block_open: "",
            block_close: "end",
            close_on_transition: false,
            indent_unit: &self.indent,
            uses_semicolons: false,
            field_terminator: ",",
            ..Default::default()
        }
    }

    #[allow(deprecated)]
    fn block_open_for(&self, condition: &str) -> Option<&str> {
        let t = condition.trim();
        if t.ends_with(" then") || t.ends_with(" do") || t == "else" {
            Some("")
        } else if t.starts_with("if ") || t.starts_with("elseif ") {
            Some(" then")
        } else if t.starts_with("for ") || t.starts_with("while ") {
            Some(" do")
        } else {
            None
        }
    }

    fn block_open_for_intent(&self, intent: BlockIntent, condition: &str) -> Option<&str> {
        lua_block_open_for_intent(intent, condition)
    }

    fn generic_syntax(&self) -> GenericSyntaxConfig<'_> {
        GenericSyntaxConfig {
            open: "",
            close: "",
            ..Default::default()
        }
    }

    fn type_presentation(&self) -> TypePresentationConfig<'_> {
        TypePresentationConfig {
            optional_absent_literal: "nil",
            ..Default::default()
        }
    }

    fn rewrite_nodes(&self, nodes: &mut Vec<crate::code_node::CodeNode>) {
        crate::lang::rewrite::walk_nodes_mut(nodes, &Self::rewrite_method_colon);
    }
}

impl CodeLang for Lua {
    fn render_visibility(&self, _vis: Visibility, _ctx: DeclarationContext) -> &str {
        ""
    }

    fn function_keyword(&self, _ctx: DeclarationContext) -> &str {
        "function"
    }

    fn type_keyword(&self, _kind: TypeKind) -> &str {
        // Lua has no class/struct/enum keywords — tables and metatables fill
        // that role. TypeSpec should not be used with Lua; use CodeBlock
        // directly for table constructors and function definitions instead.
        // If TypeSpec IS used, this returns "" so the name emits as a bare
        // identifier block (valid Lua, treated as a scope by the interpreter).
        ""
    }

    fn render_imports(&self, imports: &ImportGroup) -> String {
        let mut lines = Vec::new();
        for entry in imports.entries() {
            let module = entry.module.replace('.', "/");
            if entry.name.is_empty() || entry.is_side_effect {
                lines.push(format!("require(\"{}\");", module));
            } else {
                let name = entry.resolved_name();
                lines.push(format!("local {} = require(\"{}\");", name, module));
            }
        }
        lines.join("\n")
    }

    fn render_doc_comment(&self, lines: &[&str]) -> String {
        let mut out = String::new();
        for line in lines {
            if line.is_empty() {
                out.push_str("---\n");
            } else {
                out.push_str(&format!("--- {}\n", line));
            }
        }
        out
    }

    fn fun_block_open(&self) -> &str {
        "" // Function bodies start on the next line
    }

    fn type_header_block_open(&self, _kind: TypeKind) -> &str {
        "" // Type bodies start on the next line
    }

    fn type_decl_syntax(&self) -> TypeDeclSyntaxConfig<'_> {
        TypeDeclSyntaxConfig {
            type_before_name: false,
            return_type_is_prefix: false,
            type_annotation_separator: "", // Lua has no type annotations
            ..Default::default()
        }
    }

    fn function_syntax(&self) -> FunctionSyntaxConfig<'_> {
        FunctionSyntaxConfig {
            return_type_separator: "", // No return type syntax in Lua
            empty_body: "",
            ..Default::default()
        }
    }

    fn enum_and_annotation(&self) -> EnumAndAnnotationConfig<'_> {
        EnumAndAnnotationConfig {
            readonly_keyword: "",
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::import::ImportEntry;

    use super::*;

    #[test]
    fn test_line_comment() {
        let lua = Lua::new();
        assert_eq!(lua.line_comment_prefix(), "--");
    }

    #[test]
    fn test_render_string_literal() {
        let lua = Lua::new();
        assert_eq!(
            lua.render_string_literal("hello 'world'"),
            r#""hello 'world'""#
        );
        assert_eq!(lua.render_string_literal("say \"hi\""), r#""say \"hi\"""#);
        assert_eq!(
            lua.render_string_literal("line1\nline2"),
            r#""line1\nline2""#
        );
    }

    #[test]
    fn test_escape_reserved() {
        let lua = Lua::new();
        assert_eq!(lua.escape_reserved("foo"), "foo");
        assert_eq!(lua.escape_reserved("function"), "function_");
        assert_eq!(lua.escape_reserved("end"), "end_");
    }

    #[test]
    fn test_render_imports() {
        let lua = Lua::new();
        let entries = vec![
            ImportEntry {
                module: "path.to.mod".to_string(),
                name: "Mod".to_string(),
                alias: None,
                is_type_only: false,
                is_side_effect: false,
                is_wildcard: false,
            },
            ImportEntry {
                module: "some.lib".to_string(),
                name: String::new(),
                alias: None,
                is_type_only: false,
                is_side_effect: true,
                is_wildcard: false,
            },
        ];
        let group = ImportGroup::from(entries);
        let out = lua.render_imports(&group);
        assert!(out.contains(r#"local Mod = require("path/to/mod")"#));
        assert!(out.contains(r#"require("some/lib")"#));
        assert!(!out.ends_with('\n'), "no trailing newline");
    }

    #[test]
    fn test_render_doc_comment() {
        let lua = Lua::new();
        let lines = &["Says hello.", "", "Returns a greeting."];
        let rendered = lua.render_doc_comment(lines);
        assert_eq!(rendered, "--- Says hello.\n---\n--- Returns a greeting.\n");
    }

    #[test]
    fn test_block_syntax() {
        let lua = Lua::new();
        let bs = lua.block_syntax();
        assert_eq!(bs.block_open, "");
        assert_eq!(bs.block_close, "end");
        assert!(!bs.uses_semicolons);
        assert!(!bs.close_on_transition);
    }

    #[test]
    fn test_no_visibility() {
        let lua = Lua::new();
        assert_eq!(
            lua.render_visibility(Visibility::Public, DeclarationContext::TopLevel),
            ""
        );
    }

    #[test]
    fn test_function_keyword() {
        let lua = Lua::new();
        assert_eq!(
            lua.function_keyword(DeclarationContext::TopLevel),
            "function"
        );
    }

    #[test]
    fn test_block_intent_openers() {
        let lua = Lua::new();
        assert_eq!(
            lua.block_open_for_intent(BlockIntent::If, "if x > 0"),
            Some(" then")
        );
        assert_eq!(
            lua.block_open_for_intent(BlockIntent::If, "if x > 0 then"),
            Some("")
        );
        assert_eq!(
            lua.block_open_for_intent(BlockIntent::ElseIf, "elseif x < 0 then"),
            Some("")
        );
        assert_eq!(
            lua.block_open_for_intent(BlockIntent::Else, "else"),
            Some("")
        );
        assert_eq!(
            lua.block_open_for_intent(BlockIntent::For, "for i = 1, 10"),
            Some(" do")
        );
        assert_eq!(
            lua.block_open_for_intent(BlockIntent::While, "while x > 0 do"),
            Some("")
        );
        assert_eq!(
            lua.block_open_for_intent(BlockIntent::Generic, "if_x > 0"),
            None
        );
    }

    #[test]
    fn method_colon_rewrite_requires_a_call_shape() {
        use crate::code_node::CodeNode;

        let mut nodes = vec![
            CodeNode::Literal("object: method()".to_string()),
            CodeNode::Literal("local s = \"label: value\"".to_string()),
            CodeNode::InlineLiteral("table.key: value".to_string()),
        ];
        Lua::rewrite_method_colon(&mut nodes);
        assert!(matches!(&nodes[0], CodeNode::Literal(s) if s == "object:method()"));
        assert!(
            matches!(&nodes[1], CodeNode::Literal(s) if s == "local s = \"label: value\""),
            "string literal content must not be rewritten"
        );
        assert!(
            matches!(&nodes[2], CodeNode::InlineLiteral(s) if s == "table.key: value"),
            "table-like key text must not be rewritten"
        );
    }
}
