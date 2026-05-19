//! Python language implementation.

use crate::import::{ImportEntry, ImportGroup};
use crate::lang::config::{
    BlockSyntaxConfig, EnumAndAnnotationConfig, FunctionSyntaxConfig, GenericSyntaxConfig,
    QuoteStyle, TypeDeclSyntaxConfig, TypePresentationConfig,
};
use crate::lang::{CodeLang, RendererLang};
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};
use crate::type_name::{AssociatedTypeStyle, FunctionPresentation, TypePresentation};

/// Python language implementation.
///
/// Python-specific behaviors:
/// - 4-space indentation (PEP 8 default)
/// - Indent-based blocks (`:` to open, no closing delimiter)
/// - No semicolons
/// - `from module import name` imports with PEP 8 grouping
/// - Visibility by naming convention (no keywords)
/// - `class` keyword for all type kinds
/// - `[T]` generic syntax for type hints (PEP 585+)
/// - Docstrings inside function/class bodies
/// - Decorators via the existing `annotations` mechanism
///
/// # Type hints
///
/// Python uses `[]` for generic type hints. Use [`crate::type_name::TypeName::generic`] for
/// parameterized types:
/// ```text
/// TypeName::generic("list", vec![TypeName::primitive("int")])   // list[int]
/// TypeName::generic("dict", vec![                               // dict[str, int]
///     TypeName::primitive("str"),
///     TypeName::primitive("int"),
/// ])
/// TypeName::generic("Optional", vec![TypeName::primitive("str")]) // Optional[str]
/// ```
///
/// # Decorators
///
/// Use the `annotation()` builder method on `FunSpec` or `TypeSpec`:
/// ```text
/// fb.annotation(CodeBlock::of("@staticmethod", ()).unwrap());
/// tb.annotation(CodeBlock::of("@dataclass", ()).unwrap());
/// ```
#[derive(Debug, Clone)]
pub struct Python {
    /// Indent with this string (default: "    " — 4 spaces per PEP 8).
    pub indent: String,
    /// Quote style for string literals. Python accepts both; `Single` is the
    /// community default (Black defaults to `Double`).
    pub quote_style: QuoteStyle,
    /// File extension (default: "py"). Set to "pyi" for stub files.
    pub extension: String,
}

impl Default for Python {
    fn default() -> Self {
        Self {
            indent: "    ".to_string(),
            quote_style: QuoteStyle::Single,
            extension: "py".to_string(),
        }
    }
}

impl Python {
    /// Create a new Python language instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the quote style used for string literals.
    pub fn with_quote_style(mut self, qs: QuoteStyle) -> Self {
        self.quote_style = qs;
        self
    }

    /// Set the indent string (e.g., `"    "` for 4-space PEP 8, `"  "` for 2-space).
    pub fn with_indent(mut self, s: &str) -> Self {
        self.indent = s.to_string();
        self
    }

    /// Set the file extension (e.g., `"py"` or `"pyi"`).
    pub fn with_extension(mut self, s: &str) -> Self {
        self.extension = s.to_string();
        self
    }
}

const PYTHON_RESERVED: &[&str] = &[
    "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class", "continue",
    "def", "del", "elif", "else", "except", "finally", "for", "from", "global", "if", "import",
    "in", "is", "lambda", "nonlocal", "not", "or", "pass", "raise", "return", "try", "while",
    "with", "yield",
];

/// Common Python stdlib top-level module names.
/// Used to separate stdlib imports from third-party imports (PEP 8).
const PYTHON_STDLIB: &[&str] = &[
    "abc",
    "argparse",
    "ast",
    "asyncio",
    "base64",
    "bisect",
    "builtins",
    "calendar",
    "cmath",
    "collections",
    "concurrent",
    "contextlib",
    "copy",
    "csv",
    "ctypes",
    "dataclasses",
    "datetime",
    "decimal",
    "difflib",
    "email",
    "enum",
    "errno",
    "functools",
    "glob",
    "gzip",
    "hashlib",
    "heapq",
    "hmac",
    "html",
    "http",
    "importlib",
    "inspect",
    "io",
    "itertools",
    "json",
    "logging",
    "math",
    "mimetypes",
    "multiprocessing",
    "operator",
    "os",
    "pathlib",
    "pickle",
    "platform",
    "pprint",
    "queue",
    "random",
    "re",
    "secrets",
    "shutil",
    "signal",
    "socket",
    "sqlite3",
    "ssl",
    "statistics",
    "string",
    "struct",
    "subprocess",
    "sys",
    "tempfile",
    "textwrap",
    "threading",
    "time",
    "timeit",
    "traceback",
    "types",
    "typing",
    "unittest",
    "urllib",
    "uuid",
    "warnings",
    "weakref",
    "xml",
    "zipfile",
    "zlib",
];

/// Returns true if the module path looks like a Python stdlib package.
fn is_stdlib(module: &str) -> bool {
    let top = module.split('.').next().unwrap_or(module);
    PYTHON_STDLIB.contains(&top)
}

impl RendererLang for Python {
    fn file_extension(&self) -> &str {
        &self.extension
    }

    fn reserved_words(&self) -> &[&str] {
        PYTHON_RESERVED
    }

    fn render_string_literal(&self, s: &str) -> String {
        match self.quote_style {
            QuoteStyle::Single => format!(
                "'{}'",
                s.replace('\\', "\\\\")
                    .replace('\'', "\\'")
                    .replace('\n', "\\n")
                    .replace('\t', "\\t")
            ),
            QuoteStyle::Double => format!(
                "\"{}\"",
                s.replace('\\', "\\\\")
                    .replace('"', "\\\"")
                    .replace('\n', "\\n")
                    .replace('\t', "\\t")
            ),
        }
    }

    fn line_comment_prefix(&self) -> &str {
        "#"
    }

    fn block_open_for(&self, condition: &str) -> Option<&str> {
        if condition.trim_end().ends_with(':') {
            Some("")
        } else {
            None
        }
    }

    fn module_separator(&self) -> Option<&str> {
        Some(".")
    }

    // --- Config struct accessors ---

    fn type_presentation(&self) -> TypePresentationConfig<'_> {
        TypePresentationConfig {
            array: TypePresentation::GenericWrap { name: "list" },
            readonly_array: Some(TypePresentation::GenericWrap { name: "list" }),
            optional_absent_literal: "None",
            map: TypePresentation::Delimited {
                open: "dict[",
                sep: ", ",
                close: "]",
            },
            tuple: TypePresentation::GenericWrap { name: "tuple" },
            function: FunctionPresentation {
                keyword: "",
                params_open: "Callable[[",
                params_sep: ", ",
                params_close: "]",
                arrow: ", ",
                return_first: false,
                curried: false,
                wrapper_open: "",
                wrapper_close: "]",
            },
            associated_type: AssociatedTypeStyle::DotAccess,
            ..Default::default()
        }
    }

    fn generic_syntax(&self) -> GenericSyntaxConfig<'_> {
        GenericSyntaxConfig {
            open: "[",
            close: "]",
            constraint_keyword: "",
            constraint_separator: "",
            context_bound_keyword: "",
            ..Default::default()
        }
    }

    fn block_syntax(&self) -> BlockSyntaxConfig<'_> {
        BlockSyntaxConfig {
            block_open: ":",
            block_close: "",
            indent_unit: &self.indent,
            uses_semicolons: false,
            field_terminator: "",
            bases_close: ")",
            ..Default::default()
        }
    }
}

impl CodeLang for Python {
    fn render_imports(&self, imports: &ImportGroup) -> String {
        if imports.entries().is_empty() {
            return String::new();
        }

        let mut lines: Vec<String> = Vec::new();

        // Handle side-effect and wildcard imports first.
        for entry in imports.entries() {
            if entry.is_side_effect {
                lines.push(format!("import {}", entry.module));
            } else if entry.is_wildcard {
                lines.push(format!("from {} import *", entry.module));
            }
        }

        // Group named imports by module, merging names from the same module.
        let mut stdlib: std::collections::BTreeMap<&str, Vec<&ImportEntry>> =
            std::collections::BTreeMap::new();
        let mut thirdparty: std::collections::BTreeMap<&str, Vec<&ImportEntry>> =
            std::collections::BTreeMap::new();

        for entry in imports.entries() {
            if entry.is_side_effect || entry.is_wildcard {
                continue;
            }
            let target = if is_stdlib(&entry.module) {
                &mut stdlib
            } else {
                &mut thirdparty
            };
            target.entry(entry.module.as_str()).or_default().push(entry);
        }

        if !lines.is_empty() && (!stdlib.is_empty() || !thirdparty.is_empty()) {
            lines.push(String::new());
        }

        // Emit stdlib imports.
        for (module, entries) in &stdlib {
            lines.push(render_from_import(module, entries));
        }

        // Blank line between groups.
        if !stdlib.is_empty() && !thirdparty.is_empty() {
            lines.push(String::new());
        }

        // Emit third-party imports.
        for (module, entries) in &thirdparty {
            lines.push(render_from_import(module, entries));
        }

        lines.join("\n")
    }

    fn render_doc_comment(&self, lines: &[&str]) -> String {
        if lines.len() == 1 {
            format!("\"\"\"{}\"\"\"", lines[0])
        } else {
            let mut result = String::from("\"\"\"");
            for line in lines {
                result.push('\n');
                result.push_str(line);
            }
            result.push_str("\n\"\"\"");
            result
        }
    }

    fn render_visibility(&self, _vis: Visibility, _ctx: DeclarationContext) -> &str {
        // Python uses naming conventions (_private, __mangled), not keywords.
        ""
    }

    fn function_keyword(&self, _ctx: DeclarationContext) -> &str {
        "def"
    }

    fn type_keyword(&self, kind: TypeKind) -> &str {
        match kind {
            TypeKind::TypeAlias => "type",
            TypeKind::Newtype => "class",
            _ => "class",
        }
    }

    fn methods_inside_type_body(&self, _kind: TypeKind) -> bool {
        true
    }

    fn render_newtype_line(&self, _vis: &str, name: &str, inner: &str) -> String {
        format!("{name} = NewType(\"{name}\", {inner})")
    }

    fn doc_comment_inside_body(&self) -> bool {
        true
    }

    fn fun_block_open(&self) -> &str {
        ":"
    }

    fn type_header_block_open(&self, _kind: crate::spec::modifiers::TypeKind) -> &str {
        ":"
    }

    fn optional_field_style(&self) -> crate::lang::config::OptionalFieldStyle {
        crate::lang::config::OptionalFieldStyle::UnionWithNone(" | ")
    }

    fn function_syntax(&self) -> FunctionSyntaxConfig<'_> {
        FunctionSyntaxConfig {
            return_type_separator: " -> ",
            constructor_keyword: "def",
            empty_body: "...",
            static_keyword: "",
            ..Default::default()
        }
    }

    fn type_decl_syntax(&self) -> TypeDeclSyntaxConfig<'_> {
        TypeDeclSyntaxConfig {
            super_type_keyword: "(",
            implements_keyword: ", ",
            ..Default::default()
        }
    }

    fn enum_and_annotation(&self) -> EnumAndAnnotationConfig<'_> {
        EnumAndAnnotationConfig {
            variant_separator: "",
            ..Default::default()
        }
    }
}

/// Render a `from module import name1, name2` line.
fn render_from_import(module: &str, entries: &[&ImportEntry]) -> String {
    let mut names: Vec<&str> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for entry in entries {
        let name = entry.alias.as_deref().unwrap_or(&entry.name);
        if seen.insert(name) {
            if let Some(alias) = &entry.alias {
                // Import with alias: `from module import OrigName as Alias`
                // The alias is already the resolved name; original is entry.name.
                names.push(alias);
            } else {
                names.push(&entry.name);
            }
        }
    }
    names.sort();
    format!("from {} import {}", module, names.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_extension() {
        let py = Python::new();
        assert_eq!(py.file_extension(), "py");
    }

    #[test]
    fn test_escape_reserved() {
        let py = Python::new();
        assert_eq!(py.escape_reserved("class"), "class_");
        assert_eq!(py.escape_reserved("name"), "name");
        assert_eq!(py.escape_reserved("import"), "import_");
    }

    #[test]
    fn test_render_imports_single() {
        let py = Python::new();
        let imports = ImportGroup {
            entries: vec![ImportEntry {
                module: "json".into(),
                name: "dumps".into(),
                alias: None,
                is_type_only: false,
                is_side_effect: false,
                is_wildcard: false,
            }],
        };
        assert_eq!(py.render_imports(&imports), "from json import dumps");
    }

    #[test]
    fn test_render_imports_same_module_merged() {
        let py = Python::new();
        let imports = ImportGroup {
            entries: vec![
                ImportEntry {
                    module: "typing".into(),
                    name: "Optional".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "typing".into(),
                    name: "List".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
            ],
        };
        assert_eq!(
            py.render_imports(&imports),
            "from typing import List, Optional"
        );
    }

    #[test]
    fn test_render_imports_grouped() {
        let py = Python::new();
        let imports = ImportGroup {
            entries: vec![
                ImportEntry {
                    module: "json".into(),
                    name: "dumps".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "flask".into(),
                    name: "Flask".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
            ],
        };
        let output = py.render_imports(&imports);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines[0], "from json import dumps");
        assert_eq!(lines[1], "");
        assert_eq!(lines[2], "from flask import Flask");
    }

    #[test]
    fn test_doc_comment_single_line() {
        let py = Python::new();
        assert_eq!(
            py.render_doc_comment(&["A simple docstring."]),
            "\"\"\"A simple docstring.\"\"\""
        );
    }

    #[test]
    fn test_doc_comment_multi_line() {
        let py = Python::new();
        let doc = py.render_doc_comment(&["First line.", "", "Second paragraph."]);
        assert!(doc.starts_with("\"\"\""));
        assert!(doc.ends_with("\"\"\""));
        assert!(doc.contains("First line."));
        assert!(doc.contains("Second paragraph."));
    }

    #[test]
    fn test_string_literal() {
        let py = Python::new();
        assert_eq!(py.render_string_literal("hello"), "'hello'");
        assert_eq!(py.render_string_literal("it's"), "'it\\'s'");
    }

    #[test]
    fn test_block_delimiters() {
        let py = Python::new();
        assert_eq!(py.block_syntax().block_open, ":");
        assert_eq!(py.block_syntax().block_close, "");
    }

    #[test]
    fn test_generic_delimiters() {
        let py = Python::new();
        assert_eq!(py.generic_syntax().open, "[");
        assert_eq!(py.generic_syntax().close, "]");
    }

    #[test]
    fn test_is_stdlib() {
        assert!(is_stdlib("json"));
        assert!(is_stdlib("typing"));
        assert!(is_stdlib("collections"));
        assert!(is_stdlib("os.path"));
        assert!(!is_stdlib("flask"));
        assert!(!is_stdlib("django.db"));
        assert!(!is_stdlib("requests"));
    }

    #[test]
    fn test_doc_inside_body() {
        let py = Python::new();
        assert!(py.doc_comment_inside_body());
    }

    #[test]
    fn test_empty_body() {
        let py = Python::new();
        assert_eq!(py.function_syntax().empty_body, "...");
    }

    #[test]
    fn test_python_builder_fluent() {
        let py = Python::new()
            .with_quote_style(QuoteStyle::Double)
            .with_extension("pyi")
            .with_indent("  ");
        assert_eq!(py.file_extension(), "pyi");
        assert_eq!(py.block_syntax().indent_unit, "  ");
        assert_eq!(py.render_string_literal("hi"), "\"hi\"");
    }

    #[test]
    fn test_module_separator() {
        let py = Python::new();
        assert_eq!(py.module_separator(), Some("."));
    }
}
