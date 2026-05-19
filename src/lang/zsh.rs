//! Zsh shell language implementation.

use crate::import::ImportGroup;
use crate::lang::config::{
    BlockSyntaxConfig, EnumAndAnnotationConfig, FunctionSyntaxConfig, GenericSyntaxConfig,
    TypeDeclSyntaxConfig, TypePresentationConfig,
};
use crate::lang::{CodeLang, RendererLang};
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};

/// Zsh shell language implementation.
///
/// Zsh-specific behaviors:
/// - 4-space indentation (configurable)
/// - No semicolons (newline-separated statements)
/// - `source "path"` imports
/// - `#` comments
/// - Double-quoted string literals with `$`, `` ` ``, `\`, `"`, `!`, `%` escaping
///   (`%` is escaped because Zsh uses it for prompt expansion)
/// - `function` keyword for function declarations
/// - `{ }` brace blocks for functions
///
/// # Differences from Bash
///
/// - File extension: `.zsh` instead of `.sh`
/// - Additional reserved words for Zsh builtins (`autoload`, `compdef`, `zstyle`, etc.)
/// - String literal escaping includes `%` (Zsh prompt expansion character)
///
/// # Control Flow
///
/// Same as Bash: use manual `add()` with `%>`/`%<` for control flow blocks.
/// See [`super::bash::Bash`] for detailed examples.
#[derive(Debug, Clone)]
pub struct Zsh {
    /// Indent with this string (default: "    " -- 4 spaces).
    pub indent: String,
    /// File extension (default: "zsh").
    pub extension: String,
}

impl Default for Zsh {
    fn default() -> Self {
        Self {
            indent: "    ".to_string(),
            extension: "zsh".to_string(),
        }
    }
}

impl Zsh {
    /// Create a new Zsh language instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the indent string (e.g., `"    "` for 4-space default, `"  "` for 2 spaces, `"\t"` for tabs).
    pub fn with_indent(mut self, s: &str) -> Self {
        self.indent = s.to_string();
        self
    }

    /// Set the file extension (default: `"zsh"`).
    pub fn with_extension(mut self, s: &str) -> Self {
        self.extension = s.to_string();
        self
    }
}

const ZSH_RESERVED: &[&str] = &[
    "autoload", "bindkey", "break", "case", "chpwd", "compdef", "continue", "coproc", "declare",
    "do", "done", "elif", "else", "emulate", "esac", "eval", "exec", "exit", "export", "fi", "for",
    "function", "if", "in", "local", "precmd", "preexec", "readonly", "return", "select", "setopt",
    "shift", "source", "then", "time", "trap", "typeset", "unset", "unsetopt", "until", "while",
    "zle", "zmodload", "zshexit", "zstyle",
];

impl RendererLang for Zsh {
    fn file_extension(&self) -> &str {
        &self.extension
    }

    fn reserved_words(&self) -> &[&str] {
        ZSH_RESERVED
    }

    fn render_string_literal(&self, s: &str) -> String {
        // Double-quoted string with Zsh-specific escaping.
        // Must escape: \, ", $, `, !, %
        // The % is escaped because Zsh uses it for prompt expansion.
        let escaped = s
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('$', "\\$")
            .replace('`', "\\`")
            .replace('!', "\\!")
            .replace('%', "%%");
        format!("\"{escaped}\"")
    }

    fn line_comment_prefix(&self) -> &str {
        "#"
    }

    // --- Config struct accessors ---

    fn type_presentation(&self) -> TypePresentationConfig<'_> {
        TypePresentationConfig::default()
    }

    fn generic_syntax(&self) -> GenericSyntaxConfig<'_> {
        GenericSyntaxConfig {
            constraint_keyword: "",
            constraint_separator: "",
            ..Default::default()
        }
    }

    fn block_syntax(&self) -> BlockSyntaxConfig<'_> {
        BlockSyntaxConfig {
            indent_unit: &self.indent,
            uses_semicolons: false,
            field_terminator: "",
            ..Default::default()
        }
    }
}

impl CodeLang for Zsh {
    fn render_imports(&self, imports: &ImportGroup) -> String {
        if imports.entries().is_empty() {
            return String::new();
        }

        // Deduplicate to unique source paths.
        let mut paths: Vec<&str> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for entry in imports.entries() {
            if seen.insert(entry.module.as_str()) {
                paths.push(&entry.module);
            }
        }
        paths.sort();

        paths
            .iter()
            .map(|p| format!("source \"{p}\""))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn render_doc_comment(&self, lines: &[&str]) -> String {
        lines
            .iter()
            .map(|line| {
                if line.is_empty() {
                    "#".to_string()
                } else {
                    format!("# {line}")
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn render_visibility(&self, _vis: Visibility, _ctx: DeclarationContext) -> &str {
        ""
    }

    fn function_keyword(&self, _ctx: DeclarationContext) -> &str {
        "function"
    }

    fn type_keyword(&self, _kind: TypeKind) -> &str {
        ""
    }

    fn methods_inside_type_body(&self, _kind: TypeKind) -> bool {
        true
    }

    fn function_syntax(&self) -> FunctionSyntaxConfig<'_> {
        FunctionSyntaxConfig {
            return_type_separator: "",
            ..Default::default()
        }
    }

    fn type_decl_syntax(&self) -> TypeDeclSyntaxConfig<'_> {
        TypeDeclSyntaxConfig::default()
    }

    fn enum_and_annotation(&self) -> EnumAndAnnotationConfig<'_> {
        EnumAndAnnotationConfig::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_extension() {
        let zsh = Zsh::new();
        assert_eq!(zsh.file_extension(), "zsh");
    }

    #[test]
    fn test_reserved_words() {
        let zsh = Zsh::new();
        let reserved = zsh.reserved_words();
        // Bash-shared words
        assert!(reserved.contains(&"if"));
        assert!(reserved.contains(&"fi"));
        assert!(reserved.contains(&"function"));
        // Zsh-specific words
        assert!(reserved.contains(&"autoload"));
        assert!(reserved.contains(&"compdef"));
        assert!(reserved.contains(&"zstyle"));
        assert!(reserved.contains(&"setopt"));
        assert!(reserved.contains(&"emulate"));
        assert!(!reserved.contains(&"echo"));
    }

    #[test]
    fn test_escape_reserved() {
        let zsh = Zsh::new();
        assert_eq!(zsh.escape_reserved("autoload"), "autoload_");
        assert_eq!(zsh.escape_reserved("name"), "name");
        assert_eq!(zsh.escape_reserved("setopt"), "setopt_");
    }

    #[test]
    fn test_string_literal_basic() {
        let zsh = Zsh::new();
        assert_eq!(zsh.render_string_literal("hello"), "\"hello\"");
    }

    #[test]
    fn test_string_literal_escaping() {
        let zsh = Zsh::new();
        assert_eq!(zsh.render_string_literal("$HOME"), "\"\\$HOME\"");
        assert_eq!(
            zsh.render_string_literal("say \"hi\""),
            "\"say \\\"hi\\\"\""
        );
        assert_eq!(zsh.render_string_literal("`cmd`"), "\"\\`cmd\\`\"");
        assert_eq!(zsh.render_string_literal("a\\b"), "\"a\\\\b\"");
        assert_eq!(zsh.render_string_literal("wow!"), "\"wow\\!\"");
    }

    #[test]
    fn test_string_literal_percent_escaping() {
        let zsh = Zsh::new();
        // Zsh-specific: % is escaped to %% for prompt expansion safety
        assert_eq!(zsh.render_string_literal("100%"), "\"100%%\"");
        assert_eq!(zsh.render_string_literal("%F{red}"), "\"%%F{red}\"");
    }

    #[test]
    fn test_render_imports_empty() {
        let zsh = Zsh::new();
        let imports = ImportGroup::from(vec![]);
        assert_eq!(zsh.render_imports(&imports), "");
    }

    #[test]
    fn test_render_imports_dedup() {
        let zsh = Zsh::new();
        let imports = ImportGroup {
            entries: vec![
                crate::import::ImportEntry {
                    module: "./lib/utils.zsh".into(),
                    name: "log_info".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                crate::import::ImportEntry {
                    module: "./lib/utils.zsh".into(),
                    name: "log_error".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
            ],
        };
        assert_eq!(zsh.render_imports(&imports), "source \"./lib/utils.zsh\"");
    }

    #[test]
    fn test_doc_comment() {
        let zsh = Zsh::new();
        let doc = zsh.render_doc_comment(&["A function.", "", "Details."]);
        let lines: Vec<&str> = doc.lines().collect();
        assert_eq!(lines[0], "# A function.");
        assert_eq!(lines[1], "#");
        assert_eq!(lines[2], "# Details.");
    }

    #[test]
    fn test_no_semicolons() {
        let zsh = Zsh::new();
        assert!(!zsh.block_syntax().uses_semicolons);
    }

    #[test]
    fn test_function_keyword() {
        let zsh = Zsh::new();
        assert_eq!(
            zsh.function_keyword(DeclarationContext::TopLevel),
            "function"
        );
    }

    #[test]
    fn test_zsh_builder_fluent() {
        let zsh = Zsh::new().with_indent("\t").with_extension("sh");
        assert_eq!(zsh.file_extension(), "sh");
        assert_eq!(zsh.block_syntax().indent_unit, "\t");
    }

    #[test]
    fn test_module_separator() {
        let zsh = Zsh::new();
        assert_eq!(zsh.module_separator(), None);
    }
}
