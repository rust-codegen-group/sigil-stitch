//! Swift language implementation.

use crate::import::ImportGroup;
use crate::lang::CodeLang;
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};

/// Swift language implementation.
///
/// Swift-specific behaviors:
/// - Name-before-type declarations (`count: Int`, not `Int count`)
/// - `func` keyword, `-> ReturnType` syntax
/// - Module-level `import Foundation` directives (Apple framework / third-party grouping)
/// - No semicolons
/// - `class`, `struct`, `protocol`, `enum` keywords
/// - Single `:` for both superclass and protocol conformance
/// - Generic bounds via `:` and `&` (`<T: Comparable & Hashable>`)
/// - `///` Swift Markup doc comments
/// - `let`/`var` for readonly/mutable properties
/// - Backtick escaping for reserved words
/// - Attributes (`@objc`, `@discardableResult`) via `annotation()`
/// - `async`/`await` concurrency (Swift 5.5+)
///
/// # Import conventions
///
/// Use [`crate::type_name::TypeName::importable`] with the module name and symbol:
/// ```text
/// TypeName::importable("Foundation", "URL")        // import Foundation
/// TypeName::importable("UIKit", "UIViewController") // import UIKit
/// TypeName::importable("MyModule", "MyType")        // import MyModule
/// ```
///
/// Swift imports entire modules, so only the module name matters for import emission.
/// Multiple symbols from the same module produce a single `import` line.
///
/// # Protocol conformance
///
/// Swift uses `:` for both superclass and protocol conformance. Put all supertypes
/// into `extends()` (not `implements()`):
/// ```text
/// tb.extends(TypeName::primitive("NSObject"));
/// tb.extends(TypeName::primitive("Codable"));
/// // Emits: class Foo: NSObject, Codable {
/// ```
///
/// # `@` Attributes
///
/// Use `annotation()` for Swift attributes:
/// ```text
/// fb.annotation(CodeBlock::<Swift>::of("@objc", ()).unwrap());
/// fb.annotation(CodeBlock::<Swift>::of("@discardableResult", ()).unwrap());
/// ```
#[derive(Debug, Clone)]
pub struct Swift {
    /// Indent with this string (default: "    " — 4 spaces).
    pub indent: String,
    /// File extension (default: "swift").
    pub extension: String,
}

impl Default for Swift {
    fn default() -> Self {
        Self {
            indent: "    ".to_string(),
            extension: "swift".to_string(),
        }
    }
}

impl Swift {
    /// Create a new Swift language instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the indent string (e.g., `"    "` for 4-space default, `"\t"` for tabs).
    pub fn with_indent(mut self, s: &str) -> Self {
        self.indent = s.to_string();
        self
    }

    /// Set the file extension (default: `"swift"`).
    pub fn with_extension(mut self, s: &str) -> Self {
        self.extension = s.to_string();
        self
    }
}

#[rustfmt::skip]
const SWIFT_RESERVED: &[&str] = &[
    // Declaration keywords
    "associatedtype", "class", "deinit", "enum", "extension", "fileprivate",
    "func", "import", "init", "inout", "internal", "let", "open", "operator",
    "private", "precedencegroup", "protocol", "public", "rethrows", "static",
    "struct", "subscript", "typealias", "var",
    // Statement keywords
    "break", "case", "catch", "continue", "default", "defer", "do", "else",
    "fallthrough", "for", "guard", "if", "in", "repeat", "return", "switch",
    "throw", "try", "where", "while",
    // Expression and type keywords
    "Any", "as", "false", "is", "nil", "self", "Self", "super", "throws",
    "true",
    // Context-sensitive keywords (reserved in certain positions)
    "async", "await", "some", "any", "actor", "nonisolated", "isolated",
    "consuming", "borrowing", "sending",
];

/// Common Apple/Swift standard library framework names.
/// Used to separate Apple framework imports from third-party imports.
const APPLE_FRAMEWORKS: &[&str] = &[
    "Accelerate",
    "Accessibility",
    "AppKit",
    "AuthenticationServices",
    "Combine",
    "Contacts",
    "CoreData",
    "CoreFoundation",
    "CoreGraphics",
    "CoreImage",
    "CoreLocation",
    "CoreML",
    "CoreMedia",
    "CoreMotion",
    "CryptoKit",
    "Darwin",
    "Dispatch",
    "Foundation",
    "GameKit",
    "HealthKit",
    "MapKit",
    "Metal",
    "NaturalLanguage",
    "Network",
    "Observation",
    "ObjectiveC",
    "Photos",
    "QuartzCore",
    "RealityKit",
    "RegexBuilder",
    "SafariServices",
    "SceneKit",
    "Security",
    "SpriteKit",
    "StoreKit",
    "Swift",
    "SwiftData",
    "SwiftUI",
    "SystemConfiguration",
    "UIKit",
    "UniformTypeIdentifiers",
    "UserNotifications",
    "Vision",
    "WatchKit",
    "WebKit",
    "WidgetKit",
    "XCTest",
    "os",
];

/// Returns true if the module is an Apple/Swift standard framework.
fn is_apple_framework(module: &str) -> bool {
    APPLE_FRAMEWORKS.contains(&module)
}

impl CodeLang for Swift {
    fn file_extension(&self) -> &str {
        &self.extension
    }

    fn reserved_words(&self) -> &[&str] {
        SWIFT_RESERVED
    }

    fn escape_reserved(&self, name: &str) -> String {
        if self.reserved_words().contains(&name) {
            format!("`{name}`")
        } else {
            name.to_string()
        }
    }

    fn render_imports(&self, imports: &ImportGroup) -> String {
        if imports.entries.is_empty() {
            return String::new();
        }

        // Swift imports entire modules — deduplicate at the module level.
        let mut apple_imports: Vec<String> = Vec::new();
        let mut other_imports: Vec<String> = Vec::new();

        let mut seen = std::collections::BTreeSet::new();
        for entry in &imports.entries {
            if !seen.insert(&entry.module) {
                continue;
            }

            let line = format!("import {}", entry.module);
            if is_apple_framework(&entry.module) {
                apple_imports.push(line);
            } else {
                other_imports.push(line);
            }
        }

        apple_imports.sort();
        other_imports.sort();

        let groups: Vec<&Vec<String>> = [&apple_imports, &other_imports]
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
        // Swift Markup: /// prefix per line.
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
        false
    }

    fn render_visibility(&self, vis: Visibility, _ctx: DeclarationContext) -> &str {
        match vis {
            Visibility::Public => "public ",
            Visibility::Private => "private ",
            Visibility::Protected => "internal ",
            Visibility::PublicCrate => "internal ",
            Visibility::PublicSuper => "fileprivate ",
            Visibility::Inherited => "",
        }
    }

    fn function_keyword(&self, _ctx: DeclarationContext) -> &str {
        "func"
    }

    fn return_type_separator(&self) -> &str {
        " -> "
    }

    fn type_keyword(&self, kind: TypeKind) -> &str {
        match kind {
            TypeKind::Class => "class",
            TypeKind::Struct => "struct",
            TypeKind::Interface | TypeKind::Trait => "protocol",
            TypeKind::Enum => "enum",
            TypeKind::TypeAlias | TypeKind::Newtype => "typealias",
        }
    }

    fn field_terminator(&self) -> &str {
        ""
    }

    fn methods_inside_type_body(&self, _kind: TypeKind) -> bool {
        true
    }

    fn generic_constraint_keyword(&self) -> &str {
        ": "
    }

    fn generic_constraint_separator(&self) -> &str {
        " & "
    }

    fn super_type_keyword(&self) -> &str {
        ": "
    }

    fn implements_keyword(&self) -> &str {
        // Swift uses a single `:` list for both superclass and protocols.
        // Put all conformances in super_types(); leave impl_types() empty.
        ""
    }

    fn readonly_keyword(&self) -> &str {
        "let "
    }

    fn mutable_field_keyword(&self) -> &str {
        "var "
    }

    fn abstract_keyword(&self) -> &str {
        // Swift protocols declare requirements by omitting the body — no keyword.
        ""
    }

    fn enum_variant_prefix(&self) -> &str {
        "case "
    }

    fn enum_variant_separator(&self) -> &str {
        ""
    }

    fn property_style(&self) -> crate::spec::modifiers::PropertyStyle {
        crate::spec::modifiers::PropertyStyle::Field
    }

    fn optional_field_style(&self) -> crate::lang::config::OptionalFieldStyle {
        crate::lang::config::OptionalFieldStyle::TypeSuffix("?")
    }

    fn present_array(&self) -> crate::type_name::TypePresentation<'_> {
        crate::type_name::TypePresentation::Delimited {
            open: "[",
            sep: "",
            close: "]",
        }
    }

    fn present_readonly_array(&self) -> Option<crate::type_name::TypePresentation<'_>> {
        Some(crate::type_name::TypePresentation::Delimited {
            open: "[",
            sep: "",
            close: "]",
        })
    }

    fn present_optional(&self) -> crate::type_name::TypePresentation<'_> {
        crate::type_name::TypePresentation::Postfix { suffix: "?" }
    }

    fn present_map(&self) -> crate::type_name::TypePresentation<'_> {
        crate::type_name::TypePresentation::Delimited {
            open: "[",
            sep: ": ",
            close: "]",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import::ImportEntry;

    #[test]
    fn test_file_extension() {
        let sw = Swift::new();
        assert_eq!(sw.file_extension(), "swift");
    }

    #[test]
    fn test_escape_reserved_backticks() {
        let sw = Swift::new();
        assert_eq!(sw.escape_reserved("class"), "`class`");
        assert_eq!(sw.escape_reserved("func"), "`func`");
        assert_eq!(sw.escape_reserved("let"), "`let`");
        assert_eq!(sw.escape_reserved("name"), "name");
        assert_eq!(sw.escape_reserved("async"), "`async`");
    }

    #[test]
    fn test_render_imports_single() {
        let sw = Swift::new();
        let imports = ImportGroup {
            entries: vec![ImportEntry {
                module: "Foundation".into(),
                name: "URL".into(),
                alias: None,
                is_type_only: false,
                is_side_effect: false,
                is_wildcard: false,
            }],
        };
        assert_eq!(sw.render_imports(&imports), "import Foundation");
    }

    #[test]
    fn test_render_imports_grouped() {
        let sw = Swift::new();
        let imports = ImportGroup {
            entries: vec![
                ImportEntry {
                    module: "MyModule".into(),
                    name: "MyType".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "Foundation".into(),
                    name: "URL".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "UIKit".into(),
                    name: "UIViewController".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
            ],
        };
        let output = sw.render_imports(&imports);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines[0], "import Foundation");
        assert_eq!(lines[1], "import UIKit");
        assert_eq!(lines[2], "");
        assert_eq!(lines[3], "import MyModule");
    }

    #[test]
    fn test_render_imports_module_dedup() {
        let sw = Swift::new();
        let imports = ImportGroup {
            entries: vec![
                ImportEntry {
                    module: "Foundation".into(),
                    name: "URL".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "Foundation".into(),
                    name: "Data".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "Foundation".into(),
                    name: "JSONDecoder".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
            ],
        };
        assert_eq!(sw.render_imports(&imports), "import Foundation");
    }

    #[test]
    fn test_doc_comment_single() {
        let sw = Swift::new();
        assert_eq!(
            sw.render_doc_comment(&["A brief description."]),
            "/// A brief description."
        );
    }

    #[test]
    fn test_doc_comment_multi() {
        let sw = Swift::new();
        let doc =
            sw.render_doc_comment(&["Container class.", "", "- Parameter T: the element type"]);
        assert_eq!(
            doc,
            "/// Container class.\n///\n/// - Parameter T: the element type"
        );
    }

    #[test]
    fn test_string_literal() {
        let sw = Swift::new();
        assert_eq!(sw.render_string_literal("hello"), "\"hello\"");
        assert_eq!(sw.render_string_literal("it\"s"), "\"it\\\"s\"");
        assert_eq!(sw.render_string_literal("new\nline"), "\"new\\nline\"");
    }

    #[test]
    fn test_type_keyword() {
        let sw = Swift::new();
        assert_eq!(sw.type_keyword(TypeKind::Class), "class");
        assert_eq!(sw.type_keyword(TypeKind::Struct), "struct");
        assert_eq!(sw.type_keyword(TypeKind::Interface), "protocol");
        assert_eq!(sw.type_keyword(TypeKind::Trait), "protocol");
        assert_eq!(sw.type_keyword(TypeKind::Enum), "enum");
    }

    #[test]
    fn test_visibility() {
        let sw = Swift::new();
        assert_eq!(
            sw.render_visibility(Visibility::Public, DeclarationContext::TopLevel),
            "public "
        );
        assert_eq!(
            sw.render_visibility(Visibility::Private, DeclarationContext::Member),
            "private "
        );
        assert_eq!(
            sw.render_visibility(Visibility::Inherited, DeclarationContext::TopLevel),
            ""
        );
        assert_eq!(
            sw.render_visibility(Visibility::PublicSuper, DeclarationContext::Member),
            "fileprivate "
        );
    }

    #[test]
    fn test_no_semicolons() {
        let sw = Swift::new();
        assert!(!sw.uses_semicolons());
    }

    #[test]
    fn test_return_type_separator() {
        let sw = Swift::new();
        assert_eq!(sw.return_type_separator(), " -> ");
    }

    #[test]
    fn test_field_keywords() {
        let sw = Swift::new();
        assert_eq!(sw.readonly_keyword(), "let ");
        assert_eq!(sw.mutable_field_keyword(), "var ");
    }

    #[test]
    fn test_function_keyword() {
        let sw = Swift::new();
        assert_eq!(sw.function_keyword(DeclarationContext::TopLevel), "func");
        assert_eq!(sw.function_keyword(DeclarationContext::Member), "func");
    }

    #[test]
    fn test_is_apple_framework() {
        assert!(is_apple_framework("Foundation"));
        assert!(is_apple_framework("UIKit"));
        assert!(is_apple_framework("SwiftUI"));
        assert!(is_apple_framework("Combine"));
        assert!(!is_apple_framework("Alamofire"));
        assert!(!is_apple_framework("MyModule"));
    }

    #[test]
    fn test_abstract_keyword_empty() {
        let sw = Swift::new();
        assert_eq!(sw.abstract_keyword(), "");
    }

    #[test]
    fn test_swift_builder_fluent() {
        let sw = Swift::new()
            .with_indent("  ")
            .with_extension("swiftinterface");
        assert_eq!(sw.file_extension(), "swiftinterface");
        assert_eq!(sw.indent_unit(), "  ");
    }
}
