//! Go language implementation.

use crate::code_node::CodeNode;
use crate::import::{ImportEntry, ImportGroup};
use crate::lang::CodeLang;
use crate::lang::config::{
    BlockSyntaxConfig, EnumAndAnnotationConfig, FunctionSyntaxConfig, GenericSyntaxConfig,
    TypeDeclSyntaxConfig, TypePresentationConfig,
};
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};
use crate::type_name::{FunctionPresentation, TypePresentation, WildcardPresentation};

/// Go language implementation.
///
/// Go-specific behaviors:
/// - Tab indentation (default)
/// - No semicolons
/// - Package-level imports with qualified references (`http.Server`)
/// - Visibility by name capitalization (no keywords)
/// - `type Name struct` / `type Name interface` declaration syntax
/// - `[T constraint]` generic syntax (Go 1.18+)
/// - Receiver methods: `func (r *Type) Method()`
///
/// # Multiple return values
///
/// Go functions commonly return `(T, error)`. Use [`crate::type_name::TypeName::raw`] for this:
/// ```text
/// fb.returns(TypeName::raw("(int, error)"));
/// ```
#[derive(Debug, Clone)]
pub struct GoLang {
    /// Indent with this string (default: "\t").
    pub indent: String,
    /// File extension (default: "go").
    pub extension: String,
}

impl Default for GoLang {
    fn default() -> Self {
        Self {
            indent: "\t".to_string(),
            extension: "go".to_string(),
        }
    }
}

impl GoLang {
    /// Create a new Go language instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the indent string (e.g., `"\t"` for gofmt-style tabs, `"    "` for 4 spaces).
    pub fn with_indent(mut self, s: &str) -> Self {
        self.indent = s.to_string();
        self
    }

    /// Set the file extension (default: `"go"`).
    pub fn with_extension(mut self, s: &str) -> Self {
        self.extension = s.to_string();
        self
    }
}

const GO_RESERVED: &[&str] = &[
    "break",
    "case",
    "chan",
    "const",
    "continue",
    "default",
    "defer",
    "else",
    "fallthrough",
    "for",
    "func",
    "go",
    "goto",
    "if",
    "import",
    "interface",
    "map",
    "package",
    "range",
    "return",
    "select",
    "struct",
    "switch",
    "type",
    "var",
];

/// Extract the package name from a Go import path.
///
/// `"net/http"` → `"http"`, `"encoding/json"` → `"json"`, `"fmt"` → `"fmt"`.
fn package_name(module: &str) -> &str {
    module.rsplit('/').next().unwrap_or(module)
}

/// Returns true if the module path looks like a standard library package.
///
/// Go stdlib packages don't contain a dot in the first path segment
/// (e.g., `"fmt"`, `"net/http"` vs `"github.com/foo/bar"`).
fn is_stdlib(module: &str) -> bool {
    let first_segment = module.split('/').next().unwrap_or(module);
    !first_segment.contains('.')
}

impl GoLang {
    /// Rewrite IIFE continuation: fuse `}` + `()` onto the same line.
    ///
    /// The pattern `go func() { ... }();` produces nodes:
    ///   BlockClose("go func()"), StatementBegin, Literal("(...)"), StatementEnd, Newline
    ///
    /// We fuse these into:
    ///   Literal("}"), Literal("(...)"), Newline
    fn rewrite_iife(nodes: &mut Vec<CodeNode>) {
        let mut i = 0;
        while i < nodes.len() {
            let is_func_block_close =
                matches!(&nodes[i], CodeNode::BlockClose(cond) if cond.contains("func"));
            if !is_func_block_close {
                i += 1;
                continue;
            }

            // Check if next nodes are: StatementBegin, Literal(...), StatementEnd, Newline
            // The call arguments might span multiple literal nodes.
            let remaining = nodes.len() - i - 1;
            if remaining >= 3 && matches!(&nodes[i + 1], CodeNode::StatementBegin) {
                // Find StatementEnd
                let end_idx = nodes[(i + 2)..]
                    .iter()
                    .position(|n| matches!(n, CodeNode::StatementEnd))
                    .map(|pos| pos + i + 2);

                if let Some(stmt_end) = end_idx {
                    // Collect the call literal nodes between StatementBegin and StatementEnd
                    let call_nodes: Vec<CodeNode> = nodes[(i + 2)..stmt_end].to_vec();

                    // Check that this looks like a call (starts with "(")
                    let looks_like_call = call_nodes
                        .iter()
                        .any(|n| matches!(n, CodeNode::Literal(s) if s.starts_with('(')));

                    if looks_like_call {
                        // Determine how many nodes to remove after BlockClose
                        // Remove: StatementBegin, ...call_nodes..., StatementEnd
                        // Optionally also remove the following Newline (BlockClose already emitted one)
                        let mut remove_end = stmt_end + 1; // exclusive
                        if remove_end < nodes.len()
                            && matches!(&nodes[remove_end], CodeNode::Newline)
                        {
                            remove_end += 1;
                        }

                        // Replace BlockClose with Literal("}") + call_nodes + Newline
                        let mut replacement = Vec::with_capacity(2 + call_nodes.len());
                        replacement.push(CodeNode::Literal("}".to_string()));
                        replacement.extend(call_nodes);
                        replacement.push(CodeNode::Newline);

                        nodes.splice(i..remove_end, replacement);
                        continue;
                    }
                }
            }

            i += 1;
        }
    }

    /// Rewrite `<- ` to `<-` when used as prefix receive operator.
    /// In Go, `<-ch` receives from channel. The tokenizer produces `<- ch` because
    /// `<` and `-` are separate puncts.
    #[allow(clippy::ptr_arg)]
    fn rewrite_receive_op(nodes: &mut Vec<CodeNode>) {
        for node in nodes.iter_mut() {
            if let CodeNode::Literal(s) | CodeNode::InlineLiteral(s) = node {
                let replacements: &[(&str, &str)] = &[
                    (":= <- ", ":= <-"),
                    ("= <- ", "= <-"),
                    ("return <- ", "return <-"),
                ];
                for &(pat, fixed) in replacements {
                    *s = s.replace(pat, fixed);
                }
                if s.starts_with("<- ") {
                    *s = format!("<-{}", &s[3..]);
                }
            }
        }
    }
}

impl CodeLang for GoLang {
    fn file_extension(&self) -> &str {
        &self.extension
    }

    fn reserved_words(&self) -> &[&str] {
        GO_RESERVED
    }

    fn render_imports(&self, imports: &ImportGroup) -> String {
        if imports.entries().is_empty() {
            return String::new();
        }

        // Deduplicate to package-level: Go imports entire packages, not symbols.
        let mut seen = std::collections::BTreeSet::new();
        let mut std_packages: Vec<&ImportEntry> = Vec::new();
        let mut ext_packages: Vec<&ImportEntry> = Vec::new();

        for entry in imports.entries() {
            if seen.contains(&entry.module) {
                continue;
            }
            seen.insert(&entry.module);
            if is_stdlib(&entry.module) {
                std_packages.push(entry);
            } else {
                ext_packages.push(entry);
            }
        }

        // Sort within each group.
        std_packages.sort_by_key(|e| &e.module);
        ext_packages.sort_by_key(|e| &e.module);

        /// Render a single Go import line with prefix handling.
        fn render_go_import(entry: &ImportEntry) -> String {
            let prefix = if entry.is_side_effect {
                "_ "
            } else if entry.is_wildcard {
                ". "
            } else if let Some(alias) = &entry.alias {
                return format!("{alias} \"{}\"", entry.module);
            } else {
                ""
            };
            format!("{prefix}\"{}\"", entry.module)
        }

        let all_packages: Vec<&ImportEntry> = std_packages
            .iter()
            .copied()
            .chain(ext_packages.iter().copied())
            .collect();

        let total = all_packages.len();
        let has_both_groups = !std_packages.is_empty() && !ext_packages.is_empty();

        if total == 1 {
            format!("import {}", render_go_import(all_packages[0]))
        } else {
            let mut lines = Vec::new();
            lines.push("import (".to_string());
            for entry in &std_packages {
                lines.push(format!("\t{}", render_go_import(entry)));
            }
            if has_both_groups {
                lines.push(String::new());
            }
            for entry in &ext_packages {
                lines.push(format!("\t{}", render_go_import(entry)));
            }
            lines.push(")".to_string());
            lines.join("\n")
        }
    }

    fn render_string_literal(&self, s: &str) -> String {
        format!(
            "\"{}\"",
            s.replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', "\\n")
                .replace('\t', "\\t")
        )
    }

    fn render_doc_comment(&self, lines: &[&str]) -> String {
        lines
            .iter()
            .map(|line| {
                if line.is_empty() {
                    "//".to_string()
                } else {
                    format!("// {line}")
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn line_comment_prefix(&self) -> &str {
        "//"
    }

    fn render_visibility(&self, _vis: Visibility, _ctx: DeclarationContext) -> &str {
        // Go uses name capitalization for visibility, not keywords.
        ""
    }

    fn function_keyword(&self, ctx: DeclarationContext) -> &str {
        match ctx {
            // Top-level functions and receiver methods use `func`.
            DeclarationContext::TopLevel => "func",
            // Interface method signatures omit `func`.
            DeclarationContext::Member | DeclarationContext::InterfaceMember => "",
        }
    }

    fn type_keyword(&self, _kind: TypeKind) -> &str {
        // Go uses `type Name struct/interface`, so the prefix keyword is always `type`.
        // The kind-specific keyword (struct/interface) is handled by `type_kind_suffix`.
        "type"
    }

    fn methods_inside_type_body(&self, kind: TypeKind) -> bool {
        match kind {
            TypeKind::Interface | TypeKind::Trait => true,
            TypeKind::Struct
            | TypeKind::Class
            | TypeKind::Enum
            | TypeKind::TypeAlias
            | TypeKind::Newtype => false,
        }
    }

    fn render_newtype_line(&self, _vis: &str, name: &str, inner: &str) -> String {
        format!("type {name} {inner}")
    }

    fn qualify_import_name(&self, module: &str, resolved_name: &str) -> String {
        let pkg = package_name(module);
        format!("{pkg}.{resolved_name}")
    }

    fn type_kind_suffix(&self, kind: TypeKind) -> &str {
        match kind {
            TypeKind::Struct | TypeKind::Class => "struct",
            TypeKind::Interface | TypeKind::Trait => "interface",
            TypeKind::Enum | TypeKind::TypeAlias | TypeKind::Newtype => "",
        }
    }

    fn optional_field_style(&self) -> crate::lang::config::OptionalFieldStyle {
        crate::lang::config::OptionalFieldStyle::TypePrefix("*")
    }

    fn module_separator(&self) -> Option<&str> {
        Some(".")
    }

    // --- Config struct accessors ---

    fn type_presentation(&self) -> TypePresentationConfig<'_> {
        TypePresentationConfig {
            array: TypePresentation::Prefix { prefix: "[]" },
            readonly_array: Some(TypePresentation::Prefix { prefix: "[]" }),
            optional: TypePresentation::Prefix { prefix: "*" },
            map: TypePresentation::Delimited {
                open: "map[",
                sep: "]",
                close: "",
            },
            pointer: TypePresentation::Prefix { prefix: "*" },
            slice: TypePresentation::Prefix { prefix: "[]" },
            reference_mut: TypePresentation::Prefix { prefix: "*" },
            function: FunctionPresentation {
                keyword: "func",
                params_open: "(",
                params_sep: ", ",
                params_close: ")",
                arrow: " ",
                return_first: false,
                curried: false,
                wrapper_open: "",
                wrapper_close: "",
            },
            wildcard: WildcardPresentation {
                unbounded: "any",
                upper_keyword: "any ",
                lower_keyword: "any ",
            },
            ..Default::default()
        }
    }

    fn generic_syntax(&self) -> GenericSyntaxConfig<'_> {
        GenericSyntaxConfig {
            open: "[",
            close: "]",
            constraint_keyword: " ",
            constraint_separator: " ",
            context_bound_keyword: " ",
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

    fn function_syntax(&self) -> FunctionSyntaxConfig<'_> {
        FunctionSyntaxConfig {
            return_type_separator: " ",
            ..Default::default()
        }
    }

    fn type_decl_syntax(&self) -> TypeDeclSyntaxConfig<'_> {
        TypeDeclSyntaxConfig {
            type_annotation_separator: " ",
            ..Default::default()
        }
    }

    fn enum_and_annotation(&self) -> EnumAndAnnotationConfig<'_> {
        EnumAndAnnotationConfig {
            variant_separator: "",
            ..Default::default()
        }
    }

    fn rewrite_nodes(&self, nodes: &mut Vec<CodeNode>) {
        crate::lang::rewrite::walk_nodes_mut(nodes, &Self::rewrite_iife);
        crate::lang::rewrite::walk_nodes_mut(nodes, &Self::rewrite_receive_op);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_extension() {
        let go = GoLang::new();
        assert_eq!(go.file_extension(), "go");
    }

    #[test]
    fn test_escape_reserved() {
        let go = GoLang::new();
        assert_eq!(go.escape_reserved("type"), "type_");
        assert_eq!(go.escape_reserved("name"), "name");
        assert_eq!(go.escape_reserved("func"), "func_");
    }

    #[test]
    fn test_render_imports_single() {
        let go = GoLang::new();
        let imports = ImportGroup {
            entries: vec![ImportEntry {
                module: "fmt".into(),
                name: "Println".into(),
                alias: None,
                is_type_only: false,
                is_side_effect: false,
                is_wildcard: false,
            }],
        };
        assert_eq!(go.render_imports(&imports), "import \"fmt\"");
    }

    #[test]
    fn test_render_imports_multiple_grouped() {
        let go = GoLang::new();
        let imports = ImportGroup {
            entries: vec![
                ImportEntry {
                    module: "fmt".into(),
                    name: "Println".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "net/http".into(),
                    name: "Server".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "github.com/gin-gonic/gin".into(),
                    name: "Context".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
            ],
        };
        let output = go.render_imports(&imports);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines[0], "import (");
        assert_eq!(lines[1], "\t\"fmt\"");
        assert_eq!(lines[2], "\t\"net/http\"");
        // blank line between stdlib and external
        assert_eq!(lines[3], "");
        assert_eq!(lines[4], "\t\"github.com/gin-gonic/gin\"");
        assert_eq!(lines[5], ")");
    }

    #[test]
    fn test_render_imports_dedup_same_package() {
        let go = GoLang::new();
        let imports = ImportGroup {
            entries: vec![
                ImportEntry {
                    module: "net/http".into(),
                    name: "Server".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "net/http".into(),
                    name: "Handler".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
            ],
        };
        let output = go.render_imports(&imports);
        // Should emit only one import for the package.
        assert_eq!(output, "import \"net/http\"");
    }

    #[test]
    fn test_qualify_import_name() {
        let go = GoLang::new();
        assert_eq!(go.qualify_import_name("net/http", "Server"), "http.Server");
        assert_eq!(go.qualify_import_name("fmt", "Println"), "fmt.Println");
        assert_eq!(
            go.qualify_import_name("encoding/json", "Marshal"),
            "json.Marshal"
        );
    }

    #[test]
    fn test_doc_comment() {
        let go = GoLang::new();
        let doc = go.render_doc_comment(&["Config holds configuration.", "", "It is thread-safe."]);
        assert!(doc.contains("// Config holds configuration."));
        assert!(doc.contains("//\n"));
        assert!(doc.contains("// It is thread-safe."));
    }

    #[test]
    fn test_string_literal() {
        let go = GoLang::new();
        assert_eq!(go.render_string_literal("hello"), "\"hello\"");
        assert_eq!(go.render_string_literal("it\"s"), "\"it\\\"s\"");
    }

    #[test]
    fn test_package_name_extraction() {
        assert_eq!(package_name("net/http"), "http");
        assert_eq!(package_name("fmt"), "fmt");
        assert_eq!(package_name("encoding/json"), "json");
        assert_eq!(package_name("github.com/foo/bar"), "bar");
    }

    #[test]
    fn test_is_stdlib() {
        assert!(is_stdlib("fmt"));
        assert!(is_stdlib("net/http"));
        assert!(is_stdlib("encoding/json"));
        assert!(!is_stdlib("github.com/foo/bar"));
        assert!(!is_stdlib("golang.org/x/text"));
    }

    #[test]
    fn test_generic_delimiters() {
        let go = GoLang::new();
        assert_eq!(go.generic_syntax().open, "[");
        assert_eq!(go.generic_syntax().close, "]");
    }

    #[test]
    fn test_type_kind_suffix() {
        let go = GoLang::new();
        assert_eq!(go.type_kind_suffix(TypeKind::Struct), "struct");
        assert_eq!(go.type_kind_suffix(TypeKind::Interface), "interface");
        assert_eq!(go.type_kind_suffix(TypeKind::Enum), "");
    }

    #[test]
    fn test_go_builder_fluent() {
        let go = GoLang::new().with_indent("    ").with_extension("go2");
        assert_eq!(go.file_extension(), "go2");
        assert_eq!(go.block_syntax().indent_unit, "    ");
    }

    #[test]
    fn test_module_separator() {
        let go = GoLang::new();
        assert_eq!(go.module_separator(), Some("."));
    }
}
