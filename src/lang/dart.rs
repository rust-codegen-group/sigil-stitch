//! Dart language implementation.

use crate::import::ImportGroup;
use crate::lang::CodeLang;
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};

/// Dart language implementation.
///
/// Dart-specific behaviors:
/// - Type-before-name declarations (`String name`, not `name: String`)
/// - Return type as prefix (`String getName()`, `Future<User> fetch()`)
/// - `import 'package:foo/bar.dart';` with dart/package/relative grouping
/// - Semicolons after statements and field declarations
/// - No function keyword (like Java/C)
/// - No visibility keywords (privacy via `_` prefix naming convention)
/// - `class`, `abstract class`, `enum`, `mixin` keywords
/// - `extends` for superclass, `implements` for interfaces
/// - `final` for readonly fields
/// - `///` dartdoc comments
/// - `<T extends Bound>` generics (same as Java/TS)
/// - `@override`, `@required` annotations via `annotation()`
/// - No `async` prefix — Dart's `async` is a body modifier, not a signature modifier
///
/// # Import conventions
///
/// Use [`crate::type_name::TypeName::importable`] with the import URI as module:
/// ```ignore
/// TypeName::importable("dart:async", "Future")              // import 'dart:async';
/// TypeName::importable("package:http/http.dart", "Client")   // import 'package:http/http.dart';
/// TypeName::importable("../models/user.dart", "User")        // import '../models/user.dart';
/// ```
///
/// Dart imports entire files, so the module (URI) is what matters for import emission.
///
/// # Mixins
///
/// Dart's `with` keyword for mixin application is not directly in the trait.
/// Include mixins via `TypeName::raw`:
/// ```ignore
/// tb.extends(TypeName::raw("BaseClass with Mixin1, Mixin2"));
/// // Emits: class Foo extends BaseClass with Mixin1, Mixin2 {
/// ```
///
/// # Async functions
///
/// Dart's `async` is a body modifier (`Future<int> foo() async { ... }`),
/// not a signature prefix. Use `Future<T>` as the return type:
/// ```ignore
/// fb.returns(TypeName::primitive("Future<User>"));
/// ```
#[derive(Debug, Clone)]
pub struct DartLang {
    /// Indent with this string (default: "  " — 2 spaces per Dart style guide).
    pub indent: String,
}

impl Default for DartLang {
    fn default() -> Self {
        Self {
            indent: "  ".to_string(),
        }
    }
}

impl DartLang {
    /// Create a new Dart language instance.
    pub fn new() -> Self {
        Self::default()
    }
}

#[rustfmt::skip]
const DART_RESERVED: &[&str] = &[
    // Keywords
    "abstract", "as", "assert", "async", "await", "base", "break", "case",
    "catch", "class", "const", "continue", "covariant", "default", "deferred",
    "do", "dynamic", "else", "enum", "export", "extends", "extension",
    "external", "factory", "false", "final", "finally", "for", "Function",
    "get", "hide", "if", "implements", "import", "in", "interface", "is",
    "late", "library", "mixin", "new", "null", "of", "on", "operator",
    "part", "required", "rethrow", "return", "sealed", "set", "show",
    "static", "super", "switch", "sync", "this", "throw", "true", "try",
    "typedef", "var", "void", "when", "while", "with", "yield",
];

/// Classify a Dart import URI into a group for ordering.
/// 0 = dart:* (SDK), 1 = package:* (pub packages), 2 = relative imports.
fn import_group_order(module: &str) -> u8 {
    if module.starts_with("dart:") {
        0
    } else if module.starts_with("package:") {
        1
    } else {
        2
    }
}

impl CodeLang for DartLang {
    fn file_extension(&self) -> &str {
        "dart"
    }

    fn reserved_words(&self) -> &[&str] {
        DART_RESERVED
    }

    fn render_imports(&self, imports: &ImportGroup) -> String {
        if imports.entries.is_empty() {
            return String::new();
        }

        // Dart imports entire files — deduplicate at the module (URI) level.
        let mut dart_imports: Vec<String> = Vec::new();
        let mut package_imports: Vec<String> = Vec::new();
        let mut relative_imports: Vec<String> = Vec::new();

        let mut seen = std::collections::BTreeSet::new();
        for entry in &imports.entries {
            if !seen.insert(&entry.module) {
                continue;
            }

            let line = format!("import '{}';", entry.module);
            match import_group_order(&entry.module) {
                0 => dart_imports.push(line),
                1 => package_imports.push(line),
                _ => relative_imports.push(line),
            }
        }

        dart_imports.sort();
        package_imports.sort();
        relative_imports.sort();

        let groups: Vec<&Vec<String>> = [&dart_imports, &package_imports, &relative_imports]
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
        // Dart prefers single quotes by convention.
        format!(
            "'{}'",
            s.replace('\\', "\\\\")
                .replace('\'', "\\'")
                .replace('\n', "\\n")
                .replace('\t', "\\t")
                .replace('\r', "\\r")
                .replace('\0', "\\0")
                .replace('$', "\\$")
        )
    }

    fn render_doc_comment(&self, lines: &[&str]) -> String {
        // Dartdoc uses /// line-prefix style.
        let mut result = String::new();
        for (i, line) in lines.iter().enumerate() {
            if i > 0 {
                result.push('\n');
            }
            if line.is_empty() {
                result.push_str("///");
            } else {
                result.push_str("/// ");
                result.push_str(line);
            }
        }
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

    fn render_visibility(&self, _vis: Visibility, _ctx: DeclarationContext) -> &str {
        // Dart has no visibility keywords; privacy is via _ prefix naming.
        ""
    }

    fn function_keyword(&self, _ctx: DeclarationContext) -> &str {
        // Dart has no function keyword (like Java/C).
        ""
    }

    fn return_type_separator(&self) -> &str {
        // Unused when return_type_is_prefix() is true, but set for safety.
        " "
    }

    fn type_keyword(&self, kind: TypeKind) -> &str {
        match kind {
            TypeKind::Class | TypeKind::Struct => "class",
            TypeKind::Interface | TypeKind::Trait => "abstract class",
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
        ", "
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
        // Dart's `async` is a body modifier, not a signature prefix.
        // Use Future<T> as return type instead.
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
        let d = DartLang::new();
        assert_eq!(d.file_extension(), "dart");
    }

    #[test]
    fn test_escape_reserved() {
        let d = DartLang::new();
        assert_eq!(d.escape_reserved("class"), "class_");
        assert_eq!(d.escape_reserved("import"), "import_");
        assert_eq!(d.escape_reserved("final"), "final_");
        assert_eq!(d.escape_reserved("name"), "name");
    }

    #[test]
    fn test_render_imports_single() {
        let d = DartLang::new();
        let imports = ImportGroup {
            entries: vec![ImportEntry {
                module: "dart:async".into(),
                name: "Future".into(),
                alias: None,
                is_type_only: false,
                is_side_effect: false,
                is_wildcard: false,
            }],
        };
        assert_eq!(d.render_imports(&imports), "import 'dart:async';");
    }

    #[test]
    fn test_render_imports_grouped() {
        let d = DartLang::new();
        let imports = ImportGroup {
            entries: vec![
                ImportEntry {
                    module: "package:http/http.dart".into(),
                    name: "Client".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "dart:async".into(),
                    name: "Future".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "../models/user.dart".into(),
                    name: "User".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
            ],
        };
        let output = d.render_imports(&imports);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines[0], "import 'dart:async';");
        assert_eq!(lines[1], "");
        assert_eq!(lines[2], "import 'package:http/http.dart';");
        assert_eq!(lines[3], "");
        assert_eq!(lines[4], "import '../models/user.dart';");
    }

    #[test]
    fn test_render_imports_sorted_within_group() {
        let d = DartLang::new();
        let imports = ImportGroup {
            entries: vec![
                ImportEntry {
                    module: "dart:io".into(),
                    name: "File".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "dart:async".into(),
                    name: "Future".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "dart:convert".into(),
                    name: "json".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
            ],
        };
        let output = d.render_imports(&imports);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines[0], "import 'dart:async';");
        assert_eq!(lines[1], "import 'dart:convert';");
        assert_eq!(lines[2], "import 'dart:io';");
    }

    #[test]
    fn test_render_imports_dedup() {
        let d = DartLang::new();
        let imports = ImportGroup {
            entries: vec![
                ImportEntry {
                    module: "dart:async".into(),
                    name: "Future".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "dart:async".into(),
                    name: "Stream".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
            ],
        };
        assert_eq!(d.render_imports(&imports), "import 'dart:async';");
    }

    #[test]
    fn test_doc_comment_single() {
        let d = DartLang::new();
        assert_eq!(
            d.render_doc_comment(&["A brief description."]),
            "/// A brief description."
        );
    }

    #[test]
    fn test_doc_comment_multi() {
        let d = DartLang::new();
        let doc = d.render_doc_comment(&["Container class.", "", "See also [OtherClass]."]);
        assert_eq!(doc, "/// Container class.\n///\n/// See also [OtherClass].");
    }

    #[test]
    fn test_string_literal() {
        let d = DartLang::new();
        assert_eq!(d.render_string_literal("hello"), "'hello'");
        assert_eq!(d.render_string_literal("it's"), "'it\\'s'");
        assert_eq!(d.render_string_literal("new\nline"), "'new\\nline'");
        // Dart needs $ escaping for string interpolation.
        assert_eq!(d.render_string_literal("$name"), "'\\$name'");
    }

    #[test]
    fn test_type_keyword() {
        let d = DartLang::new();
        assert_eq!(d.type_keyword(TypeKind::Class), "class");
        assert_eq!(d.type_keyword(TypeKind::Struct), "class");
        assert_eq!(d.type_keyword(TypeKind::Interface), "abstract class");
        assert_eq!(d.type_keyword(TypeKind::Trait), "abstract class");
        assert_eq!(d.type_keyword(TypeKind::Enum), "enum");
    }

    #[test]
    fn test_no_visibility_keywords() {
        let d = DartLang::new();
        assert_eq!(
            d.render_visibility(Visibility::Public, DeclarationContext::TopLevel),
            ""
        );
        assert_eq!(
            d.render_visibility(Visibility::Private, DeclarationContext::Member),
            ""
        );
        assert_eq!(
            d.render_visibility(Visibility::Protected, DeclarationContext::Member),
            ""
        );
    }

    #[test]
    fn test_type_before_name() {
        let d = DartLang::new();
        assert!(d.type_before_name());
    }

    #[test]
    fn test_return_type_is_prefix() {
        let d = DartLang::new();
        assert!(d.return_type_is_prefix());
    }

    #[test]
    fn test_readonly_keyword() {
        let d = DartLang::new();
        assert_eq!(d.readonly_keyword(), "final ");
    }

    #[test]
    fn test_no_async_keyword() {
        let d = DartLang::new();
        assert_eq!(d.async_keyword(), "");
    }

    #[test]
    fn test_import_group_order() {
        assert_eq!(import_group_order("dart:async"), 0);
        assert_eq!(import_group_order("dart:io"), 0);
        assert_eq!(import_group_order("package:http/http.dart"), 1);
        assert_eq!(import_group_order("package:flutter/material.dart"), 1);
        assert_eq!(import_group_order("../models/user.dart"), 2);
        assert_eq!(import_group_order("./config.dart"), 2);
    }
}
