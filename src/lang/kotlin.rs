//! Kotlin language implementation.

use crate::import::ImportGroup;
use crate::lang::CodeLang;
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};

/// Kotlin language implementation.
///
/// Kotlin-specific behaviors:
/// - Name-before-type declarations (`count: Int`, not `Int count`)
/// - `fun` function keyword, `suspend` for coroutines
/// - `import pkg.Class` with kotlin/java/third-party grouping (no semicolons)
/// - No semicolons
/// - `class`, `data class`, `enum class`, `interface` keywords
/// - Single `:` for both extends and implements
/// - Generic bounds via `:` and `,` (`<T : Comparable<T>>`)
/// - `/** ... */` KDoc comments
/// - `val`/`var` for readonly/mutable properties
/// - Backtick escaping for reserved words
/// - Annotations (`@JvmStatic`, `@Override`) via `annotation()`
///
/// # Import conventions
///
/// Use [`crate::type_name::TypeName::importable`] with the package as module and class name as name:
/// ```text
/// TypeName::importable("kotlin.collections", "List")       // import kotlin.collections.List
/// TypeName::importable("java.util", "UUID")                // import java.util.UUID
/// TypeName::importable("com.example.model", "User")        // import com.example.model.User
/// ```
///
/// # Inheritance
///
/// Kotlin uses `:` for both superclass and interfaces. Put all supertypes
/// into `super_types()` (not `impl_types()`):
/// ```text
/// tb.super_type(TypeName::primitive("Base"));
/// tb.super_type(TypeName::primitive("Serializable"));
/// // Emits: class Foo : Base, Serializable {
/// ```
///
/// # `sealed class` / `object`
///
/// Use annotations for modifier-like keywords:
/// ```text
/// tb.annotation(CodeBlock::of("sealed", ()).unwrap());
/// // Combined with TypeKind::Class → "sealed\nclass Foo {"
/// ```
///
/// # Primary constructors
///
/// Use `add_primary_constructor_param()` on `TypeSpecBuilder`:
/// ```text
/// let mut tb = TypeSpec::builder("Person", TypeKind::Class);
/// tb.add_primary_constructor_param(ParameterSpec::new("val name", TypeName::primitive("String")));
/// tb.add_primary_constructor_param(ParameterSpec::new("val age", TypeName::primitive("Int")));
/// // Emits: class Person(val name: String, val age: Int) {
/// ```
///
/// # Secondary constructor delegation
///
/// Use `delegation()` on `FunSpecBuilder` for `super(...)` / `this(...)` calls.
/// Kotlin places delegation in the signature (after params, before body):
/// ```text
/// let mut ctor = FunSpec::builder("constructor");
/// ctor.is_constructor();
/// ctor.add_param(ParameterSpec::new("name", TypeName::primitive("String")));
/// ctor.delegation(CodeBlock::of("this(name, 0)", ()).unwrap());
/// ctor.body(CodeBlock::of("// secondary", ()).unwrap());
/// // Emits: constructor(name: String) : this(name, 0) { ... }
/// ```
#[derive(Debug, Clone)]
pub struct Kotlin {
    /// Indent with this string (default: "    " — 4 spaces).
    pub indent: String,
    /// File extension (default: "kt"). Set to "kts" for Kotlin script files.
    pub extension: String,
}

impl Default for Kotlin {
    fn default() -> Self {
        Self {
            indent: "    ".to_string(),
            extension: "kt".to_string(),
        }
    }
}

impl Kotlin {
    /// Create a new Kotlin language instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the indent string (e.g., `"    "` for 4-space default, `"\t"` for tabs).
    pub fn with_indent(mut self, s: &str) -> Self {
        self.indent = s.to_string();
        self
    }

    /// Set the file extension (e.g., `"kt"` or `"kts"` for scripts).
    pub fn with_extension(mut self, s: &str) -> Self {
        self.extension = s.to_string();
        self
    }
}

#[rustfmt::skip]
const KOTLIN_RESERVED: &[&str] = &[
    // Hard keywords
    "as", "break", "class", "continue", "do", "else", "false", "for",
    "fun", "if", "in", "interface", "is", "null", "object", "package",
    "return", "super", "this", "throw", "true", "try", "typealias",
    "typeof", "val", "var", "when", "while",
    // Soft keywords (contextually reserved)
    "by", "catch", "constructor", "delegate", "dynamic", "field",
    "file", "finally", "get", "import", "init", "param", "property",
    "receiver", "set", "setparam", "where",
    // Modifier keywords
    "abstract", "actual", "annotation", "companion", "const",
    "crossinline", "data", "enum", "expect", "external", "final",
    "infix", "inline", "inner", "internal", "lateinit", "noinline",
    "open", "operator", "out", "override", "private", "protected",
    "public", "reified", "sealed", "suspend", "tailrec", "vararg",
];

/// Classify an import module into a group for ordering.
/// 0 = kotlin.*, 1 = kotlinx.*, 2 = java.*, 3 = javax.*, 4 = everything else.
fn import_group_order(module: &str) -> u8 {
    if module.starts_with("kotlin.") || module == "kotlin" {
        0
    } else if module.starts_with("kotlinx.") || module == "kotlinx" {
        1
    } else if module.starts_with("java.") {
        2
    } else if module.starts_with("javax.") {
        3
    } else {
        4
    }
}

impl CodeLang for Kotlin {
    fn file_extension(&self) -> &str {
        &self.extension
    }

    fn reserved_words(&self) -> &[&str] {
        KOTLIN_RESERVED
    }

    fn escape_reserved(&self, name: &str) -> String {
        if self.reserved_words().contains(&name) {
            format!("`{name}`")
        } else {
            name.to_string()
        }
    }

    fn render_imports(&self, imports: &ImportGroup) -> String {
        if imports.entries().is_empty() {
            return String::new();
        }

        // Collect unique fully-qualified imports, grouped by category.
        let mut kotlin_imports: Vec<String> = Vec::new();
        let mut kotlinx_imports: Vec<String> = Vec::new();
        let mut java_imports: Vec<String> = Vec::new();
        let mut javax_imports: Vec<String> = Vec::new();
        let mut other_imports: Vec<String> = Vec::new();

        let mut seen = std::collections::BTreeSet::new();
        for entry in imports.entries() {
            let line = if entry.is_wildcard {
                let fqn = format!("{}.*", entry.module);
                if !seen.insert(fqn.clone()) {
                    continue;
                }
                format!("import {fqn}")
            } else if entry.is_side_effect {
                // Kotlin has no side-effect imports; skip.
                continue;
            } else {
                let fqn = format!("{}.{}", entry.module, entry.name);
                if !seen.insert(fqn.clone()) {
                    continue;
                }
                format!("import {fqn}")
            };

            match import_group_order(&entry.module) {
                0 => kotlin_imports.push(line),
                1 => kotlinx_imports.push(line),
                2 => java_imports.push(line),
                3 => javax_imports.push(line),
                _ => other_imports.push(line),
            }
        }

        kotlin_imports.sort();
        kotlinx_imports.sort();
        java_imports.sort();
        javax_imports.sort();
        other_imports.sort();

        let groups: Vec<&Vec<String>> = [
            &kotlin_imports,
            &kotlinx_imports,
            &java_imports,
            &javax_imports,
            &other_imports,
        ]
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
                .replace('$', "\\$")
        )
    }

    fn render_doc_comment(&self, lines: &[&str]) -> String {
        // KDoc uses /** ... */ style (same as Javadoc).
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

    fn render_visibility(&self, vis: Visibility, ctx: DeclarationContext) -> &str {
        match ctx {
            DeclarationContext::TopLevel => match vis {
                Visibility::Public => "", // public is default in Kotlin
                Visibility::Private => "private ",
                Visibility::Protected => "protected ",
                _ => "internal ",
            },
            DeclarationContext::Member => match vis {
                Visibility::Public => "", // public is default
                Visibility::Private => "private ",
                Visibility::Protected => "protected ",
                _ => "internal ",
            },
            // Interface members are implicitly public in Kotlin — no visibility modifier.
            DeclarationContext::InterfaceMember => "",
        }
    }

    fn function_keyword(&self, _ctx: DeclarationContext) -> &str {
        "fun"
    }

    fn type_keyword(&self, kind: TypeKind) -> &str {
        match kind {
            TypeKind::Class => "class",
            TypeKind::Struct => "data class",
            TypeKind::Interface | TypeKind::Trait => "interface",
            TypeKind::Enum => "enum class",
            TypeKind::TypeAlias => "typealias",
            TypeKind::Newtype => "value class",
        }
    }

    fn methods_inside_type_body(&self, _kind: TypeKind) -> bool {
        true
    }

    fn render_newtype_line(&self, vis: &str, name: &str, inner: &str) -> String {
        format!("{vis}value class {name}(val value: {inner})")
    }

    fn property_style(&self) -> crate::spec::modifiers::PropertyStyle {
        crate::spec::modifiers::PropertyStyle::Field
    }

    fn property_getter_keyword(&self) -> &str {
        "get()"
    }

    fn optional_field_style(&self) -> crate::lang::config::OptionalFieldStyle {
        crate::lang::config::OptionalFieldStyle::TypeSuffix("?")
    }

    fn type_presentation(&self) -> crate::lang::config::TypePresentationConfig<'_> {
        crate::lang::config::TypePresentationConfig {
            array: crate::type_name::TypePresentation::GenericWrap { name: "List" },
            readonly_array: Some(crate::type_name::TypePresentation::GenericWrap { name: "List" }),
            optional: crate::type_name::TypePresentation::Postfix { suffix: "?" },
            function: crate::type_name::FunctionPresentation {
                keyword: "",
                params_open: "(",
                params_sep: ", ",
                params_close: ")",
                arrow: " -> ",
                return_first: false,
                curried: false,
                wrapper_open: "",
                wrapper_close: "",
            },
            associated_type: crate::type_name::AssociatedTypeStyle::DotAccess,
            wildcard: crate::type_name::WildcardPresentation {
                unbounded: "*",
                upper_keyword: "out ",
                lower_keyword: "in ",
            },
            ..Default::default()
        }
    }

    fn generic_syntax(&self) -> crate::lang::config::GenericSyntaxConfig<'_> {
        crate::lang::config::GenericSyntaxConfig {
            constraint_keyword: " : ",
            constraint_separator: ", ",
            context_bound_keyword: " : ",
            ..Default::default()
        }
    }

    fn module_separator(&self) -> Option<&str> {
        Some(".")
    }

    fn block_syntax(&self) -> crate::lang::config::BlockSyntaxConfig<'_> {
        crate::lang::config::BlockSyntaxConfig {
            indent_unit: &self.indent,
            uses_semicolons: false,
            field_terminator: "",
            ..Default::default()
        }
    }

    fn function_syntax(&self) -> crate::lang::config::FunctionSyntaxConfig<'_> {
        crate::lang::config::FunctionSyntaxConfig {
            return_type_separator: ": ",
            async_keyword: "suspend ",
            constructor_delegation_style:
                crate::spec::modifiers::ConstructorDelegationStyle::Signature,
            ..Default::default()
        }
    }

    fn type_decl_syntax(&self) -> crate::lang::config::TypeDeclSyntaxConfig<'_> {
        crate::lang::config::TypeDeclSyntaxConfig {
            super_type_keyword: " : ",
            supports_primary_constructor: true,
            ..Default::default()
        }
    }

    fn enum_and_annotation(&self) -> crate::lang::config::EnumAndAnnotationConfig<'_> {
        crate::lang::config::EnumAndAnnotationConfig {
            readonly_keyword: "val ",
            mutable_field_keyword: "var ",
            variant_value_format: crate::lang::config::VariantValueFormat::ConstructorArg,
            variants_before_fields: true,
            variant_section_terminator: ";",
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
        let kt = Kotlin::new();
        assert_eq!(kt.file_extension(), "kt");
    }

    #[test]
    fn test_escape_reserved_backticks() {
        let kt = Kotlin::new();
        assert_eq!(kt.escape_reserved("class"), "`class`");
        assert_eq!(kt.escape_reserved("when"), "`when`");
        assert_eq!(kt.escape_reserved("val"), "`val`");
        assert_eq!(kt.escape_reserved("name"), "name");
        assert_eq!(kt.escape_reserved("override"), "`override`");
    }

    #[test]
    fn test_render_imports_single() {
        let kt = Kotlin::new();
        let imports = ImportGroup {
            entries: vec![ImportEntry {
                module: "kotlin.collections".into(),
                name: "List".into(),
                alias: None,
                is_type_only: false,
                is_side_effect: false,
                is_wildcard: false,
            }],
        };
        assert_eq!(
            kt.render_imports(&imports),
            "import kotlin.collections.List"
        );
    }

    #[test]
    fn test_render_imports_grouped() {
        let kt = Kotlin::new();
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
                    module: "kotlin.collections".into(),
                    name: "List".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "java.util".into(),
                    name: "UUID".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
            ],
        };
        let output = kt.render_imports(&imports);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines[0], "import kotlin.collections.List");
        assert_eq!(lines[1], "");
        assert_eq!(lines[2], "import java.util.UUID");
        assert_eq!(lines[3], "");
        assert_eq!(lines[4], "import com.example.model.User");
    }

    #[test]
    fn test_render_imports_sorted_within_group() {
        let kt = Kotlin::new();
        let imports = ImportGroup {
            entries: vec![
                ImportEntry {
                    module: "kotlin.collections".into(),
                    name: "Set".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "kotlin.collections".into(),
                    name: "List".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "kotlin.collections".into(),
                    name: "Map".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
            ],
        };
        let output = kt.render_imports(&imports);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines[0], "import kotlin.collections.List");
        assert_eq!(lines[1], "import kotlin.collections.Map");
        assert_eq!(lines[2], "import kotlin.collections.Set");
    }

    #[test]
    fn test_render_imports_dedup() {
        let kt = Kotlin::new();
        let imports = ImportGroup {
            entries: vec![
                ImportEntry {
                    module: "kotlin.collections".into(),
                    name: "List".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "kotlin.collections".into(),
                    name: "List".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
            ],
        };
        assert_eq!(
            kt.render_imports(&imports),
            "import kotlin.collections.List"
        );
    }

    #[test]
    fn test_doc_comment_single() {
        let kt = Kotlin::new();
        assert_eq!(
            kt.render_doc_comment(&["A brief description."]),
            "/**\n * A brief description.\n */"
        );
    }

    #[test]
    fn test_doc_comment_multi() {
        let kt = Kotlin::new();
        let doc = kt.render_doc_comment(&["Container class.", "", "@param T the element type"]);
        assert_eq!(
            doc,
            "/**\n * Container class.\n *\n * @param T the element type\n */"
        );
    }

    #[test]
    fn test_string_literal() {
        let kt = Kotlin::new();
        assert_eq!(kt.render_string_literal("hello"), "\"hello\"");
        assert_eq!(kt.render_string_literal("it\"s"), "\"it\\\"s\"");
        assert_eq!(kt.render_string_literal("new\nline"), "\"new\\nline\"");
        // Kotlin needs $ escaping for string templates.
        assert_eq!(kt.render_string_literal("$name"), "\"\\$name\"");
    }

    #[test]
    fn test_type_keyword() {
        let kt = Kotlin::new();
        assert_eq!(kt.type_keyword(TypeKind::Class), "class");
        assert_eq!(kt.type_keyword(TypeKind::Struct), "data class");
        assert_eq!(kt.type_keyword(TypeKind::Interface), "interface");
        assert_eq!(kt.type_keyword(TypeKind::Trait), "interface");
        assert_eq!(kt.type_keyword(TypeKind::Enum), "enum class");
    }

    #[test]
    fn test_visibility_top_level() {
        let kt = Kotlin::new();
        // Public is default in Kotlin — no keyword.
        assert_eq!(
            kt.render_visibility(Visibility::Public, DeclarationContext::TopLevel),
            ""
        );
        assert_eq!(
            kt.render_visibility(Visibility::Private, DeclarationContext::TopLevel),
            "private "
        );
    }

    #[test]
    fn test_visibility_member() {
        let kt = Kotlin::new();
        assert_eq!(
            kt.render_visibility(Visibility::Public, DeclarationContext::Member),
            ""
        );
        assert_eq!(
            kt.render_visibility(Visibility::Private, DeclarationContext::Member),
            "private "
        );
        assert_eq!(
            kt.render_visibility(Visibility::Protected, DeclarationContext::Member),
            "protected "
        );
    }

    #[test]
    fn test_no_semicolons() {
        let kt = Kotlin::new();
        assert!(!kt.block_syntax().uses_semicolons);
    }

    #[test]
    fn test_async_keyword() {
        let kt = Kotlin::new();
        assert_eq!(kt.function_syntax().async_keyword, "suspend ");
    }

    #[test]
    fn test_field_keywords() {
        let kt = Kotlin::new();
        assert_eq!(kt.enum_and_annotation().readonly_keyword, "val ");
        assert_eq!(kt.enum_and_annotation().mutable_field_keyword, "var ");
    }

    #[test]
    fn test_enum_config() {
        let kt = Kotlin::new();
        let ea = kt.enum_and_annotation();
        assert_eq!(
            ea.variant_value_format,
            crate::lang::config::VariantValueFormat::ConstructorArg
        );
        assert!(ea.variants_before_fields);
        assert_eq!(ea.variant_section_terminator, ";");
    }

    #[test]
    fn test_import_group_order() {
        assert_eq!(import_group_order("kotlin.collections"), 0);
        assert_eq!(import_group_order("kotlinx.coroutines"), 1);
        assert_eq!(import_group_order("java.util"), 2);
        assert_eq!(import_group_order("javax.inject"), 3);
        assert_eq!(import_group_order("com.example.model"), 4);
        assert_eq!(import_group_order("io.ktor.server"), 4);
    }

    #[test]
    fn test_kotlin_builder_fluent() {
        let kt = Kotlin::new().with_indent("  ").with_extension("kts");
        assert_eq!(kt.file_extension(), "kts");
        assert_eq!(kt.block_syntax().indent_unit, "  ");
    }

    #[test]
    fn test_module_separator() {
        let kt = Kotlin::new();
        assert_eq!(kt.module_separator(), Some("."));
    }
}
