//! Python language implementation.

use crate::import::{ImportEntry, ImportGroup};
use crate::lang::CodeLang;
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};

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
/// ```ignore
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
/// ```ignore
/// fb.annotation(CodeBlock::of("@staticmethod", ()).unwrap());
/// tb.annotation(CodeBlock::of("@dataclass", ()).unwrap());
/// ```
#[derive(Debug, Clone)]
pub struct Python {
    /// Indent with this string (default: "    " — 4 spaces per PEP 8).
    pub indent: String,
}

impl Default for Python {
    fn default() -> Self {
        Self {
            indent: "    ".to_string(),
        }
    }
}

impl Python {
    /// Create a new Python language instance.
    pub fn new() -> Self {
        Self::default()
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

impl CodeLang for Python {
    fn file_extension(&self) -> &str {
        "py"
    }

    fn reserved_words(&self) -> &[&str] {
        PYTHON_RESERVED
    }

    fn render_imports(&self, imports: &ImportGroup) -> String {
        if imports.entries.is_empty() {
            return String::new();
        }

        let mut lines: Vec<String> = Vec::new();

        // Handle side-effect and wildcard imports first.
        for entry in &imports.entries {
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

        for entry in &imports.entries {
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

    fn render_string_literal(&self, s: &str) -> String {
        // Python prefers single quotes by convention (both are valid).
        format!(
            "'{}'",
            s.replace('\\', "\\\\")
                .replace('\'', "\\'")
                .replace('\n', "\\n")
                .replace('\t', "\\t")
        )
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

    fn line_comment_prefix(&self) -> &str {
        "#"
    }

    fn indent_unit(&self) -> &str {
        &self.indent
    }

    fn uses_semicolons(&self) -> bool {
        false
    }

    fn render_visibility(&self, _vis: Visibility, _ctx: DeclarationContext) -> &str {
        // Python uses naming conventions (_private, __mangled), not keywords.
        ""
    }

    fn function_keyword(&self, _ctx: DeclarationContext) -> &str {
        "def"
    }

    fn return_type_separator(&self) -> &str {
        " -> "
    }

    fn type_keyword(&self, _kind: TypeKind) -> &str {
        "class"
    }

    fn field_terminator(&self) -> &str {
        ""
    }

    fn methods_inside_type_body(&self, _kind: TypeKind) -> bool {
        // Python always has methods inside the class body.
        true
    }

    fn generic_constraint_keyword(&self) -> &str {
        // Python uses TypeVar("T", bound=X) — not inline syntax.
        ""
    }

    fn generic_constraint_separator(&self) -> &str {
        ""
    }

    fn super_type_keyword(&self) -> &str {
        // Python uses parenthesized bases: `class Foo(Base):`
        "("
    }

    fn implements_keyword(&self) -> &str {
        // Both super_types and impl_types go in the same parenthesized list.
        ", "
    }

    fn generic_open(&self) -> &str {
        "["
    }

    fn generic_close(&self) -> &str {
        "]"
    }

    fn block_open(&self) -> &str {
        ":"
    }

    fn block_close(&self) -> &str {
        ""
    }

    fn doc_comment_inside_body(&self) -> bool {
        true
    }

    fn bases_close(&self) -> &str {
        ")"
    }

    fn empty_body(&self) -> &str {
        "..."
    }

    fn enum_variant_separator(&self) -> &str {
        ""
    }

    fn constructor_keyword(&self) -> &str {
        "def"
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
        assert_eq!(py.block_open(), ":");
        assert_eq!(py.block_close(), "");
    }

    #[test]
    fn test_generic_delimiters() {
        let py = Python::new();
        assert_eq!(py.generic_open(), "[");
        assert_eq!(py.generic_close(), "]");
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
        assert_eq!(py.empty_body(), "...");
    }
}
