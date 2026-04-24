//! OCaml language implementation.

use crate::import::ImportGroup;
use crate::lang::CodeLang;
use crate::lang::config::{
    BlockSyntaxConfig, EnumAndAnnotationConfig, FunctionSyntaxConfig, GenericSyntaxConfig,
    TypeDeclSyntaxConfig, TypePresentationConfig,
};
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};
use crate::type_name::{
    AssociatedTypeStyle, FunctionPresentation, GenericApplicationStyle, TypePresentation,
};

/// OCaml language implementation.
///
/// OCaml-specific behaviors:
/// - Postfix generic application: `int list`, `(int, string) result`
/// - `let` function keyword
/// - `open Module` for imports
/// - No semicolons (expression-based)
/// - `type` keyword for all type declarations
/// - Record fields terminated with `;`
/// - `(** ... *)` OCamldoc comments
/// - Block comments `(* ... *)` only (no line comments)
/// - Curried function types: `int -> string -> bool`
/// - Tuple types with `*`: `int * string`
/// - Visibility is controlled via `.mli` files, not keywords
///
/// # Import conventions
///
/// Use [`crate::type_name::TypeName::importable`] with the module name:
/// ```text
/// TypeName::importable("List", "t")        // open List
/// TypeName::importable("Hashtbl", "t")     // open Hashtbl
/// ```
///
/// # Postfix generics
///
/// OCaml uses postfix generic application:
/// - Single param: `int option`, `string list`
/// - Multi param: `(int, string) result`
///
/// This is handled automatically via `generic_application_style() -> PostfixJuxtaposition`.
///
/// # Known limitations
///
/// - OCaml has no line comments; `line_comment_prefix` returns `"(*"` as the
///   closest approximation. Multi-line block comments `(* ... *)` should be
///   built with raw `CodeBlock` when needed.
/// - Module signatures (`.mli` files) are not directly modeled; use separate
///   `FileSpec` instances.
#[derive(Debug, Clone)]
pub struct OCaml {
    /// Indent with this string (default: "  " — 2 spaces).
    pub indent: String,
    /// File extension (default: "ml").
    pub extension: String,
}

impl Default for OCaml {
    fn default() -> Self {
        Self {
            indent: "  ".to_string(),
            extension: "ml".to_string(),
        }
    }
}

impl OCaml {
    /// Create a new OCaml language instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the indent string (e.g., `"  "` for 2-space default, `"\t"` for tabs).
    pub fn with_indent(mut self, s: &str) -> Self {
        self.indent = s.to_string();
        self
    }

    /// Set the file extension (default: `"ml"`). Set to `"mli"` for interface files.
    pub fn with_extension(mut self, s: &str) -> Self {
        self.extension = s.to_string();
        self
    }

    /// Build a `module Name = struct ... end` block.
    ///
    /// OCaml modules are structurally different from `TypeSpec` (they contain
    /// multiple types and values), so they are built as raw `CodeBlock`s.
    pub fn module_block(
        name: &str,
        body: crate::code_block::CodeBlock,
    ) -> Result<crate::code_block::CodeBlock, crate::error::SigilStitchError> {
        let mut cb = crate::code_block::CodeBlock::builder();
        cb.begin_control_flow_with_open(&format!("module {name}"), (), " = struct");
        cb.add_code(body);
        cb.end_control_flow();
        cb.add("end", ());
        cb.build()
    }

    /// Build a `module type Name = sig ... end` block.
    pub fn module_sig_block(
        name: &str,
        body: crate::code_block::CodeBlock,
    ) -> Result<crate::code_block::CodeBlock, crate::error::SigilStitchError> {
        let mut cb = crate::code_block::CodeBlock::builder();
        cb.begin_control_flow_with_open(&format!("module type {name}"), (), " = sig");
        cb.add_code(body);
        cb.end_control_flow();
        cb.add("end", ());
        cb.build()
    }
}

#[rustfmt::skip]
const OCAML_RESERVED: &[&str] = &[
    "and", "as", "assert", "asr", "begin", "class", "constraint", "do",
    "done", "downto", "else", "end", "exception", "external", "false",
    "for", "fun", "function", "functor", "if", "in", "include",
    "inherit", "initializer", "land", "lazy", "let", "lor", "lsl",
    "lsr", "lxor", "match", "method", "mod", "module", "mutable",
    "new", "nonrec", "object", "of", "open", "or", "private", "rec",
    "sig", "struct", "then", "to", "true", "try", "type", "val",
    "virtual", "when", "while", "with",
];

impl CodeLang for OCaml {
    fn file_extension(&self) -> &str {
        &self.extension
    }

    fn reserved_words(&self) -> &[&str] {
        OCAML_RESERVED
    }

    fn escape_reserved(&self, name: &str) -> String {
        if self.reserved_words().contains(&name) {
            format!("{name}_")
        } else {
            name.to_string()
        }
    }

    fn render_imports(&self, imports: &ImportGroup) -> String {
        if imports.entries.is_empty() {
            return String::new();
        }

        let mut seen = std::collections::BTreeSet::new();
        let mut lines: Vec<String> = Vec::new();

        for entry in &imports.entries {
            if entry.is_side_effect {
                continue;
            }
            let module = &entry.module;
            if !seen.insert(module.clone()) {
                continue;
            }
            lines.push(format!("open {module}"));
        }

        lines.sort();
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
        )
    }

    fn render_doc_comment(&self, lines: &[&str]) -> String {
        if lines.len() == 1 {
            return format!("(** {} *)", lines[0]);
        }
        let mut result = String::from("(**");
        for (i, line) in lines.iter().enumerate() {
            result.push('\n');
            if line.is_empty() {
                if i < lines.len() - 1 {
                    result.push_str("    ");
                }
            } else {
                result.push_str("    ");
                result.push_str(line);
            }
        }
        result.push_str(" *)");
        result
    }

    fn line_comment_prefix(&self) -> &str {
        "(*"
    }

    fn line_comment_suffix(&self) -> &str {
        " *)"
    }

    fn render_visibility(&self, _vis: Visibility, _ctx: DeclarationContext) -> &str {
        ""
    }

    fn function_keyword(&self, _ctx: DeclarationContext) -> &str {
        "let"
    }

    fn type_keyword(&self, _kind: TypeKind) -> &str {
        "type"
    }

    fn methods_inside_type_body(&self, _kind: TypeKind) -> bool {
        false
    }

    fn fun_block_open(&self) -> &str {
        " ="
    }

    fn type_header_block_open(&self, _kind: TypeKind) -> &str {
        " ="
    }

    fn type_body_prefix(&self, _name: &str, kind: crate::spec::modifiers::TypeKind) -> String {
        match kind {
            crate::spec::modifiers::TypeKind::Struct => "{".to_string(),
            _ => String::new(),
        }
    }

    fn type_body_suffix(&self, _name: &str, kind: crate::spec::modifiers::TypeKind) -> String {
        match kind {
            crate::spec::modifiers::TypeKind::Struct => "}".to_string(),
            _ => String::new(),
        }
    }

    // --- Config struct accessors ---

    fn type_presentation(&self) -> TypePresentationConfig<'_> {
        TypePresentationConfig {
            array: TypePresentation::GenericWrap { name: "list" },
            readonly_array: Some(TypePresentation::GenericWrap { name: "list" }),
            optional: TypePresentation::GenericWrap { name: "option" },
            map: TypePresentation::Delimited {
                open: "(",
                sep: ", ",
                close: ") Hashtbl.t",
            },
            tuple: TypePresentation::Infix { sep: " * " },
            function: FunctionPresentation {
                keyword: "",
                params_open: "",
                params_sep: " -> ",
                params_close: "",
                arrow: " -> ",
                return_first: false,
                curried: true,
                wrapper_open: "",
                wrapper_close: "",
            },
            associated_type: AssociatedTypeStyle::DotAccess,
            union: TypePresentation::Infix { sep: " | " },
            ..Default::default()
        }
    }

    fn generic_syntax(&self) -> GenericSyntaxConfig<'_> {
        GenericSyntaxConfig {
            open: "(",
            close: ")",
            application_style: GenericApplicationStyle::PostfixJuxtaposition,
            constraint_keyword: "",
            constraint_separator: "",
            ..Default::default()
        }
    }

    fn block_syntax(&self) -> BlockSyntaxConfig<'_> {
        BlockSyntaxConfig {
            block_open: " =",
            block_close: "",
            indent_unit: &self.indent,
            uses_semicolons: false,
            field_terminator: ";",
            ..Default::default()
        }
    }

    fn function_syntax(&self) -> FunctionSyntaxConfig<'_> {
        FunctionSyntaxConfig {
            return_type_separator: " : ",
            param_list_style: crate::spec::fun_spec::ParamListStyle::Curried,
            ..Default::default()
        }
    }

    fn type_decl_syntax(&self) -> TypeDeclSyntaxConfig<'_> {
        TypeDeclSyntaxConfig {
            type_annotation_separator: " : ",
            ..Default::default()
        }
    }

    fn enum_and_annotation(&self) -> EnumAndAnnotationConfig<'_> {
        EnumAndAnnotationConfig::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import::ImportEntry;

    #[test]
    fn test_file_extension() {
        let ml = OCaml::new();
        assert_eq!(ml.file_extension(), "ml");
    }

    #[test]
    fn test_escape_reserved() {
        let ml = OCaml::new();
        assert_eq!(ml.escape_reserved("match"), "match_");
        assert_eq!(ml.escape_reserved("type"), "type_");
        assert_eq!(ml.escape_reserved("name"), "name");
    }

    #[test]
    fn test_render_imports() {
        let ml = OCaml::new();
        let imports = ImportGroup {
            entries: vec![
                ImportEntry {
                    module: "List".into(),
                    name: "t".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "Hashtbl".into(),
                    name: "t".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
            ],
        };
        let output = ml.render_imports(&imports);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines[0], "open Hashtbl");
        assert_eq!(lines[1], "open List");
    }

    #[test]
    fn test_render_imports_dedup() {
        let ml = OCaml::new();
        let imports = ImportGroup {
            entries: vec![
                ImportEntry {
                    module: "List".into(),
                    name: "t".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "List".into(),
                    name: "map".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
            ],
        };
        assert_eq!(ml.render_imports(&imports), "open List");
    }

    #[test]
    fn test_doc_comment_single() {
        let ml = OCaml::new();
        assert_eq!(
            ml.render_doc_comment(&["A brief description."]),
            "(** A brief description. *)"
        );
    }

    #[test]
    fn test_doc_comment_multi() {
        let ml = OCaml::new();
        let doc = ml.render_doc_comment(&["Container module.", "", "@param t the element type"]);
        assert_eq!(
            doc,
            "(**\n    Container module.\n    \n    @param t the element type *)"
        );
    }

    #[test]
    fn test_string_literal() {
        let ml = OCaml::new();
        assert_eq!(ml.render_string_literal("hello"), "\"hello\"");
        assert_eq!(ml.render_string_literal("it\"s"), "\"it\\\"s\"");
        assert_eq!(ml.render_string_literal("new\nline"), "\"new\\nline\"");
    }

    #[test]
    fn test_type_keyword() {
        let ml = OCaml::new();
        assert_eq!(ml.type_keyword(TypeKind::Class), "type");
        assert_eq!(ml.type_keyword(TypeKind::Struct), "type");
        assert_eq!(ml.type_keyword(TypeKind::Enum), "type");
    }

    #[test]
    fn test_visibility_always_empty() {
        let ml = OCaml::new();
        assert_eq!(
            ml.render_visibility(Visibility::Public, DeclarationContext::TopLevel),
            ""
        );
        assert_eq!(
            ml.render_visibility(Visibility::Private, DeclarationContext::TopLevel),
            ""
        );
    }

    #[test]
    fn test_no_semicolons() {
        let ml = OCaml::new();
        assert!(!ml.block_syntax().uses_semicolons);
    }

    #[test]
    fn test_generic_application_style() {
        let ml = OCaml::new();
        assert!(matches!(
            ml.generic_syntax().application_style,
            crate::type_name::GenericApplicationStyle::PostfixJuxtaposition
        ));
    }

    #[test]
    fn test_ocaml_builder_fluent() {
        let ml = OCaml::new().with_indent("\t").with_extension("mli");
        assert_eq!(ml.file_extension(), "mli");
        assert_eq!(ml.block_syntax().indent_unit, "\t");
    }
}
