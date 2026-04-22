//! C language implementation.

use crate::import::{ImportEntry, ImportGroup};
use crate::lang::CodeLang;
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};

/// C language implementation.
///
/// C-specific behaviors:
/// - Type-before-name declarations (`int count`, not `count: int`)
/// - Return type as prefix (`int add(int a, int b)`)
/// - `#include` directives (system `<>` vs local `""`)
/// - Semicolons after statements and after struct/enum closing braces
/// - No function keyword (no `fn`/`func`/`def`)
/// - No visibility keywords, generics, or inheritance
/// - `/* ... */` doc comments
///
/// # Import conventions
///
/// Use [`crate::type_name::TypeName::importable`] with the header path as the module:
/// ```text
/// TypeName::importable("stdio.h", "printf")    // #include <stdio.h>
/// TypeName::importable("./config.h", "Config")  // #include "config.h"
/// ```
///
/// System headers (no `./` or `../` prefix) get `#include <...>`.
/// Local headers (starting with `./` or `../`) get `#include "..."`.
///
/// # Header guards
///
/// Use `FileSpec::header` for `#pragma once` or include guards:
/// ```text
/// fb.header(CodeBlock::<CLang>::of("#pragma once", ()).unwrap());
/// ```
#[derive(Debug, Clone)]
pub struct CLang {
    /// Indent with this string (default: "    " — 4 spaces).
    pub indent: String,
    /// File extension (default: "c"). Set to "h" for header files.
    pub extension: String,
}

impl Default for CLang {
    fn default() -> Self {
        Self {
            indent: "    ".to_string(),
            extension: "c".to_string(),
        }
    }
}

impl CLang {
    /// Create a new C language instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a CLang configured for header files (.h extension).
    pub fn header() -> Self {
        Self {
            indent: "    ".to_string(),
            extension: "h".to_string(),
        }
    }

    /// Set the indent string (e.g., `"    "` for 4-space default, `"\t"` for tabs).
    pub fn with_indent(mut self, s: &str) -> Self {
        self.indent = s.to_string();
        self
    }

    /// Set the file extension (e.g., `"c"` or `"h"`).
    pub fn with_extension(mut self, s: &str) -> Self {
        self.extension = s.to_string();
        self
    }
}

const C_RESERVED: &[&str] = &[
    "auto",
    "break",
    "case",
    "char",
    "const",
    "continue",
    "default",
    "do",
    "double",
    "else",
    "enum",
    "extern",
    "float",
    "for",
    "goto",
    "if",
    "inline",
    "int",
    "long",
    "register",
    "restrict",
    "return",
    "short",
    "signed",
    "sizeof",
    "static",
    "struct",
    "switch",
    "typedef",
    "union",
    "unsigned",
    "void",
    "volatile",
    "while",
    // C11 additions
    "_Alignas",
    "_Alignof",
    "_Atomic",
    "_Bool",
    "_Complex",
    "_Generic",
    "_Imaginary",
    "_Noreturn",
    "_Static_assert",
    "_Thread_local",
];

/// Returns true if the header path looks like a system header (no `./` or `../` prefix).
fn is_system_header(module: &str) -> bool {
    !module.starts_with("./") && !module.starts_with("../")
}

/// Strip leading `./` from local header paths for the `#include` directive.
fn strip_local_prefix(module: &str) -> &str {
    module.strip_prefix("./").unwrap_or(module)
}

impl CodeLang for CLang {
    fn file_extension(&self) -> &str {
        &self.extension
    }

    fn reserved_words(&self) -> &[&str] {
        C_RESERVED
    }

    fn render_imports(&self, imports: &ImportGroup) -> String {
        if imports.entries.is_empty() {
            return String::new();
        }

        // Deduplicate to header-level: C includes entire headers, not symbols.
        let mut seen = std::collections::BTreeSet::new();
        let mut system_headers: Vec<&ImportEntry> = Vec::new();
        let mut local_headers: Vec<&ImportEntry> = Vec::new();

        for entry in &imports.entries {
            if seen.contains(&entry.module) {
                continue;
            }
            seen.insert(&entry.module);
            if is_system_header(&entry.module) {
                system_headers.push(entry);
            } else {
                local_headers.push(entry);
            }
        }

        system_headers.sort_by_key(|e| &e.module);
        local_headers.sort_by_key(|e| &e.module);

        let mut lines: Vec<String> = Vec::new();

        for entry in &system_headers {
            lines.push(format!("#include <{}>", entry.module));
        }

        if !system_headers.is_empty() && !local_headers.is_empty() {
            lines.push(String::new());
        }

        for entry in &local_headers {
            lines.push(format!(
                "#include \"{}\"",
                strip_local_prefix(&entry.module)
            ));
        }

        lines.join("\n")
    }

    fn render_string_literal(&self, s: &str) -> String {
        format!(
            "\"{}\"",
            s.replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', "\\n")
                .replace('\t', "\\t")
                .replace('\r', "\\r")
                .replace('\0', "\\0")
        )
    }

    fn render_doc_comment(&self, lines: &[&str]) -> String {
        if lines.len() == 1 {
            format!("/* {} */", lines[0])
        } else {
            let mut result = String::from("/*");
            for line in lines {
                result.push('\n');
                if line.is_empty() {
                    result.push_str(" *");
                } else {
                    result.push_str(" * ");
                    result.push_str(line);
                }
            }
            result.push_str("\n */");
            result
        }
    }

    fn line_comment_prefix(&self) -> &str {
        "//"
    }

    fn indent_unit(&self) -> &str {
        &self.indent
    }

    fn uses_semicolons(&self) -> bool {
        true
    }

    fn render_visibility(&self, _vis: Visibility, _ctx: DeclarationContext) -> &str {
        // C has no visibility keywords.
        ""
    }

    fn function_keyword(&self, _ctx: DeclarationContext) -> &str {
        // C has no function keyword.
        ""
    }

    fn return_type_separator(&self) -> &str {
        // Unused when return_type_is_prefix() is true, but set for safety.
        " "
    }

    fn type_keyword(&self, kind: TypeKind) -> &str {
        match kind {
            TypeKind::Struct | TypeKind::Class => "struct",
            TypeKind::Enum => "enum",
            TypeKind::Interface | TypeKind::Trait => "struct",
            TypeKind::TypeAlias => "typedef",
            TypeKind::Newtype => "typedef",
        }
    }

    fn field_terminator(&self) -> &str {
        ";"
    }

    fn methods_inside_type_body(&self, _kind: TypeKind) -> bool {
        false
    }

    fn type_alias_target_first(&self) -> bool {
        true
    }

    fn render_newtype_line(&self, _vis: &str, name: &str, inner: &str) -> String {
        format!("typedef {inner} {name};")
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

    fn type_before_name(&self) -> bool {
        true
    }

    fn return_type_is_prefix(&self) -> bool {
        true
    }

    fn type_close_terminator(&self) -> &str {
        ";"
    }

    fn render_annotation_prefix(&self) -> (&str, &str) {
        ("__attribute__((", "))")
    }

    fn optional_field_style(&self) -> crate::lang::config::OptionalFieldStyle {
        crate::lang::config::OptionalFieldStyle::TypePrefix("*")
    }

    fn present_pointer(&self) -> crate::type_name::TypePresentation<'_> {
        crate::type_name::TypePresentation::Postfix { suffix: "*" }
    }

    fn present_reference(&self) -> crate::type_name::TypePresentation<'_> {
        crate::type_name::TypePresentation::Surround {
            prefix: "const ",
            suffix: "*",
        }
    }

    fn present_reference_mut(&self) -> crate::type_name::TypePresentation<'_> {
        crate::type_name::TypePresentation::Postfix { suffix: "*" }
    }

    fn present_function(&self) -> crate::type_name::FunctionPresentation<'_> {
        crate::type_name::FunctionPresentation {
            keyword: "",
            params_open: "(",
            params_sep: ", ",
            params_close: ")",
            arrow: "",
            return_first: true,
            curried: false,
            wrapper_open: "",
            wrapper_close: "",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_extension() {
        let c = CLang::new();
        assert_eq!(c.file_extension(), "c");
    }

    #[test]
    fn test_header_extension() {
        let c = CLang::header();
        assert_eq!(c.file_extension(), "h");
    }

    #[test]
    fn test_escape_reserved() {
        let c = CLang::new();
        assert_eq!(c.escape_reserved("int"), "int_");
        assert_eq!(c.escape_reserved("struct"), "struct_");
        assert_eq!(c.escape_reserved("name"), "name");
    }

    #[test]
    fn test_render_imports_system() {
        let c = CLang::new();
        let imports = ImportGroup {
            entries: vec![ImportEntry {
                module: "stdio.h".into(),
                name: "printf".into(),
                alias: None,
                is_type_only: false,
                is_side_effect: false,
                is_wildcard: false,
            }],
        };
        assert_eq!(c.render_imports(&imports), "#include <stdio.h>");
    }

    #[test]
    fn test_render_imports_local() {
        let c = CLang::new();
        let imports = ImportGroup {
            entries: vec![ImportEntry {
                module: "./config.h".into(),
                name: "Config".into(),
                alias: None,
                is_type_only: false,
                is_side_effect: false,
                is_wildcard: false,
            }],
        };
        assert_eq!(c.render_imports(&imports), "#include \"config.h\"");
    }

    #[test]
    fn test_render_imports_grouped() {
        let c = CLang::new();
        let imports = ImportGroup {
            entries: vec![
                ImportEntry {
                    module: "stdio.h".into(),
                    name: "printf".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "stdlib.h".into(),
                    name: "malloc".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "./config.h".into(),
                    name: "Config".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
            ],
        };
        let output = c.render_imports(&imports);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines[0], "#include <stdio.h>");
        assert_eq!(lines[1], "#include <stdlib.h>");
        assert_eq!(lines[2], "");
        assert_eq!(lines[3], "#include \"config.h\"");
    }

    #[test]
    fn test_render_imports_dedup() {
        let c = CLang::new();
        let imports = ImportGroup {
            entries: vec![
                ImportEntry {
                    module: "stdio.h".into(),
                    name: "printf".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "stdio.h".into(),
                    name: "fprintf".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
            ],
        };
        assert_eq!(c.render_imports(&imports), "#include <stdio.h>");
    }

    #[test]
    fn test_doc_comment_single() {
        let c = CLang::new();
        assert_eq!(
            c.render_doc_comment(&["A brief description."]),
            "/* A brief description. */"
        );
    }

    #[test]
    fn test_doc_comment_multi() {
        let c = CLang::new();
        let doc = c.render_doc_comment(&["Configuration struct.", "", "Thread-safe."]);
        assert!(doc.starts_with("/*"));
        assert!(doc.ends_with(" */"));
        assert!(doc.contains(" * Configuration struct."));
        assert!(doc.contains(" *\n"));
        assert!(doc.contains(" * Thread-safe."));
    }

    #[test]
    fn test_string_literal() {
        let c = CLang::new();
        assert_eq!(c.render_string_literal("hello"), "\"hello\"");
        assert_eq!(c.render_string_literal("it\"s"), "\"it\\\"s\"");
        assert_eq!(c.render_string_literal("new\nline"), "\"new\\nline\"");
    }

    #[test]
    fn test_type_before_name() {
        let c = CLang::new();
        assert!(c.type_before_name());
    }

    #[test]
    fn test_return_type_is_prefix() {
        let c = CLang::new();
        assert!(c.return_type_is_prefix());
    }

    #[test]
    fn test_type_close_terminator() {
        let c = CLang::new();
        assert_eq!(c.type_close_terminator(), ";");
    }

    #[test]
    fn test_is_system_header() {
        assert!(is_system_header("stdio.h"));
        assert!(is_system_header("stdlib.h"));
        assert!(is_system_header("string.h"));
        assert!(!is_system_header("./config.h"));
        assert!(!is_system_header("../utils/helper.h"));
    }

    #[test]
    fn test_c_builder_fluent() {
        let c = CLang::new().with_indent("\t").with_extension("h");
        assert_eq!(c.file_extension(), "h");
        assert_eq!(c.indent_unit(), "\t");
    }
}
