//! PHP language implementation.
//!
//! PHP characteristics:
//! - `<?php` opening tag (not emitted — callers add it via `add("%L", "<?php\n");`)
//! - C-style braces with semicolons
//! - `$`-prefixed variables (use `$$var` in `sigil_quote!`)
//! - `function` keyword, no function keyword needed for `__construct`
//! - `: Type` return type declaration (PHP 7+)
//! - `use Namespace\Class;` imports with `\` separator
//! - `public`/`private`/`protected` visibility
//! - `class`, `interface`, `trait`, `enum` (PHP 8.1+) type keywords
//! - `#[...]` attributes (PHP 8.0+), same syntax as Rust
//! - `/** ... */` PHPDoc comments
//! - `readonly` keyword for immutable properties (PHP 8.1+)
//! - `?Type` nullable type declarations
//! - 4-space indent by PSR-12 convention
//!
//! # Dollar-sign escaping
//!
//! PHP variables use `$` prefixes (`$var`). Since `$` is sigil_quote's
//! interpolation marker, every `$` in PHP code must be written as `$$` inside
//! `sigil_quote!(Php { ... })`:
//!
//! ```text
//! sigil_quote!(Php {
//!     $$name = $S("Alice");
//!     echo $$name;
//! })
//! ```
//!
//! The `$$` escape is handled automatically by the parser and renders as a
//! literal `$` in the output.
//!
//! # Attributes
//!
//! PHP 8 attributes use `#[...]` syntax, identical to Rust. Use `$attr(...)`:
//!
//! ```text
//! sigil_quote!(Php {
//!     $attr("Override")
//!     public function toString(): string { return $$this->name; }
//! })
//! ```

use crate::import::ImportGroup;
use crate::lang::config::{
    BlockSyntaxConfig, EnumAndAnnotationConfig, FunctionSyntaxConfig, OptionalFieldStyle,
    TypeDeclSyntaxConfig, TypePresentationConfig,
};
use crate::lang::{CodeLang, RendererLang};
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};
use crate::type_name::{AssociatedTypeStyle, TypePresentation};

/// PHP language implementation.
///
/// Configurable indent (default: 4 spaces per PSR-12) and file extension
/// (default: `"php"`).
#[derive(Debug, Clone)]
pub struct Php {
    /// Indent with this string (default: `"    "` — 4 spaces).
    pub indent: String,
    /// File extension (default: `"php"`).
    pub extension: String,
}

impl Default for Php {
    fn default() -> Self {
        Self {
            indent: "    ".to_string(),
            extension: "php".to_string(),
        }
    }
}

impl Php {
    /// Create a new PHP language instance with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the indent string (e.g., `"    "` for 4-space, `"\t"` for tabs).
    pub fn with_indent(mut self, s: &str) -> Self {
        self.indent = s.to_string();
        self
    }

    /// Set the file extension (default: `"php"`).
    pub fn with_extension(mut self, s: &str) -> Self {
        self.extension = s.to_string();
        self
    }
}

#[rustfmt::skip]
const PHP_RESERVED: &[&str] = &[
    // Strict keywords
    "abstract", "and", "array", "as", "break", "callable", "case", "catch",
    "class", "clone", "const", "continue", "declare", "default", "die",
    "do", "echo", "else", "elseif", "empty", "enddeclare", "endfor",
    "endforeach", "endif", "endswitch", "endwhile", "eval", "exit", "extends",
    "final", "finally", "fn", "for", "foreach", "function", "global",
    "goto", "if", "implements", "include", "include_once", "instanceof",
    "insteadof", "interface", "isset", "list", "match", "namespace", "new",
    "or", "print", "private", "protected", "public", "readonly", "require",
    "require_once", "return", "static", "switch", "throw", "trait", "try",
    "unset", "use", "var", "while", "xor", "yield",
    // Soft reserved (type names)
    "int", "float", "bool", "string", "true", "false", "null", "void",
    "never", "mixed", "iterable", "self", "parent", "enum",
    "from",
    // Compile-time constants
    "__class__", "__dir__", "__file__", "__function__", "__line__",
    "__method__", "__namespace__", "__trait__",
];

impl RendererLang for Php {
    fn file_extension(&self) -> &str {
        &self.extension
    }

    fn line_comment_prefix(&self) -> &str {
        "//"
    }

    fn reserved_words(&self) -> &[&str] {
        PHP_RESERVED
    }

    fn module_separator(&self) -> Option<&str> {
        Some("\\")
    }

    fn render_string_literal(&self, s: &str) -> String {
        format!(
            "\"{}\"",
            s.replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('$', "\\$")
                .replace('\n', "\\n")
                .replace('\t', "\\t")
                .replace('\r', "\\r")
                .replace('\0', "\\0")
        )
    }

    fn render_verbatim_string(&self, s: &str) -> String {
        let escaped = s
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('$', "\\$");
        format!("\"{escaped}\"")
    }

    fn render_attribute(&self, text: &str) -> String {
        format!("#[{text}]")
    }

    fn block_syntax(&self) -> BlockSyntaxConfig<'_> {
        BlockSyntaxConfig {
            indent_unit: &self.indent,
            field_terminator: ";",
            ..Default::default()
        }
    }

    fn type_presentation(&self) -> TypePresentationConfig<'_> {
        TypePresentationConfig {
            array: TypePresentation::GenericWrap { name: "array" },
            readonly_array: Some(TypePresentation::GenericWrap { name: "array" }),
            optional: TypePresentation::Prefix { prefix: "?" },
            optional_absent_literal: "null",
            associated_type: AssociatedTypeStyle::DotAccess,
            ..Default::default()
        }
    }
}

impl CodeLang for Php {
    fn variable_prefix(&self) -> &str {
        "$"
    }

    fn render_imports(&self, imports: &ImportGroup) -> String {
        if imports.entries().is_empty() {
            return String::new();
        }

        let mut seen = std::collections::BTreeSet::new();
        let mut lines = Vec::new();

        for entry in imports.entries() {
            // PHP has no side-effect or wildcard imports.
            if entry.is_side_effect || entry.is_wildcard {
                continue;
            }

            let fqn = if entry.module.is_empty() {
                entry.name.clone()
            } else {
                format!("{}\\{}", entry.module, entry.name)
            };

            if !seen.insert(fqn.clone()) {
                continue;
            }

            lines.push(if let Some(alias) = &entry.alias {
                format!("use {} as {};", fqn, alias)
            } else {
                format!("use {};", fqn)
            });
        }

        lines.sort();
        lines.join("\n")
    }

    fn render_doc_comment(&self, lines: &[&str]) -> String {
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

    fn render_visibility(&self, vis: Visibility, ctx: DeclarationContext) -> &str {
        match ctx {
            DeclarationContext::TopLevel => "",
            DeclarationContext::Member => match vis {
                Visibility::Public => "public ",
                Visibility::Private => "private ",
                Visibility::Protected => "protected ",
                _ => "public ",
            },
            DeclarationContext::InterfaceMember => match vis {
                Visibility::Public => "public ",
                _ => "",
            },
        }
    }

    fn function_keyword(&self, _ctx: DeclarationContext) -> &str {
        "function"
    }

    fn type_keyword(&self, kind: TypeKind) -> &str {
        match kind {
            TypeKind::Class | TypeKind::Struct => "class",
            TypeKind::Interface => "interface",
            TypeKind::Trait => "trait",
            TypeKind::Enum => "enum",
            TypeKind::TypeAlias | TypeKind::Newtype => "class",
        }
    }

    fn methods_inside_type_body(&self, _kind: TypeKind) -> bool {
        true
    }

    fn optional_field_style(&self) -> OptionalFieldStyle {
        OptionalFieldStyle::TypePrefix("?")
    }

    fn function_syntax(&self) -> FunctionSyntaxConfig<'_> {
        FunctionSyntaxConfig {
            return_type_separator: ": ",
            async_keyword: "",
            abstract_keyword: "abstract ",
            override_keyword: "",
            override_annotation: "#[Override]",
            static_keyword: "static ",
            ..Default::default()
        }
    }

    fn type_decl_syntax(&self) -> TypeDeclSyntaxConfig<'_> {
        TypeDeclSyntaxConfig {
            type_before_name: true,
            return_type_is_prefix: false,
            super_type_keyword: " extends ",
            implements_keyword: " implements ",
            ..Default::default()
        }
    }

    fn enum_and_annotation(&self) -> EnumAndAnnotationConfig<'_> {
        EnumAndAnnotationConfig {
            annotation_prefix: "#[",
            annotation_suffix: "]",
            readonly_keyword: "readonly ",
            ..Default::default()
        }
    }

    fn render_newtype_line(&self, vis: &str, name: &str, inner: &str) -> String {
        format!("{vis}class {name} {{ public function __construct(private {inner} $value) {{}} }}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import::ImportEntry;

    #[test]
    fn test_file_extension() {
        let php = Php::new();
        assert_eq!(php.file_extension(), "php");
    }

    #[test]
    fn test_line_comment() {
        let php = Php::new();
        assert_eq!(php.line_comment_prefix(), "//");
    }

    #[test]
    fn test_escape_reserved() {
        let php = Php::new();
        assert_eq!(php.escape_reserved("class"), "class_");
        assert_eq!(php.escape_reserved("function"), "function_");
        assert_eq!(php.escape_reserved("echo"), "echo_");
        assert_eq!(php.escape_reserved("name"), "name");
    }

    #[test]
    fn test_render_string_literal() {
        let php = Php::new();
        assert_eq!(php.render_string_literal("hello"), "\"hello\"");
        assert_eq!(
            php.render_string_literal("say \"hi\""),
            "\"say \\\"hi\\\"\""
        );
        assert_eq!(php.render_string_literal("$var"), "\"\\$var\"");
        assert_eq!(php.render_string_literal("new\nline"), "\"new\\nline\"");
    }

    #[test]
    fn test_render_verbatim_string_escapes_dollar() {
        let php = Php::new();
        let out = php.render_verbatim_string("Hello $name");
        assert_eq!(out, "\"Hello \\$name\"");
    }

    #[test]
    fn test_render_attribute() {
        let php = Php::new();
        assert_eq!(php.render_attribute("Override"), "#[Override]");
    }

    #[test]
    fn test_block_syntax() {
        let php = Php::new();
        let bs = php.block_syntax();
        assert_eq!(bs.block_open, " {");
        assert_eq!(bs.block_close, "}");
        assert!(bs.uses_semicolons);
        assert_eq!(bs.indent_unit, "    ");
    }

    #[test]
    fn test_module_separator() {
        let php = Php::new();
        assert_eq!(php.module_separator(), Some("\\"));
    }

    #[test]
    fn test_render_doc_comment() {
        let php = Php::new();
        assert_eq!(
            php.render_doc_comment(&["A brief description."]),
            "/**\n * A brief description.\n */"
        );
    }

    #[test]
    fn test_render_doc_comment_multi() {
        let php = Php::new();
        let doc = php.render_doc_comment(&["Container class.", "", "@param T $value"]);
        assert_eq!(doc, "/**\n * Container class.\n *\n * @param T $value\n */");
    }

    #[test]
    fn test_render_imports_single() {
        let php = Php::new();
        let imports = ImportGroup {
            entries: vec![ImportEntry {
                module: "App\\Models".into(),
                name: "User".into(),
                alias: None,
                is_type_only: false,
                is_side_effect: false,
                is_wildcard: false,
            }],
        };
        assert_eq!(php.render_imports(&imports), "use App\\Models\\User;");
    }

    #[test]
    fn test_render_imports_sorted() {
        let php = Php::new();
        let imports = ImportGroup {
            entries: vec![
                ImportEntry {
                    module: "App\\Models".into(),
                    name: "User".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "App\\Http".into(),
                    name: "Controller".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
            ],
        };
        let output = php.render_imports(&imports);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines[0], "use App\\Http\\Controller;");
        assert_eq!(lines[1], "use App\\Models\\User;");
    }

    #[test]
    fn test_render_imports_dedup() {
        let php = Php::new();
        let imports = ImportGroup {
            entries: vec![
                ImportEntry {
                    module: "App\\Models".into(),
                    name: "User".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "App\\Models".into(),
                    name: "User".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
            ],
        };
        assert_eq!(php.render_imports(&imports), "use App\\Models\\User;");
    }

    #[test]
    fn test_render_imports_with_alias() {
        let php = Php::new();
        let imports = ImportGroup {
            entries: vec![ImportEntry {
                module: "App\\Models".into(),
                name: "User".into(),
                alias: Some("UserModel".into()),
                is_type_only: false,
                is_side_effect: false,
                is_wildcard: false,
            }],
        };
        assert_eq!(
            php.render_imports(&imports),
            "use App\\Models\\User as UserModel;"
        );
    }

    #[test]
    fn test_visibility_member() {
        let php = Php::new();
        assert_eq!(
            php.render_visibility(Visibility::Public, DeclarationContext::Member),
            "public "
        );
        assert_eq!(
            php.render_visibility(Visibility::Private, DeclarationContext::Member),
            "private "
        );
        assert_eq!(
            php.render_visibility(Visibility::Protected, DeclarationContext::Member),
            "protected "
        );
    }

    #[test]
    fn test_visibility_top_level() {
        let php = Php::new();
        assert_eq!(
            php.render_visibility(Visibility::Public, DeclarationContext::TopLevel),
            ""
        );
    }

    #[test]
    fn test_visibility_interface_member() {
        let php = Php::new();
        assert_eq!(
            php.render_visibility(Visibility::Public, DeclarationContext::InterfaceMember),
            "public "
        );
        assert_eq!(
            php.render_visibility(Visibility::Private, DeclarationContext::InterfaceMember),
            ""
        );
    }

    #[test]
    fn test_function_keyword() {
        let php = Php::new();
        assert_eq!(
            php.function_keyword(DeclarationContext::TopLevel),
            "function"
        );
    }

    #[test]
    fn test_type_keyword() {
        let php = Php::new();
        assert_eq!(php.type_keyword(TypeKind::Class), "class");
        assert_eq!(php.type_keyword(TypeKind::Struct), "class");
        assert_eq!(php.type_keyword(TypeKind::Interface), "interface");
        assert_eq!(php.type_keyword(TypeKind::Trait), "trait");
        assert_eq!(php.type_keyword(TypeKind::Enum), "enum");
    }

    #[test]
    fn test_type_before_name() {
        let php = Php::new();
        assert!(php.type_decl_syntax().type_before_name);
    }

    #[test]
    fn test_return_type_is_prefix() {
        let php = Php::new();
        assert!(!php.type_decl_syntax().return_type_is_prefix);
    }

    #[test]
    fn test_return_type_separator() {
        let php = Php::new();
        assert_eq!(php.function_syntax().return_type_separator, ": ");
    }

    #[test]
    fn test_readonly_keyword() {
        let php = Php::new();
        assert_eq!(php.enum_and_annotation().readonly_keyword, "readonly ");
    }

    #[test]
    fn test_abstract_keyword() {
        let php = Php::new();
        assert_eq!(php.function_syntax().abstract_keyword, "abstract ");
    }

    #[test]
    fn test_static_keyword() {
        let php = Php::new();
        assert_eq!(php.function_syntax().static_keyword, "static ");
    }

    #[test]
    fn test_optional_field_style() {
        let php = Php::new();
        assert!(matches!(
            php.optional_field_style(),
            OptionalFieldStyle::TypePrefix("?")
        ));
    }

    #[test]
    fn test_render_newtype_line() {
        let php = Php::new();
        let line = php.render_newtype_line("", "Name", "string");
        assert!(line.contains("class Name"));
        assert!(line.contains("__construct"));
        assert!(line.contains("private string $value"));
    }

    #[test]
    fn test_builder_fluent() {
        let php = Php::new().with_indent("\t").with_extension("phtml");
        assert_eq!(php.file_extension(), "phtml");
        assert_eq!(php.block_syntax().indent_unit, "\t");
    }

    #[test]
    fn test_annotation_config() {
        let php = Php::new();
        let ea = php.enum_and_annotation();
        assert_eq!(ea.annotation_prefix, "#[");
        assert_eq!(ea.annotation_suffix, "]");
    }
}
