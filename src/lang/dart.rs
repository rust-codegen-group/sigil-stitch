//! Dart language implementation.

use crate::import::ImportGroup;
use crate::lang::{CodeLang, RendererLang};
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
/// - `async` as a body modifier suffix (`Future<int> foo() async { ... }`)
///
/// # Import conventions
///
/// Use [`crate::type_name::TypeName::importable`] with the import URI as module:
/// ```text
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
/// ```text
/// tb.extends(TypeName::raw("BaseClass with Mixin1, Mixin2"));
/// // Emits: class Foo extends BaseClass with Mixin1, Mixin2 {
/// ```
///
/// # Async functions
///
/// Dart's `async` is a body modifier (`Future<int> foo() async { ... }`),
/// not a signature prefix. Set `is_async()` on the builder and use
/// `Future<T>` as the return type:
/// ```text
/// fb.returns(TypeName::primitive("Future<User>"))
///   .is_async();
/// ```
#[derive(Debug, Clone)]
pub struct Dart {
    /// Indent with this string (default: "  " — 2 spaces per Dart style guide).
    pub indent: String,
    /// File extension (default: "dart").
    pub extension: String,
}

impl Default for Dart {
    fn default() -> Self {
        Self {
            indent: "  ".to_string(),
            extension: "dart".to_string(),
        }
    }
}

impl Dart {
    /// Create a new Dart language instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the indent string (e.g., `"  "` for 2-space default, `"    "` for 4 spaces).
    pub fn with_indent(mut self, s: &str) -> Self {
        self.indent = s.to_string();
        self
    }

    /// Set the file extension (default: `"dart"`).
    pub fn with_extension(mut self, s: &str) -> Self {
        self.extension = s.to_string();
        self
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

impl RendererLang for Dart {
    fn file_extension(&self) -> &str {
        &self.extension
    }

    fn reserved_words(&self) -> &[&str] {
        DART_RESERVED
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

    fn render_verbatim_string(&self, s: &str) -> String {
        let escaped = s.replace('\\', "\\\\").replace('\'', "\\'");
        format!("'{escaped}'")
    }

    fn line_comment_prefix(&self) -> &str {
        "//"
    }

    fn type_presentation(&self) -> crate::lang::config::TypePresentationConfig<'_> {
        crate::lang::config::TypePresentationConfig {
            array: crate::type_name::TypePresentation::GenericWrap { name: "List" },
            readonly_array: Some(crate::type_name::TypePresentation::GenericWrap { name: "List" }),
            optional: crate::type_name::TypePresentation::Postfix { suffix: "?" },
            function: crate::type_name::FunctionPresentation {
                keyword: " Function",
                params_open: "(",
                params_sep: ", ",
                params_close: ")",
                arrow: "",
                return_first: true,
                curried: false,
                wrapper_open: "",
                wrapper_close: "",
            },
            ..Default::default()
        }
    }

    fn generic_syntax(&self) -> crate::lang::config::GenericSyntaxConfig<'_> {
        crate::lang::config::GenericSyntaxConfig {
            constraint_keyword: " extends ",
            constraint_separator: ", ",
            context_bound_keyword: " extends ",
            ..Default::default()
        }
    }

    fn module_separator(&self) -> Option<&str> {
        Some(".")
    }

    fn block_syntax(&self) -> crate::lang::config::BlockSyntaxConfig<'_> {
        crate::lang::config::BlockSyntaxConfig {
            indent_unit: &self.indent,
            field_terminator: ";",
            ..Default::default()
        }
    }
}

impl CodeLang for Dart {
    fn render_imports(&self, imports: &ImportGroup) -> String {
        if imports.entries().is_empty() {
            return String::new();
        }

        // Dart imports entire files — deduplicate at the module (URI) level.
        let mut dart_imports: Vec<String> = Vec::new();
        let mut package_imports: Vec<String> = Vec::new();
        let mut relative_imports: Vec<String> = Vec::new();

        let mut seen = std::collections::BTreeSet::new();
        for entry in imports.entries() {
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

    fn render_visibility(&self, _vis: Visibility, _ctx: DeclarationContext) -> &str {
        // Dart has no visibility keywords; privacy is via _ prefix naming.
        ""
    }

    fn function_keyword(&self, _ctx: DeclarationContext) -> &str {
        // Dart has no function keyword (like Java/C).
        ""
    }

    fn type_keyword(&self, kind: TypeKind) -> &str {
        match kind {
            TypeKind::Class | TypeKind::Struct => "class",
            TypeKind::Interface | TypeKind::Trait => "abstract class",
            TypeKind::Enum => "enum",
            TypeKind::TypeAlias => "typedef",
            TypeKind::Newtype => "class",
        }
    }

    fn methods_inside_type_body(&self, _kind: TypeKind) -> bool {
        true
    }

    fn optional_field_style(&self) -> crate::lang::config::OptionalFieldStyle {
        crate::lang::config::OptionalFieldStyle::TypeSuffix("?")
    }

    fn function_syntax(&self) -> crate::lang::config::FunctionSyntaxConfig<'_> {
        crate::lang::config::FunctionSyntaxConfig {
            return_type_separator: " ",
            async_keyword: "",
            async_suffix: " async",
            ..Default::default()
        }
    }

    fn type_decl_syntax(&self) -> crate::lang::config::TypeDeclSyntaxConfig<'_> {
        crate::lang::config::TypeDeclSyntaxConfig {
            type_before_name: true,
            return_type_is_prefix: true,
            super_type_keyword: " extends ",
            implements_keyword: " implements ",
            ..Default::default()
        }
    }

    fn enum_and_annotation(&self) -> crate::lang::config::EnumAndAnnotationConfig<'_> {
        crate::lang::config::EnumAndAnnotationConfig {
            readonly_keyword: "final ",
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import::ImportEntry;

    #[test]
    fn test_file_extension() {
        let d = Dart::new();
        assert_eq!(d.file_extension(), "dart");
    }

    #[test]
    fn test_escape_reserved() {
        let d = Dart::new();
        assert_eq!(d.escape_reserved("class"), "class_");
        assert_eq!(d.escape_reserved("import"), "import_");
        assert_eq!(d.escape_reserved("final"), "final_");
        assert_eq!(d.escape_reserved("name"), "name");
    }

    #[test]
    fn test_render_imports_single() {
        let d = Dart::new();
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
        let d = Dart::new();
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
        let d = Dart::new();
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
        let d = Dart::new();
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
        let d = Dart::new();
        assert_eq!(
            d.render_doc_comment(&["A brief description."]),
            "/// A brief description."
        );
    }

    #[test]
    fn test_doc_comment_multi() {
        let d = Dart::new();
        let doc = d.render_doc_comment(&["Container class.", "", "See also [OtherClass]."]);
        assert_eq!(doc, "/// Container class.\n///\n/// See also [OtherClass].");
    }

    #[test]
    fn test_string_literal() {
        let d = Dart::new();
        assert_eq!(d.render_string_literal("hello"), "'hello'");
        assert_eq!(d.render_string_literal("it's"), "'it\\'s'");
        assert_eq!(d.render_string_literal("new\nline"), "'new\\nline'");
        // Dart needs $ escaping for string interpolation.
        assert_eq!(d.render_string_literal("$name"), "'\\$name'");
    }

    #[test]
    fn test_type_keyword() {
        let d = Dart::new();
        assert_eq!(d.type_keyword(TypeKind::Class), "class");
        assert_eq!(d.type_keyword(TypeKind::Struct), "class");
        assert_eq!(d.type_keyword(TypeKind::Interface), "abstract class");
        assert_eq!(d.type_keyword(TypeKind::Trait), "abstract class");
        assert_eq!(d.type_keyword(TypeKind::Enum), "enum");
    }

    #[test]
    fn test_no_visibility_keywords() {
        let d = Dart::new();
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
        let d = Dart::new();
        assert!(d.type_decl_syntax().type_before_name);
    }

    #[test]
    fn test_return_type_is_prefix() {
        let d = Dart::new();
        assert!(d.type_decl_syntax().return_type_is_prefix);
    }

    #[test]
    fn test_readonly_keyword() {
        let d = Dart::new();
        assert_eq!(d.enum_and_annotation().readonly_keyword, "final ");
    }

    #[test]
    fn test_no_async_keyword() {
        let d = Dart::new();
        assert_eq!(d.function_syntax().async_keyword, "");
    }

    #[test]
    fn test_async_suffix() {
        let d = Dart::new();
        assert_eq!(d.function_syntax().async_suffix, " async");
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

    #[test]
    fn test_dart_builder_fluent() {
        let d = Dart::new().with_indent("    ").with_extension("g.dart");
        assert_eq!(d.file_extension(), "g.dart");
        assert_eq!(d.block_syntax().indent_unit, "    ");
    }

    #[test]
    fn test_module_separator() {
        let d = Dart::new();
        assert_eq!(d.module_separator(), Some("."));
    }
}
