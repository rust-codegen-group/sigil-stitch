//! Scala language implementation.

use crate::import::ImportGroup;
use crate::lang::CodeLang;
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};

/// Scala language implementation.
///
/// Scala-specific behaviors:
/// - Name-before-type declarations (`count: Int`, not `Int count`)
/// - `def` function keyword
/// - `import pkg.{A, B}` with scala/java/third-party grouping (no semicolons)
/// - No semicolons
/// - `class`, `case class`, `trait`, `enum`, `type` keywords
/// - `:` for extends, `with` for mixin traits
/// - Generic bounds via `<:` (`[T <: Comparable[T]]`)
/// - `/** ... */` Scaladoc comments
/// - `val`/`var` for readonly/mutable properties
/// - Backtick escaping for reserved words
/// - Square brackets for generics (`List[Int]`, not `List<Int>`)
/// - Higher-kinded types (`F[_]`)
///
/// # Import conventions
///
/// Use [`crate::type_name::TypeName::importable`] with the package as module and class name as name:
/// ```text
/// TypeName::importable("scala.collection.mutable", "ListBuffer")
/// TypeName::importable("java.util", "UUID")
/// TypeName::importable("com.example.model", "User")
/// ```
///
/// # Inheritance
///
/// Scala uses `extends` for the first supertype and `with` for subsequent traits:
/// ```text
/// tb.super_type(TypeName::primitive("Base"));
/// tb.super_type(TypeName::primitive("Serializable"));
/// // Emits: class Foo extends Base with Serializable {
/// ```
///
/// # `sealed trait` / `case class`
///
/// Use `TypeKind::Trait` for traits and `TypeKind::Struct` for case classes.
/// For sealed modifiers, use annotations:
/// ```text
/// tb.annotation(CodeBlock::of("sealed", ()).unwrap());
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
#[derive(Debug, Clone)]
pub struct Scala {
    /// Indent with this string (default: "  " — 2 spaces).
    pub indent: String,
    /// File extension (default: "scala").
    pub extension: String,
}

impl Default for Scala {
    fn default() -> Self {
        Self {
            indent: "  ".to_string(),
            extension: "scala".to_string(),
        }
    }
}

impl Scala {
    /// Create a new Scala language instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the indent string (e.g., `"  "` for 2-space default, `"\t"` for tabs).
    pub fn with_indent(mut self, s: &str) -> Self {
        self.indent = s.to_string();
        self
    }

    /// Set the file extension (default: `"scala"`).
    pub fn with_extension(mut self, s: &str) -> Self {
        self.extension = s.to_string();
        self
    }
}

#[rustfmt::skip]
const SCALA_RESERVED: &[&str] = &[
    // Scala 2 + 3 keywords
    "abstract", "case", "catch", "class", "def", "do", "else", "enum",
    "export", "extends", "false", "final", "finally", "for", "forSome",
    "given", "if", "implicit", "import", "lazy", "match", "new", "null",
    "object", "override", "package", "private", "protected", "return",
    "sealed", "super", "then", "this", "throw", "trait", "true", "try",
    "type", "val", "var", "while", "with", "yield",
];

/// Classify an import module into a group for ordering.
/// 0 = scala.*, 1 = java.*/javax.*, 2 = everything else.
fn import_group_order(module: &str) -> u8 {
    if module.starts_with("scala.") || module == "scala" {
        0
    } else if module.starts_with("java.") || module.starts_with("javax.") {
        1
    } else {
        2
    }
}

impl CodeLang for Scala {
    fn file_extension(&self) -> &str {
        &self.extension
    }

    fn reserved_words(&self) -> &[&str] {
        SCALA_RESERVED
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

        let mut scala_imports: Vec<String> = Vec::new();
        let mut java_imports: Vec<String> = Vec::new();
        let mut other_imports: Vec<String> = Vec::new();

        let mut seen = std::collections::BTreeSet::new();
        for entry in imports.entries() {
            let line = if entry.is_wildcard {
                let fqn = format!("{}._", entry.module);
                if !seen.insert(fqn.clone()) {
                    continue;
                }
                format!("import {fqn}")
            } else if entry.is_side_effect {
                continue;
            } else {
                let fqn = format!("{}.{}", entry.module, entry.name);
                if !seen.insert(fqn.clone()) {
                    continue;
                }
                format!("import {fqn}")
            };

            match import_group_order(&entry.module) {
                0 => scala_imports.push(line),
                1 => java_imports.push(line),
                _ => other_imports.push(line),
            }
        }

        scala_imports.sort();
        java_imports.sort();
        other_imports.sort();

        let groups: Vec<&Vec<String>> = [&scala_imports, &java_imports, &other_imports]
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

    fn render_visibility(&self, vis: Visibility, _ctx: DeclarationContext) -> &str {
        match vis {
            Visibility::Public | Visibility::Inherited => "",
            Visibility::Private => "private ",
            Visibility::Protected => "protected ",
            Visibility::PublicCrate => "private[this] ",
            Visibility::PublicSuper => "protected ",
        }
    }

    fn function_keyword(&self, _ctx: DeclarationContext) -> &str {
        "def"
    }

    fn type_keyword(&self, kind: TypeKind) -> &str {
        match kind {
            TypeKind::Class => "class",
            TypeKind::Struct => "case class",
            TypeKind::Interface | TypeKind::Trait => "trait",
            TypeKind::Enum => "enum",
            TypeKind::TypeAlias => "type",
            TypeKind::Newtype => "class",
        }
    }

    fn methods_inside_type_body(&self, _kind: TypeKind) -> bool {
        true
    }

    fn optional_field_style(&self) -> crate::lang::config::OptionalFieldStyle {
        crate::lang::config::OptionalFieldStyle::TypeWrap {
            open: "Option[",
            close: "]",
        }
    }

    fn render_type_param_kind(&self, kind: &crate::spec::fun_spec::TypeParamKind) -> String {
        match kind {
            crate::spec::fun_spec::TypeParamKind::Constructor1 => "[_]".to_string(),
            crate::spec::fun_spec::TypeParamKind::Constructor2 => "[_, _]".to_string(),
            crate::spec::fun_spec::TypeParamKind::Raw(s) => s.clone(),
        }
    }

    fn render_newtype_line(&self, vis: &str, name: &str, inner: &str) -> String {
        format!("{vis}class {name}(val value: {inner})")
    }

    fn fun_block_open(&self) -> &str {
        " = {"
    }

    fn type_presentation(&self) -> crate::lang::config::TypePresentationConfig<'_> {
        crate::lang::config::TypePresentationConfig {
            array: crate::type_name::TypePresentation::GenericWrap { name: "Array" },
            readonly_array: Some(crate::type_name::TypePresentation::GenericWrap { name: "List" }),
            optional: crate::type_name::TypePresentation::GenericWrap { name: "Option" },
            intersection: crate::type_name::TypePresentation::Infix { sep: " with " },
            associated_type: crate::type_name::AssociatedTypeStyle::DotAccess,
            wildcard: crate::type_name::WildcardPresentation {
                unbounded: "_",
                upper_keyword: "_ <: ",
                lower_keyword: "_ >: ",
            },
            ..Default::default()
        }
    }

    fn generic_syntax(&self) -> crate::lang::config::GenericSyntaxConfig<'_> {
        crate::lang::config::GenericSyntaxConfig {
            open: "[",
            close: "]",
            constraint_keyword: " <: ",
            constraint_separator: " with ",
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
            abstract_keyword: "",
            ..Default::default()
        }
    }

    fn type_decl_syntax(&self) -> crate::lang::config::TypeDeclSyntaxConfig<'_> {
        crate::lang::config::TypeDeclSyntaxConfig {
            super_type_keyword: " extends ",
            super_type_subsequent_separator: Some(" with "),
            implements_keyword: " with ",
            supports_primary_constructor: true,
            ..Default::default()
        }
    }

    fn enum_and_annotation(&self) -> crate::lang::config::EnumAndAnnotationConfig<'_> {
        crate::lang::config::EnumAndAnnotationConfig {
            readonly_keyword: "val ",
            mutable_field_keyword: "var ",
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
        let sc = Scala::new();
        assert_eq!(sc.file_extension(), "scala");
    }

    #[test]
    fn test_escape_reserved_backticks() {
        let sc = Scala::new();
        assert_eq!(sc.escape_reserved("type"), "`type`");
        assert_eq!(sc.escape_reserved("val"), "`val`");
        assert_eq!(sc.escape_reserved("match"), "`match`");
        assert_eq!(sc.escape_reserved("name"), "name");
    }

    #[test]
    fn test_render_imports_single() {
        let sc = Scala::new();
        let imports = ImportGroup {
            entries: vec![ImportEntry {
                module: "scala.collection.mutable".into(),
                name: "ListBuffer".into(),
                alias: None,
                is_type_only: false,
                is_side_effect: false,
                is_wildcard: false,
            }],
        };
        assert_eq!(
            sc.render_imports(&imports),
            "import scala.collection.mutable.ListBuffer"
        );
    }

    #[test]
    fn test_render_imports_grouped() {
        let sc = Scala::new();
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
                    module: "scala.collection.immutable".into(),
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
        let output = sc.render_imports(&imports);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines[0], "import scala.collection.immutable.List");
        assert_eq!(lines[1], "");
        assert_eq!(lines[2], "import java.util.UUID");
        assert_eq!(lines[3], "");
        assert_eq!(lines[4], "import com.example.model.User");
    }

    #[test]
    fn test_render_imports_sorted_within_group() {
        let sc = Scala::new();
        let imports = ImportGroup {
            entries: vec![
                ImportEntry {
                    module: "scala.collection.immutable".into(),
                    name: "Set".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "scala.collection.immutable".into(),
                    name: "List".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "scala.collection.immutable".into(),
                    name: "Map".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
            ],
        };
        let output = sc.render_imports(&imports);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines[0], "import scala.collection.immutable.List");
        assert_eq!(lines[1], "import scala.collection.immutable.Map");
        assert_eq!(lines[2], "import scala.collection.immutable.Set");
    }

    #[test]
    fn test_render_imports_dedup() {
        let sc = Scala::new();
        let imports = ImportGroup {
            entries: vec![
                ImportEntry {
                    module: "scala.collection.immutable".into(),
                    name: "List".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "scala.collection.immutable".into(),
                    name: "List".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
            ],
        };
        assert_eq!(
            sc.render_imports(&imports),
            "import scala.collection.immutable.List"
        );
    }

    #[test]
    fn test_doc_comment_single() {
        let sc = Scala::new();
        assert_eq!(
            sc.render_doc_comment(&["A brief description."]),
            "/**\n * A brief description.\n */"
        );
    }

    #[test]
    fn test_doc_comment_multi() {
        let sc = Scala::new();
        let doc = sc.render_doc_comment(&["Container class.", "", "@tparam T the element type"]);
        assert_eq!(
            doc,
            "/**\n * Container class.\n *\n * @tparam T the element type\n */"
        );
    }

    #[test]
    fn test_string_literal() {
        let sc = Scala::new();
        assert_eq!(sc.render_string_literal("hello"), "\"hello\"");
        assert_eq!(sc.render_string_literal("it\"s"), "\"it\\\"s\"");
        assert_eq!(sc.render_string_literal("new\nline"), "\"new\\nline\"");
        assert_eq!(sc.render_string_literal("$name"), "\"$name\"");
    }

    #[test]
    fn test_type_keyword() {
        let sc = Scala::new();
        assert_eq!(sc.type_keyword(TypeKind::Class), "class");
        assert_eq!(sc.type_keyword(TypeKind::Struct), "case class");
        assert_eq!(sc.type_keyword(TypeKind::Interface), "trait");
        assert_eq!(sc.type_keyword(TypeKind::Trait), "trait");
        assert_eq!(sc.type_keyword(TypeKind::Enum), "enum");
        assert_eq!(sc.type_keyword(TypeKind::TypeAlias), "type");
    }

    #[test]
    fn test_visibility() {
        let sc = Scala::new();
        assert_eq!(
            sc.render_visibility(Visibility::Public, DeclarationContext::TopLevel),
            ""
        );
        assert_eq!(
            sc.render_visibility(Visibility::Private, DeclarationContext::TopLevel),
            "private "
        );
        assert_eq!(
            sc.render_visibility(Visibility::Protected, DeclarationContext::Member),
            "protected "
        );
    }

    #[test]
    fn test_no_semicolons() {
        let sc = Scala::new();
        assert!(!sc.block_syntax().uses_semicolons);
    }

    #[test]
    fn test_generic_brackets() {
        let sc = Scala::new();
        assert_eq!(sc.generic_syntax().open, "[");
        assert_eq!(sc.generic_syntax().close, "]");
    }

    #[test]
    fn test_field_keywords() {
        let sc = Scala::new();
        assert_eq!(sc.enum_and_annotation().readonly_keyword, "val ");
        assert_eq!(sc.enum_and_annotation().mutable_field_keyword, "var ");
    }

    #[test]
    fn test_import_group_order() {
        assert_eq!(import_group_order("scala.collection.immutable"), 0);
        assert_eq!(import_group_order("java.util"), 1);
        assert_eq!(import_group_order("javax.inject"), 1);
        assert_eq!(import_group_order("com.example.model"), 2);
        assert_eq!(import_group_order("org.apache.spark"), 2);
    }

    #[test]
    fn test_hkt_rendering() {
        let sc = Scala::new();
        use crate::spec::fun_spec::TypeParamKind;
        assert_eq!(
            sc.render_type_param_kind(&TypeParamKind::Constructor1),
            "[_]"
        );
        assert_eq!(
            sc.render_type_param_kind(&TypeParamKind::Constructor2),
            "[_, _]"
        );
        assert_eq!(
            sc.render_type_param_kind(&TypeParamKind::Raw("[_[_]]".to_string())),
            "[_[_]]"
        );
    }

    #[test]
    fn test_scala_builder_fluent() {
        let sc = Scala::new().with_indent("\t").with_extension("sc");
        assert_eq!(sc.file_extension(), "sc");
        assert_eq!(sc.block_syntax().indent_unit, "\t");
    }

    #[test]
    fn test_super_type_subsequent_separator() {
        let sc = Scala::new();
        assert_eq!(
            sc.type_decl_syntax().super_type_subsequent_separator,
            Some(" with ")
        );
    }

    #[test]
    fn test_context_bound_keyword() {
        let sc = Scala::new();
        assert_eq!(sc.generic_syntax().context_bound_keyword, " : ");
    }

    #[test]
    fn test_render_newtype_line() {
        let sc = Scala::new();
        assert_eq!(
            sc.render_newtype_line("", "Meters", "Double"),
            "class Meters(val value: Double)"
        );
    }

    #[test]
    fn test_module_separator() {
        let sc = Scala::new();
        assert_eq!(sc.module_separator(), Some("."));
    }
}
