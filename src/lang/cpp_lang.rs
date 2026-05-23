//! C++ language implementation.

use crate::import::{ImportEntry, ImportGroup};
use crate::lang::{CodeLang, RendererLang};
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};

/// C++ language implementation.
///
/// C++-specific behaviors:
/// - Type-before-name declarations (`int count`, not `count: int`)
/// - Return type as prefix (`int add(int a, int b)`)
/// - `#include` directives (system `<>` vs local `""`)
/// - Semicolons after statements and after struct/class/enum closing braces
/// - No function keyword (no `fn`/`func`/`def`)
/// - `class` keyword with `: public` inheritance
/// - `enum class` for scoped enums
/// - `virtual` instead of `abstract` for polymorphic methods
/// - `///` Doxygen-style doc comments
/// - Access specifiers (`public:`, `private:`, `protected:`) via `extra_member`
/// - Templates via `annotation` (e.g., `template<typename T>`)
/// - Method suffixes (`const`, `override`, `noexcept`, `= 0`) via `suffix()`
///
/// # Import conventions
///
/// Same as C — use [`crate::type_name::TypeName::importable`] with the header path as the module:
/// ```text
/// TypeName::importable("iostream", "std::cout")  // #include <iostream>
/// TypeName::importable("vector", "std::vector")   // #include <vector>
/// TypeName::importable("./myclass.hpp", "MyClass") // #include "myclass.hpp"
/// ```
///
/// # Access specifiers
///
/// C++ uses section-header access specifiers. Add them as `extra_member`:
/// ```text
/// let mut access = CodeBlock::builder();
/// access.add("%<", ());          // dedent
/// access.add("public:", ());
/// access.add_line();
/// access.add("%>", ());          // re-indent
/// tb.extra_member(access.build().unwrap());
/// ```
///
/// # Templates
///
/// Use annotations for `template<typename T>`:
/// ```text
/// fb.annotation(CodeBlock::of("template<typename T>", ()).unwrap());
/// ```
///
/// # Virtual / pure virtual
///
/// Use `is_abstract()` for `virtual` prefix, and `suffix("= 0")` for pure virtual:
/// ```text
/// fb.is_abstract();        // emits "virtual"
/// fb.suffix("= 0");       // emits "= 0" after params
/// ```
#[derive(Debug, Clone)]
pub struct CppLang {
    /// Indent with this string (default: "    " — 4 spaces).
    pub indent: String,
    /// File extension (default: "cpp"). Set to "hpp" or "h" for header files.
    pub extension: String,
}

impl Default for CppLang {
    fn default() -> Self {
        Self {
            indent: "    ".to_string(),
            extension: "cpp".to_string(),
        }
    }
}

impl CppLang {
    /// Create a new C++ language instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a CppLang configured for header files (.hpp extension).
    pub fn header() -> Self {
        Self {
            indent: "    ".to_string(),
            extension: "hpp".to_string(),
        }
    }

    /// Create a CppLang configured for .h header files.
    pub fn header_h() -> Self {
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

    /// Set the file extension (e.g., `"cpp"`, `"cxx"`, `"cc"`, `"hpp"`, `"hxx"`, `"h"`).
    pub fn with_extension(mut self, s: &str) -> Self {
        self.extension = s.to_string();
        self
    }

    /// Rewrite lambda blocks: insert `;` after `}` when the block condition
    /// contains `[` (lambda capture list).
    fn rewrite_lambda_semicolon(nodes: &mut Vec<crate::code_node::CodeNode>) {
        use crate::code_node::CodeNode;
        let mut i = 0;
        while i < nodes.len() {
            let is_lambda_close =
                matches!(&nodes[i], CodeNode::BlockClose(cond) if cond.contains('['));
            if is_lambda_close {
                // Insert a Literal(";") after the BlockClose, before Newline
                // The renderer will emit "}" then we want ";" immediately after
                // Actually, we need to replace BlockClose with Literal("};\n")
                // because BlockClose already emits "}" + newline.
                // Instead: replace BlockClose with Literal("};") + Newline
                let replacement = vec![CodeNode::Literal("};".to_string()), CodeNode::Newline];
                nodes.splice(i..i + 1, replacement);
                i += 2;
                continue;
            }
            i += 1;
        }
    }
}

#[rustfmt::skip]
const CPP_RESERVED: &[&str] = &[
    // C keywords (inherited)
    "auto", "break", "case", "char", "const", "continue", "default", "do",
    "double", "else", "enum", "extern", "float", "for", "goto", "if",
    "inline", "int", "long", "register", "return", "short", "signed",
    "sizeof", "static", "struct", "switch", "typedef", "union", "unsigned",
    "void", "volatile", "while",
    // C++ keywords
    "alignas", "alignof", "and", "and_eq", "asm", "atomic_cancel",
    "atomic_commit", "atomic_noexcept", "bitand", "bitor", "bool",
    "catch", "char8_t", "char16_t", "char32_t", "class", "co_await",
    "co_return", "co_yield", "compl", "concept", "const_cast", "consteval",
    "constexpr", "constinit", "decltype", "delete", "dynamic_cast",
    "explicit", "export", "false", "friend", "module", "mutable",
    "namespace", "new", "noexcept", "not", "not_eq", "nullptr",
    "operator", "or", "or_eq", "override", "private", "protected",
    "public", "reflexpr", "reinterpret_cast", "requires", "static_assert",
    "static_cast", "template", "this", "thread_local", "throw", "true",
    "try", "typeid", "typename", "using", "virtual", "wchar_t", "xor",
    "xor_eq",
];

/// Returns true if the header path looks like a system header (no `./` or `../` prefix).
fn is_system_header(module: &str) -> bool {
    !module.starts_with("./") && !module.starts_with("../")
}

/// Strip leading `./` from local header paths for the `#include` directive.
fn strip_local_prefix(module: &str) -> &str {
    module.strip_prefix("./").unwrap_or(module)
}

impl RendererLang for CppLang {
    fn file_extension(&self) -> &str {
        &self.extension
    }

    fn reserved_words(&self) -> &[&str] {
        CPP_RESERVED
    }

    fn line_comment_prefix(&self) -> &str {
        "//"
    }

    fn render_attribute(&self, text: &str) -> String {
        format!("[[{text}]]")
    }

    fn type_presentation(&self) -> crate::lang::config::TypePresentationConfig<'_> {
        crate::lang::config::TypePresentationConfig {
            array: crate::type_name::TypePresentation::GenericWrap {
                name: "std::vector",
            },
            readonly_array: Some(crate::type_name::TypePresentation::GenericWrap {
                name: "std::vector",
            }),
            optional: crate::type_name::TypePresentation::GenericWrap {
                name: "std::optional",
            },
            map: crate::type_name::TypePresentation::GenericWrap { name: "std::map" },
            pointer: crate::type_name::TypePresentation::Postfix { suffix: "*" },
            reference: crate::type_name::TypePresentation::Surround {
                prefix: "const ",
                suffix: "&",
            },
            reference_mut: crate::type_name::TypePresentation::Postfix { suffix: "&" },
            function: crate::type_name::FunctionPresentation {
                arrow: "",
                return_first: true,
                wrapper_open: "std::function<",
                wrapper_close: ">",
                ..Default::default()
            },
            tuple: crate::type_name::TypePresentation::GenericWrap { name: "std::tuple" },
            ..Default::default()
        }
    }

    fn generic_syntax(&self) -> crate::lang::config::GenericSyntaxConfig<'_> {
        crate::lang::config::GenericSyntaxConfig {
            constraint_keyword: "",
            constraint_separator: "",
            context_bound_keyword: "",
            ..Default::default()
        }
    }

    fn module_separator(&self) -> Option<&str> {
        Some("::")
    }

    fn block_syntax(&self) -> crate::lang::config::BlockSyntaxConfig<'_> {
        crate::lang::config::BlockSyntaxConfig {
            indent_unit: &self.indent,
            field_terminator: ";",
            type_close_terminator: ";",
            ..Default::default()
        }
    }

    fn rewrite_nodes(&self, nodes: &mut Vec<crate::code_node::CodeNode>) {
        crate::lang::rewrite::walk_nodes_mut(nodes, &Self::rewrite_lambda_semicolon);
    }
}

impl CodeLang for CppLang {
    fn render_imports(&self, imports: &ImportGroup) -> String {
        if imports.entries().is_empty() {
            return String::new();
        }

        // Deduplicate to header-level: C++ includes entire headers, not symbols.
        let mut seen = std::collections::BTreeSet::new();
        let mut system_headers: Vec<&ImportEntry> = Vec::new();
        let mut local_headers: Vec<&ImportEntry> = Vec::new();

        for entry in imports.entries() {
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

    fn render_doc_comment(&self, lines: &[&str]) -> String {
        // Doxygen-style /// comments.
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
        // C++ uses section-header access specifiers, not per-member keywords.
        // Users add `public:` / `private:` / `protected:` via extra_member.
        ""
    }

    fn function_keyword(&self, _ctx: DeclarationContext) -> &str {
        // C++ has no function keyword.
        ""
    }

    fn type_keyword(&self, kind: TypeKind) -> &str {
        match kind {
            TypeKind::Class => "class",
            TypeKind::Struct => "struct",
            TypeKind::Enum => "enum class",
            TypeKind::Interface | TypeKind::Trait => "class",
            TypeKind::TypeAlias => "using",
            TypeKind::Newtype => "struct",
        }
    }

    fn methods_inside_type_body(&self, kind: TypeKind) -> bool {
        match kind {
            TypeKind::Class | TypeKind::Interface | TypeKind::Trait => true,
            TypeKind::Struct | TypeKind::Enum | TypeKind::TypeAlias | TypeKind::Newtype => false,
        }
    }

    fn optional_field_style(&self) -> crate::lang::config::OptionalFieldStyle {
        // Note: callers must `#include <optional>` to use `std::optional<T>`.
        crate::lang::config::OptionalFieldStyle::TypeWrap {
            open: "std::optional<",
            close: ">",
        }
    }

    fn function_syntax(&self) -> crate::lang::config::FunctionSyntaxConfig<'_> {
        crate::lang::config::FunctionSyntaxConfig {
            return_type_separator: " ",
            abstract_keyword: "virtual ",
            ..Default::default()
        }
    }

    fn type_decl_syntax(&self) -> crate::lang::config::TypeDeclSyntaxConfig<'_> {
        crate::lang::config::TypeDeclSyntaxConfig {
            type_before_name: true,
            return_type_is_prefix: true,
            super_type_keyword: " : public ",
            super_type_separator: ", public ",
            ..Default::default()
        }
    }

    fn enum_and_annotation(&self) -> crate::lang::config::EnumAndAnnotationConfig<'_> {
        crate::lang::config::EnumAndAnnotationConfig {
            annotation_prefix: "[[",
            annotation_suffix: "]]",
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_extension() {
        let cpp = CppLang::new();
        assert_eq!(cpp.file_extension(), "cpp");
    }

    #[test]
    fn test_header_extension() {
        let cpp = CppLang::header();
        assert_eq!(cpp.file_extension(), "hpp");
    }

    #[test]
    fn test_header_h_extension() {
        let cpp = CppLang::header_h();
        assert_eq!(cpp.file_extension(), "h");
    }

    #[test]
    fn test_escape_reserved() {
        let cpp = CppLang::new();
        assert_eq!(cpp.escape_reserved("class"), "class_");
        assert_eq!(cpp.escape_reserved("virtual"), "virtual_");
        assert_eq!(cpp.escape_reserved("template"), "template_");
        assert_eq!(cpp.escape_reserved("name"), "name");
    }

    #[test]
    fn test_render_imports_system() {
        let cpp = CppLang::new();
        let imports = ImportGroup {
            entries: vec![ImportEntry {
                module: "iostream".into(),
                name: "std::cout".into(),
                alias: None,
                is_type_only: false,
                is_side_effect: false,
                is_wildcard: false,
            }],
        };
        assert_eq!(cpp.render_imports(&imports), "#include <iostream>");
    }

    #[test]
    fn test_render_imports_local() {
        let cpp = CppLang::new();
        let imports = ImportGroup {
            entries: vec![ImportEntry {
                module: "./myclass.hpp".into(),
                name: "MyClass".into(),
                alias: None,
                is_type_only: false,
                is_side_effect: false,
                is_wildcard: false,
            }],
        };
        assert_eq!(cpp.render_imports(&imports), "#include \"myclass.hpp\"");
    }

    #[test]
    fn test_render_imports_grouped() {
        let cpp = CppLang::new();
        let imports = ImportGroup {
            entries: vec![
                ImportEntry {
                    module: "iostream".into(),
                    name: "std::cout".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "vector".into(),
                    name: "std::vector".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "./myclass.hpp".into(),
                    name: "MyClass".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
            ],
        };
        let output = cpp.render_imports(&imports);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines[0], "#include <iostream>");
        assert_eq!(lines[1], "#include <vector>");
        assert_eq!(lines[2], "");
        assert_eq!(lines[3], "#include \"myclass.hpp\"");
    }

    #[test]
    fn test_render_imports_dedup() {
        let cpp = CppLang::new();
        let imports = ImportGroup {
            entries: vec![
                ImportEntry {
                    module: "vector".into(),
                    name: "std::vector".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "vector".into(),
                    name: "std::vector".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
            ],
        };
        assert_eq!(cpp.render_imports(&imports), "#include <vector>");
    }

    #[test]
    fn test_doc_comment_single() {
        let cpp = CppLang::new();
        assert_eq!(
            cpp.render_doc_comment(&["A brief description."]),
            "/// A brief description."
        );
    }

    #[test]
    fn test_doc_comment_multi() {
        let cpp = CppLang::new();
        let doc = cpp.render_doc_comment(&["Container class.", "", "Thread-safe."]);
        assert_eq!(doc, "/// Container class.\n///\n/// Thread-safe.");
    }

    #[test]
    fn test_string_literal() {
        let cpp = CppLang::new();
        assert_eq!(cpp.render_string_literal("hello"), "\"hello\"");
        assert_eq!(cpp.render_string_literal("it\"s"), "\"it\\\"s\"");
        assert_eq!(cpp.render_string_literal("new\nline"), "\"new\\nline\"");
    }

    #[test]
    fn test_type_before_name() {
        let cpp = CppLang::new();
        assert!(cpp.type_decl_syntax().type_before_name);
    }

    #[test]
    fn test_return_type_is_prefix() {
        let cpp = CppLang::new();
        assert!(cpp.type_decl_syntax().return_type_is_prefix);
    }

    #[test]
    fn test_type_close_terminator() {
        let cpp = CppLang::new();
        assert_eq!(cpp.block_syntax().type_close_terminator, ";");
    }

    #[test]
    fn test_type_keyword() {
        let cpp = CppLang::new();
        assert_eq!(cpp.type_keyword(TypeKind::Class), "class");
        assert_eq!(cpp.type_keyword(TypeKind::Struct), "struct");
        assert_eq!(cpp.type_keyword(TypeKind::Enum), "enum class");
        assert_eq!(cpp.type_keyword(TypeKind::Interface), "class");
    }

    #[test]
    fn test_abstract_keyword() {
        let cpp = CppLang::new();
        assert_eq!(cpp.function_syntax().abstract_keyword, "virtual ");
    }

    #[test]
    fn test_super_type_keyword() {
        let cpp = CppLang::new();
        assert_eq!(cpp.type_decl_syntax().super_type_keyword, " : public ");
    }

    #[test]
    fn test_super_type_separator() {
        let cpp = CppLang::new();
        assert_eq!(cpp.type_decl_syntax().super_type_separator, ", public ");
    }

    #[test]
    fn test_methods_inside_type_body() {
        let cpp = CppLang::new();
        assert!(cpp.methods_inside_type_body(TypeKind::Class));
        assert!(cpp.methods_inside_type_body(TypeKind::Interface));
        assert!(!cpp.methods_inside_type_body(TypeKind::Struct));
        assert!(!cpp.methods_inside_type_body(TypeKind::Enum));
    }

    #[test]
    fn test_is_system_header() {
        assert!(is_system_header("iostream"));
        assert!(is_system_header("vector"));
        assert!(is_system_header("string"));
        assert!(!is_system_header("./myclass.hpp"));
        assert!(!is_system_header("../utils/helper.h"));
    }

    #[test]
    fn test_cpp_builder_fluent() {
        let cpp = CppLang::new().with_indent("  ").with_extension("cxx");
        assert_eq!(cpp.file_extension(), "cxx");
        assert_eq!(cpp.block_syntax().indent_unit, "  ");
    }

    #[test]
    fn test_module_separator() {
        let cpp = CppLang::new();
        assert_eq!(cpp.module_separator(), Some("::"));
    }
}
