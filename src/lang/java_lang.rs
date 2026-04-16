//! Java language implementation.

use crate::import::ImportGroup;
use crate::lang::CodeLang;
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};

/// Java language implementation.
///
/// Java-specific behaviors:
/// - Type-before-name declarations (`int count`, not `count: int`)
/// - Return type as prefix (`int add(int a, int b)`)
/// - `import pkg.Class;` with java/javax/third-party grouping
/// - Semicolons after statements
/// - No function keyword (no `fn`/`func`/`def`)
/// - `class`, `interface`, and `enum` keywords
/// - `extends` for class inheritance, `implements` for interfaces
/// - Generic bounds via `extends` and `&` (`<T extends Comparable & Serializable>`)
/// - `/** ... */` Javadoc comments
/// - `final` instead of `const` for readonly fields
/// - Annotations (`@Override`, `@Nullable`) via `annotation()`
///
/// # Import conventions
///
/// Use [`crate::type_name::TypeName::importable`] with the package as module and class name as name:
/// ```ignore
/// TypeName::importable("java.util", "List")            // import java.util.List;
/// TypeName::importable("java.util", "Map")             // import java.util.Map;
/// TypeName::importable("com.example.model", "User")    // import com.example.model.User;
/// ```
///
/// # Annotations
///
/// Use `annotation()` on builders for Java annotations:
/// ```ignore
/// fb.annotation(CodeBlock::<JavaLang>::of("@Override", ()).unwrap());
/// fb.annotation(CodeBlock::<JavaLang>::of("@Nullable", ()).unwrap());
/// ```
///
/// # Constructors
///
/// Omit `.returns()` — with `return_type_is_prefix()`, no return type means
/// the signature starts with modifiers then name: `public ClassName(...)`.
#[derive(Debug, Clone)]
pub struct JavaLang {
    /// Indent with this string (default: "    " — 4 spaces).
    pub indent: String,
}

impl Default for JavaLang {
    fn default() -> Self {
        Self {
            indent: "    ".to_string(),
        }
    }
}

impl JavaLang {
    /// Create a new Java language instance.
    pub fn new() -> Self {
        Self::default()
    }
}

#[rustfmt::skip]
const JAVA_RESERVED: &[&str] = &[
    "abstract", "assert", "boolean", "break", "byte", "case", "catch",
    "char", "class", "const", "continue", "default", "do", "double",
    "else", "enum", "extends", "final", "finally", "float", "for",
    "goto", "if", "implements", "import", "instanceof", "int",
    "interface", "long", "native", "new", "package", "private",
    "protected", "public", "return", "short", "static", "strictfp",
    "super", "switch", "synchronized", "this", "throw", "throws",
    "transient", "try", "var", "void", "volatile", "while", "yield",
];

/// Classify an import module into a group for ordering.
/// 0 = java.*, 1 = javax.*, 2 = everything else.
fn import_group_order(module: &str) -> u8 {
    if module.starts_with("java.") {
        0
    } else if module.starts_with("javax.") {
        1
    } else {
        2
    }
}

impl CodeLang for JavaLang {
    fn file_extension(&self) -> &str {
        "java"
    }

    fn reserved_words(&self) -> &[&str] {
        JAVA_RESERVED
    }

    fn render_imports(&self, imports: &ImportGroup) -> String {
        if imports.entries.is_empty() {
            return String::new();
        }

        // Collect unique fully-qualified imports, grouped by category.
        let mut java_imports: Vec<String> = Vec::new();
        let mut javax_imports: Vec<String> = Vec::new();
        let mut other_imports: Vec<String> = Vec::new();

        let mut seen = std::collections::BTreeSet::new();
        for entry in &imports.entries {
            let line = if entry.is_wildcard {
                let fqn = format!("{}.*", entry.module);
                if !seen.insert(fqn.clone()) {
                    continue;
                }
                format!("import {};", fqn)
            } else if entry.is_side_effect {
                // Java has no side-effect imports; skip.
                continue;
            } else {
                let fqn = format!("{}.{}", entry.module, entry.name);
                if !seen.insert(fqn.clone()) {
                    continue;
                }
                format!("import {};", fqn)
            };

            match import_group_order(&entry.module) {
                0 => java_imports.push(line),
                1 => javax_imports.push(line),
                _ => other_imports.push(line),
            }
        }

        java_imports.sort();
        javax_imports.sort();
        other_imports.sort();

        let groups: Vec<&Vec<String>> = [&java_imports, &javax_imports, &other_imports]
            .into_iter()
            .filter(|g| !g.is_empty())
            .collect();

        let mut lines = Vec::new();
        for (i, group) in groups.iter().enumerate() {
            if i > 0 {
                lines.push(String::new());
            }
            lines.extend(group.iter().cloned());
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
        // Javadoc /** ... */ style.
        let mut result = String::from("/**");
        for line in lines {
            result.push('\n');
            if line.is_empty() {
                result.push_str(" *");
            } else {
                result.push_str(" * ");
                result.push_str(line);
            }
        }
        result.push('\n');
        result.push_str(" */");
        result
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

    fn render_visibility(&self, vis: Visibility, ctx: DeclarationContext) -> &str {
        match ctx {
            DeclarationContext::TopLevel => match vis {
                Visibility::Public => "public ",
                _ => "", // package-private
            },
            DeclarationContext::Member => match vis {
                Visibility::Public => "public ",
                Visibility::Private => "private ",
                Visibility::Protected => "protected ",
                _ => "", // package-private
            },
        }
    }

    fn function_keyword(&self, _ctx: DeclarationContext) -> &str {
        // Java has no function keyword.
        ""
    }

    fn return_type_separator(&self) -> &str {
        // Unused when return_type_is_prefix() is true, but set for safety.
        " "
    }

    fn type_keyword(&self, kind: TypeKind) -> &str {
        match kind {
            TypeKind::Class | TypeKind::Struct => "class",
            TypeKind::Interface | TypeKind::Trait => "interface",
            TypeKind::Enum => "enum",
        }
    }

    fn field_terminator(&self) -> &str {
        ";"
    }

    fn methods_inside_type_body(&self, _kind: TypeKind) -> bool {
        true
    }

    fn generic_constraint_keyword(&self) -> &str {
        " extends "
    }

    fn generic_constraint_separator(&self) -> &str {
        " & "
    }

    fn super_type_keyword(&self) -> &str {
        " extends "
    }

    fn implements_keyword(&self) -> &str {
        " implements "
    }

    fn type_before_name(&self) -> bool {
        true
    }

    fn return_type_is_prefix(&self) -> bool {
        true
    }

    fn async_keyword(&self) -> &str {
        // Java has no async keyword.
        ""
    }

    fn readonly_keyword(&self) -> &str {
        "final "
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import::ImportEntry;

    #[test]
    fn test_file_extension() {
        let java = JavaLang::new();
        assert_eq!(java.file_extension(), "java");
    }

    #[test]
    fn test_escape_reserved() {
        let java = JavaLang::new();
        assert_eq!(java.escape_reserved("class"), "class_");
        assert_eq!(java.escape_reserved("import"), "import_");
        assert_eq!(java.escape_reserved("synchronized"), "synchronized_");
        assert_eq!(java.escape_reserved("name"), "name");
    }

    #[test]
    fn test_render_imports_single() {
        let java = JavaLang::new();
        let imports = ImportGroup {
            entries: vec![ImportEntry {
                module: "java.util".into(),
                name: "List".into(),
                alias: None,
                is_type_only: false,
                is_side_effect: false,
                is_wildcard: false,
            }],
        };
        assert_eq!(java.render_imports(&imports), "import java.util.List;");
    }

    #[test]
    fn test_render_imports_grouped() {
        let java = JavaLang::new();
        let imports = ImportGroup {
            entries: vec![
                ImportEntry {
                    module: "com.example.model".into(),
                    name: "User".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "java.util".into(),
                    name: "List".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "javax.persistence".into(),
                    name: "Entity".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
            ],
        };
        let output = java.render_imports(&imports);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines[0], "import java.util.List;");
        assert_eq!(lines[1], "");
        assert_eq!(lines[2], "import javax.persistence.Entity;");
        assert_eq!(lines[3], "");
        assert_eq!(lines[4], "import com.example.model.User;");
    }

    #[test]
    fn test_render_imports_sorted_within_group() {
        let java = JavaLang::new();
        let imports = ImportGroup {
            entries: vec![
                ImportEntry {
                    module: "java.util".into(),
                    name: "Map".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "java.io".into(),
                    name: "File".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "java.util".into(),
                    name: "List".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
            ],
        };
        let output = java.render_imports(&imports);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines[0], "import java.io.File;");
        assert_eq!(lines[1], "import java.util.List;");
        assert_eq!(lines[2], "import java.util.Map;");
    }

    #[test]
    fn test_render_imports_dedup() {
        let java = JavaLang::new();
        let imports = ImportGroup {
            entries: vec![
                ImportEntry {
                    module: "java.util".into(),
                    name: "List".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "java.util".into(),
                    name: "List".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
            ],
        };
        assert_eq!(java.render_imports(&imports), "import java.util.List;");
    }

    #[test]
    fn test_doc_comment_single() {
        let java = JavaLang::new();
        assert_eq!(
            java.render_doc_comment(&["A brief description."]),
            "/**\n * A brief description.\n */"
        );
    }

    #[test]
    fn test_doc_comment_multi() {
        let java = JavaLang::new();
        let doc = java.render_doc_comment(&["Container class.", "", "@param <T> the element type"]);
        assert_eq!(
            doc,
            "/**\n * Container class.\n *\n * @param <T> the element type\n */"
        );
    }

    #[test]
    fn test_string_literal() {
        let java = JavaLang::new();
        assert_eq!(java.render_string_literal("hello"), "\"hello\"");
        assert_eq!(java.render_string_literal("it\"s"), "\"it\\\"s\"");
        assert_eq!(java.render_string_literal("new\nline"), "\"new\\nline\"");
    }

    #[test]
    fn test_type_keyword() {
        let java = JavaLang::new();
        assert_eq!(java.type_keyword(TypeKind::Class), "class");
        assert_eq!(java.type_keyword(TypeKind::Struct), "class");
        assert_eq!(java.type_keyword(TypeKind::Interface), "interface");
        assert_eq!(java.type_keyword(TypeKind::Trait), "interface");
        assert_eq!(java.type_keyword(TypeKind::Enum), "enum");
    }

    #[test]
    fn test_visibility_top_level() {
        let java = JavaLang::new();
        assert_eq!(
            java.render_visibility(Visibility::Public, DeclarationContext::TopLevel),
            "public "
        );
        assert_eq!(
            java.render_visibility(Visibility::Private, DeclarationContext::TopLevel),
            ""
        );
    }

    #[test]
    fn test_visibility_member() {
        let java = JavaLang::new();
        assert_eq!(
            java.render_visibility(Visibility::Public, DeclarationContext::Member),
            "public "
        );
        assert_eq!(
            java.render_visibility(Visibility::Private, DeclarationContext::Member),
            "private "
        );
        assert_eq!(
            java.render_visibility(Visibility::Protected, DeclarationContext::Member),
            "protected "
        );
    }

    #[test]
    fn test_type_before_name() {
        let java = JavaLang::new();
        assert!(java.type_before_name());
    }

    #[test]
    fn test_return_type_is_prefix() {
        let java = JavaLang::new();
        assert!(java.return_type_is_prefix());
    }

    #[test]
    fn test_readonly_keyword() {
        let java = JavaLang::new();
        assert_eq!(java.readonly_keyword(), "final ");
    }

    #[test]
    fn test_no_async() {
        let java = JavaLang::new();
        assert_eq!(java.async_keyword(), "");
    }

    #[test]
    fn test_import_group_order() {
        assert_eq!(import_group_order("java.util"), 0);
        assert_eq!(import_group_order("java.io"), 0);
        assert_eq!(import_group_order("javax.persistence"), 1);
        assert_eq!(import_group_order("com.example.model"), 2);
        assert_eq!(import_group_order("org.springframework"), 2);
    }
}
