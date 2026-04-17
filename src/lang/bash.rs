//! Bash shell language implementation.

use crate::import::ImportGroup;
use crate::lang::CodeLang;
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};

/// Bash shell language implementation.
///
/// Bash-specific behaviors:
/// - 4-space indentation (configurable)
/// - No semicolons (newline-separated statements)
/// - `source "path"` imports
/// - `#` comments
/// - Double-quoted string literals with `$`, `` ` ``, `\`, `"`, `!` escaping
/// - `function` keyword for function declarations
/// - `{ }` brace blocks for functions
///
/// # Control Flow
///
/// Bash uses keyword-based block closers that vary per construct (`fi`, `done`,
/// `esac`). The `begin_control_flow`/`end_control_flow` helpers auto-append
/// `block_open()`/`block_close()` (i.e., `{`/`}`), which is wrong for shell
/// control flow. Instead, use manual `add()` with explicit indent/dedent:
///
/// ```text
/// // if/then/fi
/// b.add("if [ -f \"$file\" ]; then\n", ());
/// b.add("%>", ());
/// b.add_statement("echo \"found\"", ());
/// b.add("%<", ());
/// b.add("fi\n", ());
///
/// // for/do/done
/// b.add("for f in *.txt; do\n", ());
/// b.add("%>", ());
/// b.add_statement("process \"$f\"", ());
/// b.add("%<", ());
/// b.add("done\n", ());
///
/// // while/do/done
/// b.add("while read -r line; do\n", ());
/// b.add("%>", ());
/// b.add_statement("echo \"$line\"", ());
/// b.add("%<", ());
/// b.add("done\n", ());
///
/// // case/esac
/// b.add("case \"$1\" in\n", ());
/// b.add("%>", ());
/// b.add("start)\n", ());
/// b.add("%>", ());
/// b.add_statement("start_service", ());
/// b.add("%<", ());
/// b.add(";;\n", ());
/// b.add("*)\n", ());
/// b.add("%>", ());
/// b.add_statement("echo \"unknown\"", ());
/// b.add("%<", ());
/// b.add(";;\n", ());
/// b.add("%<", ());
/// b.add("esac\n", ());
/// ```
///
/// The `begin_control_flow`/`end_control_flow` helpers **do** work for function
/// bodies since functions use `{ }` braces:
///
/// ```text
/// let mut fb = FunSpec::<Bash>::builder("greet");
/// fb.body(body);
/// let fun = fb.build().unwrap();
/// // function greet {
/// //     echo "hello"
/// // }
/// ```
///
/// # Shebang
///
/// Use `FileSpec::header()` for the shebang line:
///
/// ```text
/// let mut header_b = CodeBlock::<Bash>::builder();
/// header_b.add("#!/usr/bin/env bash\n", ());
/// header_b.add("set -euo pipefail", ());
/// fb.header(header_b.build().unwrap());
/// ```
#[derive(Debug, Clone)]
pub struct Bash {
    /// Indent with this string (default: "    " -- 4 spaces).
    pub indent: String,
}

impl Default for Bash {
    fn default() -> Self {
        Self {
            indent: "    ".to_string(),
        }
    }
}

impl Bash {
    /// Create a new Bash language instance.
    pub fn new() -> Self {
        Self::default()
    }
}

const BASH_RESERVED: &[&str] = &[
    "break", "case", "continue", "coproc", "declare", "do", "done", "elif", "else", "esac", "eval",
    "exec", "exit", "export", "fi", "for", "function", "if", "in", "local", "readonly", "return",
    "select", "shift", "source", "then", "time", "trap", "typeset", "unset", "until", "while",
];

impl CodeLang for Bash {
    fn file_extension(&self) -> &str {
        "bash"
    }

    fn reserved_words(&self) -> &[&str] {
        BASH_RESERVED
    }

    fn render_imports(&self, imports: &ImportGroup) -> String {
        if imports.entries.is_empty() {
            return String::new();
        }

        // Deduplicate to unique source paths.
        let mut paths: Vec<&str> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for entry in &imports.entries {
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

    fn render_string_literal(&self, s: &str) -> String {
        // Double-quoted string with Bash-specific escaping.
        // Must escape: \, ", $, `, !
        let escaped = s
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('$', "\\$")
            .replace('`', "\\`")
            .replace('!', "\\!");
        format!("\"{escaped}\"")
    }

    fn render_doc_comment(&self, lines: &[&str]) -> String {
        // Bash has no doc comment convention; use # comment blocks.
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

    fn line_comment_prefix(&self) -> &str {
        "#"
    }

    fn indent_unit(&self) -> &str {
        &self.indent
    }

    fn uses_semicolons(&self) -> bool {
        false
    }

    // --- Spec support ---
    // Shell has no visibility, generics, inheritance, or interfaces.
    // Return empty/no-op for all structural methods.

    fn render_visibility(&self, _vis: Visibility, _ctx: DeclarationContext) -> &str {
        ""
    }

    fn function_keyword(&self, _ctx: DeclarationContext) -> &str {
        "function"
    }

    fn return_type_separator(&self) -> &str {
        ""
    }

    fn type_keyword(&self, _kind: TypeKind) -> &str {
        ""
    }

    fn field_terminator(&self) -> &str {
        ""
    }

    fn methods_inside_type_body(&self, _kind: TypeKind) -> bool {
        true
    }

    fn generic_constraint_keyword(&self) -> &str {
        ""
    }

    fn generic_constraint_separator(&self) -> &str {
        ""
    }

    fn super_type_keyword(&self) -> &str {
        ""
    }

    fn implements_keyword(&self) -> &str {
        ""
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_extension() {
        let bash = Bash::new();
        assert_eq!(bash.file_extension(), "bash");
    }

    #[test]
    fn test_reserved_words() {
        let bash = Bash::new();
        let reserved = bash.reserved_words();
        assert!(reserved.contains(&"if"));
        assert!(reserved.contains(&"fi"));
        assert!(reserved.contains(&"function"));
        assert!(reserved.contains(&"esac"));
        assert!(!reserved.contains(&"echo"));
    }

    #[test]
    fn test_escape_reserved() {
        let bash = Bash::new();
        assert_eq!(bash.escape_reserved("if"), "if_");
        assert_eq!(bash.escape_reserved("name"), "name");
        assert_eq!(bash.escape_reserved("function"), "function_");
    }

    #[test]
    fn test_string_literal_basic() {
        let bash = Bash::new();
        assert_eq!(bash.render_string_literal("hello"), "\"hello\"");
    }

    #[test]
    fn test_string_literal_escaping() {
        let bash = Bash::new();
        // Dollar sign
        assert_eq!(bash.render_string_literal("$HOME"), "\"\\$HOME\"");
        // Double quote
        assert_eq!(
            bash.render_string_literal("say \"hi\""),
            "\"say \\\"hi\\\"\""
        );
        // Backtick
        assert_eq!(bash.render_string_literal("`cmd`"), "\"\\`cmd\\`\"");
        // Backslash
        assert_eq!(bash.render_string_literal("a\\b"), "\"a\\\\b\"");
        // Exclamation
        assert_eq!(bash.render_string_literal("wow!"), "\"wow\\!\"");
    }

    #[test]
    fn test_string_literal_combined() {
        let bash = Bash::new();
        assert_eq!(
            bash.render_string_literal("$USER says \"hi!\""),
            "\"\\$USER says \\\"hi\\!\\\"\"",
        );
    }

    #[test]
    fn test_render_imports_empty() {
        let bash = Bash::new();
        let imports = ImportGroup { entries: vec![] };
        assert_eq!(bash.render_imports(&imports), "");
    }

    #[test]
    fn test_render_imports_single() {
        let bash = Bash::new();
        let imports = ImportGroup {
            entries: vec![crate::import::ImportEntry {
                module: "./lib/utils.sh".into(),
                name: "log_info".into(),
                alias: None,
                is_type_only: false,
                is_side_effect: false,
                is_wildcard: false,
            }],
        };
        assert_eq!(bash.render_imports(&imports), "source \"./lib/utils.sh\"");
    }

    #[test]
    fn test_render_imports_dedup() {
        let bash = Bash::new();
        let imports = ImportGroup {
            entries: vec![
                crate::import::ImportEntry {
                    module: "./lib/utils.sh".into(),
                    name: "log_info".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                crate::import::ImportEntry {
                    module: "./lib/utils.sh".into(),
                    name: "log_error".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                crate::import::ImportEntry {
                    module: "./lib/config.sh".into(),
                    name: "load_config".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
            ],
        };
        let output = bash.render_imports(&imports);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "source \"./lib/config.sh\"");
        assert_eq!(lines[1], "source \"./lib/utils.sh\"");
    }

    #[test]
    fn test_doc_comment_single() {
        let bash = Bash::new();
        assert_eq!(bash.render_doc_comment(&["A function."]), "# A function.");
    }

    #[test]
    fn test_doc_comment_multi() {
        let bash = Bash::new();
        let doc = bash.render_doc_comment(&["First line.", "", "Second paragraph."]);
        let lines: Vec<&str> = doc.lines().collect();
        assert_eq!(lines[0], "# First line.");
        assert_eq!(lines[1], "#");
        assert_eq!(lines[2], "# Second paragraph.");
    }

    #[test]
    fn test_no_semicolons() {
        let bash = Bash::new();
        assert!(!bash.uses_semicolons());
    }

    #[test]
    fn test_comment_prefix() {
        let bash = Bash::new();
        assert_eq!(bash.line_comment_prefix(), "#");
    }

    #[test]
    fn test_function_keyword() {
        let bash = Bash::new();
        assert_eq!(
            bash.function_keyword(DeclarationContext::TopLevel),
            "function"
        );
    }

    #[test]
    fn test_block_delimiters() {
        let bash = Bash::new();
        assert_eq!(bash.block_open(), " {");
        assert_eq!(bash.block_close(), "}");
    }
}
