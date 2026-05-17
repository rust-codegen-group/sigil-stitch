//! JavaScript language implementation.

use crate::import::{ImportEntry, ImportGroup};
use crate::lang::CodeLang;
use crate::lang::config::{
    BlockSyntaxConfig, EnumAndAnnotationConfig, FunctionSyntaxConfig, GenericSyntaxConfig,
    QuoteStyle, TypeDeclSyntaxConfig, TypePresentationConfig,
};
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};
use crate::type_name::TypePresentation;

/// JavaScript language implementation.
///
/// JavaScript-specific behaviors:
/// - `import { X } from 'module'` (ESM) — no `import type`
/// - `/** ... */` JSDoc doc comments
/// - `export` for top-level visibility, no member visibility keywords
/// - `class` keyword only (no `interface`, no `abstract`)
/// - No type annotations on parameters, fields, or return types
/// - Configurable semicolons and quote style
///
/// # Type annotations
///
/// JavaScript has no type annotations. Use `TypeName::primitive("")` for
/// parameter and field types, and don't call `.returns()` on `FunSpecBuilder`:
/// ```text
/// // Parameter without type annotation:
/// ParameterSpec::new("name", TypeName::primitive(""))
///
/// // Field without type annotation:
/// FieldSpec::builder("count", TypeName::primitive("")).build()
///
/// // Function without return type — just don't call .returns():
/// let mut fb = FunSpec::builder("greet");
/// fb.add_param(ParameterSpec::new("name", TypeName::primitive("")));
/// fb.body(body);
/// ```
///
/// # Private fields
///
/// ES2022 private fields use `#` prefix. Name the field directly:
/// ```text
/// FieldSpec::builder("#count", TypeName::primitive("")).build()
/// ```
#[derive(Debug, Clone)]
pub struct JavaScript {
    /// Quote style for string literals and import paths.
    pub quote_style: QuoteStyle,
    /// Indent with this string (default: "  ").
    pub indent: String,
    /// File extension (default: "js"). Set to "mjs" or "cjs" for explicit module type.
    pub extension: String,
    /// Whether to emit semicolons (default: true).
    pub semicolons: bool,
}

impl Default for JavaScript {
    fn default() -> Self {
        Self {
            quote_style: QuoteStyle::Single,
            indent: "  ".to_string(),
            extension: "js".to_string(),
            semicolons: true,
        }
    }
}

impl JavaScript {
    /// Create a new JavaScript language instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a JavaScript configured for ES modules (.mjs extension).
    pub fn esm() -> Self {
        Self {
            extension: "mjs".to_string(),
            ..Self::default()
        }
    }

    /// Create a JavaScript configured for CommonJS modules (.cjs extension).
    pub fn cjs() -> Self {
        Self {
            extension: "cjs".to_string(),
            ..Self::default()
        }
    }

    /// Set the quote style used for string literals and import paths.
    pub fn with_quote_style(mut self, qs: QuoteStyle) -> Self {
        self.quote_style = qs;
        self
    }

    /// Set the indent string (e.g., `"  "`, `"    "`, `"\t"`).
    pub fn with_indent(mut self, s: &str) -> Self {
        self.indent = s.to_string();
        self
    }

    /// Control whether statements are terminated with `;`.
    pub fn with_semicolons(mut self, b: bool) -> Self {
        self.semicolons = b;
        self
    }

    /// Set the file extension (e.g., `"js"`, `"mjs"`, `"cjs"`).
    pub fn with_extension(mut self, s: &str) -> Self {
        self.extension = s.to_string();
        self
    }
}

#[rustfmt::skip]
const JS_RESERVED: &[&str] = &[
    // ECMAScript reserved words
    "break", "case", "catch", "class", "const", "continue", "debugger",
    "default", "delete", "do", "else", "enum", "export", "extends", "false",
    "finally", "for", "function", "if", "import", "in", "instanceof", "new",
    "null", "return", "super", "switch", "this", "throw", "true", "try",
    "typeof", "var", "void", "while", "with",
    // Strict-mode reserved words
    "implements", "interface", "let", "package", "private", "protected",
    "public", "static", "yield",
    // Async/await (ES2017+)
    "async", "await",
];

impl CodeLang for JavaScript {
    fn file_extension(&self) -> &str {
        &self.extension
    }

    fn reserved_words(&self) -> &[&str] {
        JS_RESERVED
    }

    fn render_imports(&self, imports: &ImportGroup) -> String {
        let mut lines = Vec::new();
        let quote = self.quote_style.char();
        let semi = if self.semicolons { ";" } else { "" };

        // Group entries by module path.
        let mut by_module: std::collections::BTreeMap<&str, Vec<&ImportEntry>> =
            std::collections::BTreeMap::new();
        for entry in imports.entries() {
            if entry.is_side_effect {
                lines.push(format!("import {quote}{}{quote}{semi}", entry.module));
                continue;
            }
            if entry.is_wildcard {
                let alias = super::module_to_alias(&entry.module);
                lines.push(format!(
                    "import * as {} from {quote}{}{quote}{semi}",
                    alias, entry.module,
                ));
                continue;
            }
            by_module.entry(&entry.module).or_default().push(entry);
        }

        for (module, entries) in &by_module {
            // JavaScript has no `import type` — all entries are value imports.
            let mut names: Vec<String> = Vec::new();

            for entry in entries {
                let spec = if let Some(alias) = &entry.alias {
                    format!("{} as {}", entry.name, alias)
                } else {
                    entry.name.clone()
                };
                names.push(spec);
            }

            names.sort();

            if !names.is_empty() {
                lines.push(format!(
                    "import {{ {} }} from {quote}{}{quote}{semi}",
                    names.join(", "),
                    module,
                ));
            }
        }

        lines.join("\n")
    }

    fn render_string_literal(&self, s: &str) -> String {
        match self.quote_style {
            QuoteStyle::Single => {
                format!("'{}'", s.replace('\\', "\\\\").replace('\'', "\\'"))
            }
            QuoteStyle::Double => {
                format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
            }
        }
    }

    fn render_doc_comment(&self, lines: &[&str]) -> String {
        if lines.is_empty() {
            return String::new();
        }
        let mut out = String::from("/**\n");
        for line in lines {
            if line.is_empty() {
                out.push_str(" *\n");
            } else {
                out.push_str(&format!(" * {line}\n"));
            }
        }
        out.push_str(" */");
        out
    }

    fn line_comment_prefix(&self) -> &str {
        "//"
    }

    fn render_visibility(&self, vis: Visibility, ctx: DeclarationContext) -> &str {
        match ctx {
            DeclarationContext::TopLevel => match vis {
                Visibility::Public => "export ",
                _ => "",
            },
            // JavaScript has no member visibility keywords.
            // Use #name convention for private fields.
            DeclarationContext::Member | DeclarationContext::InterfaceMember => "",
        }
    }

    fn function_keyword(&self, ctx: DeclarationContext) -> &str {
        match ctx {
            DeclarationContext::TopLevel => "function",
            DeclarationContext::Member | DeclarationContext::InterfaceMember => "",
        }
    }

    fn type_keyword(&self, kind: TypeKind) -> &str {
        match kind {
            TypeKind::Class | TypeKind::Struct => "class",
            TypeKind::Interface | TypeKind::Trait => "class",
            TypeKind::Enum => "class",
            TypeKind::TypeAlias | TypeKind::Newtype => "class",
        }
    }

    fn methods_inside_type_body(&self, _kind: TypeKind) -> bool {
        true
    }

    // --- Config struct accessors ---

    fn type_presentation(&self) -> TypePresentationConfig<'_> {
        TypePresentationConfig {
            tuple: TypePresentation::Delimited {
                open: "[",
                sep: ", ",
                close: "]",
            },
            ..Default::default()
        }
    }

    fn generic_syntax(&self) -> GenericSyntaxConfig<'_> {
        GenericSyntaxConfig {
            constraint_keyword: "",
            constraint_separator: "",
            context_bound_keyword: "",
            ..Default::default()
        }
    }

    fn block_syntax(&self) -> BlockSyntaxConfig<'_> {
        BlockSyntaxConfig {
            indent_unit: &self.indent,
            uses_semicolons: self.semicolons,
            field_terminator: ";",
            ..Default::default()
        }
    }

    fn function_syntax(&self) -> FunctionSyntaxConfig<'_> {
        FunctionSyntaxConfig {
            abstract_keyword: "",
            ..Default::default()
        }
    }

    fn type_decl_syntax(&self) -> TypeDeclSyntaxConfig<'_> {
        TypeDeclSyntaxConfig {
            super_type_keyword: " extends ",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_extension() {
        let js = JavaScript::new();
        assert_eq!(js.file_extension(), "js");
    }

    #[test]
    fn test_esm_extension() {
        let js = JavaScript::esm();
        assert_eq!(js.file_extension(), "mjs");
    }

    #[test]
    fn test_cjs_extension() {
        let js = JavaScript::cjs();
        assert_eq!(js.file_extension(), "cjs");
    }

    #[test]
    fn test_escape_reserved() {
        let js = JavaScript::new();
        assert_eq!(js.escape_reserved("class"), "class_");
        assert_eq!(js.escape_reserved("async"), "async_");
        assert_eq!(js.escape_reserved("yield"), "yield_");
        assert_eq!(js.escape_reserved("myVar"), "myVar");
        // TS-specific keywords are NOT reserved in JS.
        assert_eq!(js.escape_reserved("type"), "type");
        assert_eq!(js.escape_reserved("interface"), "interface_");
        assert_eq!(js.escape_reserved("any"), "any");
        assert_eq!(js.escape_reserved("string"), "string");
    }

    #[test]
    fn test_render_imports_basic() {
        let js = JavaScript::new();
        let imports = ImportGroup {
            entries: vec![ImportEntry {
                module: "./utils".into(),
                name: "formatDate".into(),
                alias: None,
                is_type_only: false,
                is_side_effect: false,
                is_wildcard: false,
            }],
        };
        assert_eq!(
            js.render_imports(&imports),
            "import { formatDate } from './utils';"
        );
    }

    #[test]
    fn test_render_imports_no_import_type() {
        let js = JavaScript::new();
        let imports = ImportGroup {
            entries: vec![
                ImportEntry {
                    module: "./models".into(),
                    name: "User".into(),
                    alias: None,
                    is_type_only: true, // Should be ignored in JS.
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "./models".into(),
                    name: "createUser".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
            ],
        };
        let output = js.render_imports(&imports);
        // Both should be in a single import statement, no `import type`.
        assert_eq!(output, "import { User, createUser } from './models';");
        assert!(!output.contains("import type"));
    }

    #[test]
    fn test_render_imports_with_alias() {
        let js = JavaScript::new();
        let imports = ImportGroup {
            entries: vec![ImportEntry {
                module: "./other".into(),
                name: "User".into(),
                alias: Some("OtherUser".into()),
                is_type_only: false,
                is_side_effect: false,
                is_wildcard: false,
            }],
        };
        assert_eq!(
            js.render_imports(&imports),
            "import { User as OtherUser } from './other';"
        );
    }

    #[test]
    fn test_render_imports_multiple_modules() {
        let js = JavaScript::new();
        let imports = ImportGroup {
            entries: vec![
                ImportEntry {
                    module: "./models".into(),
                    name: "User".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "./utils".into(),
                    name: "format".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
            ],
        };
        let output = js.render_imports(&imports);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "import { User } from './models';");
        assert_eq!(lines[1], "import { format } from './utils';");
    }

    #[test]
    fn test_render_imports_no_semicolons() {
        let js = JavaScript {
            semicolons: false,
            ..Default::default()
        };
        let imports = ImportGroup {
            entries: vec![ImportEntry {
                module: "./utils".into(),
                name: "format".into(),
                alias: None,
                is_type_only: false,
                is_side_effect: false,
                is_wildcard: false,
            }],
        };
        assert_eq!(
            js.render_imports(&imports),
            "import { format } from './utils'"
        );
    }

    #[test]
    fn test_doc_comment_single() {
        let js = JavaScript::new();
        let doc = js.render_doc_comment(&["A brief description."]);
        assert!(doc.starts_with("/**\n"));
        assert!(doc.contains(" * A brief description.\n"));
        assert!(doc.ends_with(" */"));
    }

    #[test]
    fn test_doc_comment_multi() {
        let js = JavaScript::new();
        let doc = js.render_doc_comment(&["Get user.", "", "Returns null if not found."]);
        assert!(doc.contains(" * Get user.\n"));
        assert!(doc.contains(" *\n"));
        assert!(doc.contains(" * Returns null if not found.\n"));
    }

    #[test]
    fn test_string_literal_single_quotes() {
        let js = JavaScript::new();
        assert_eq!(js.render_string_literal("hello"), "'hello'");
        assert_eq!(js.render_string_literal("it's"), "'it\\'s'");
    }

    #[test]
    fn test_string_literal_double_quotes() {
        let js = JavaScript::new().with_quote_style(QuoteStyle::Double);
        assert_eq!(js.render_string_literal("hello"), "\"hello\"");
    }

    #[test]
    fn test_javascript_builder_fluent() {
        let js = JavaScript::new()
            .with_semicolons(false)
            .with_quote_style(QuoteStyle::Double)
            .with_extension("mjs")
            .with_indent("    ");
        assert!(!js.block_syntax().uses_semicolons);
        assert_eq!(js.file_extension(), "mjs");
        assert_eq!(js.block_syntax().indent_unit, "    ");
        assert_eq!(js.render_string_literal("hi"), "\"hi\"");
    }

    #[test]
    fn test_visibility_top_level() {
        let js = JavaScript::new();
        assert_eq!(
            js.render_visibility(Visibility::Public, DeclarationContext::TopLevel),
            "export "
        );
        assert_eq!(
            js.render_visibility(Visibility::Private, DeclarationContext::TopLevel),
            ""
        );
    }

    #[test]
    fn test_visibility_member() {
        let js = JavaScript::new();
        // JS has no member visibility keywords.
        assert_eq!(
            js.render_visibility(Visibility::Public, DeclarationContext::Member),
            ""
        );
        assert_eq!(
            js.render_visibility(Visibility::Private, DeclarationContext::Member),
            ""
        );
        assert_eq!(
            js.render_visibility(Visibility::Protected, DeclarationContext::Member),
            ""
        );
    }

    #[test]
    fn test_no_abstract() {
        let js = JavaScript::new();
        assert_eq!(js.function_syntax().abstract_keyword, "");
    }

    #[test]
    fn test_no_implements() {
        let js = JavaScript::new();
        assert_eq!(js.type_decl_syntax().implements_keyword, "");
    }

    #[test]
    fn test_type_keyword() {
        let js = JavaScript::new();
        assert_eq!(js.type_keyword(TypeKind::Class), "class");
        assert_eq!(js.type_keyword(TypeKind::Struct), "class");
        assert_eq!(js.type_keyword(TypeKind::Interface), "class");
        assert_eq!(js.type_keyword(TypeKind::Enum), "class");
    }

    #[test]
    fn test_module_separator() {
        let js = JavaScript::new();
        assert_eq!(js.module_separator(), None);
    }
}
