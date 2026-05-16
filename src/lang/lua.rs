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

use crate::import::ImportGroup;
use crate::lang::CodeLang;
use crate::lang::config::{
    BlockSyntaxConfig, EnumAndAnnotationConfig, FunctionSyntaxConfig, GenericSyntaxConfig,
    TypeDeclSyntaxConfig, TypePresentationConfig,
};
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};

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

impl Lua {
    /// Create a new Lua language instance with default settings.
    pub fn new() -> Self {
        Self::default()
    }
}

/// Lua keywords (Lua 5.x).
/// Contextual keyword `self` intentionally not included — treat as regular ident.
const LUA_RESERVED: &[&str] = &[
    "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "goto", "if", "in",
    "local", "nil", "not", "or", "repeat", "return", "then", "true", "until", "while",
];

impl CodeLang for Lua {
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

    fn module_separator(&self) -> Option<&str> {
        Some(".")
    }

    fn escape_reserved(&self, name: &str) -> String {
        // Lua doesn't support `@` or backtick escaping.
        // Append `_` suffix (e.g., `function_` for `function`).
        if LUA_RESERVED.contains(&name) {
            format!("{}_", name)
        } else {
            name.to_string()
        }
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

    fn generic_syntax(&self) -> GenericSyntaxConfig<'_> {
        GenericSyntaxConfig {
            open: "",
            close: "",
            ..Default::default()
        }
    }

    fn enum_and_annotation(&self) -> EnumAndAnnotationConfig<'_> {
        EnumAndAnnotationConfig {
            readonly_keyword: "",
            ..Default::default()
        }
    }

    fn type_presentation(&self) -> TypePresentationConfig<'_> {
        TypePresentationConfig {
            optional_absent_literal: "nil",
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
}
