//! OCaml language implementation.

use crate::code_block::CodeBlock;
use crate::code_node::BlockIntent;
use crate::error::SigilStitchError;
use crate::import::ImportGroup;
use crate::lang::capability::{
    FunctionBodyPolicy, FunctionCapability, FunctionCapabilityProfile, FunctionContext,
    FunctionForm, LanguageCapabilities, TypeCapability, TypeCapabilityProfile, VariantCapability,
    VariantCapabilityProfile,
};
#[expect(deprecated, reason = "0.6.8 compatibility implementation")]
use crate::lang::config::{
    BlockSyntaxConfig, EnumAndAnnotationConfig, FunctionSyntaxConfig, GenericSyntaxConfig,
    TypeDeclSyntaxConfig, TypePresentationConfig,
};
use crate::lang::{CodeLang, RendererLang};
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};
#[expect(deprecated, reason = "0.6.8 compatibility implementation")]
use crate::type_name::{
    AssociatedTypeStyle, FunctionPresentation, GenericApplicationStyle, TypePresentation,
};

fn ocaml_block_open_for_intent(intent: BlockIntent) -> Option<&'static str> {
    match intent {
        BlockIntent::ModuleType => Some(" = sig"),
        BlockIntent::Module => Some(" = struct"),
        BlockIntent::Match | BlockIntent::Try => Some(""),
        BlockIntent::If | BlockIntent::ElseIf => Some(" then"),
        BlockIntent::Else => Some(""),
        BlockIntent::For | BlockIntent::While => Some(" do"),
        _ => None,
    }
}

fn ocaml_block_close_for_intent(intent: BlockIntent) -> Option<&'static str> {
    match intent {
        BlockIntent::ModuleType | BlockIntent::Module => Some("end"),
        BlockIntent::For | BlockIntent::While => Some("done"),
        _ => None,
    }
}

fn ocaml_block_open_from_condition_text(condition: &str) -> Option<&'static str> {
    let trimmed = condition.trim();
    if trimmed.starts_with("module type ") {
        Some(" = sig")
    } else if trimmed.starts_with("module ") {
        Some(" = struct")
    } else if trimmed.starts_with("match ")
        || trimmed.starts_with("try ")
        || trimmed.ends_with(" with")
    {
        Some("")
    } else if trimmed.starts_with("if ") || trimmed.starts_with("else if ") {
        Some(" then")
    } else if trimmed == "else" {
        Some("")
    } else if trimmed.starts_with("for ") || trimmed.starts_with("while ") {
        Some(" do")
    } else {
        None
    }
}

fn ocaml_block_close_from_condition_text(condition: &str) -> Option<&'static str> {
    let trimmed = condition.trim();
    if trimmed.starts_with("module type ") || trimmed.starts_with("module ") {
        Some("end")
    } else if trimmed.starts_with("for ") || trimmed.starts_with("while ") {
        Some("done")
    } else {
        None
    }
}

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
        cb.begin_control_flow(&format!("module {name}"), ());
        cb.add_code(body);
        cb.end_control_flow();
        cb.build()
    }

    /// Build a `module type Name = sig ... end` block.
    pub fn module_sig_block(
        name: &str,
        body: crate::code_block::CodeBlock,
    ) -> Result<crate::code_block::CodeBlock, crate::error::SigilStitchError> {
        let mut cb = crate::code_block::CodeBlock::builder();
        cb.begin_control_flow(&format!("module type {name}"), ());
        cb.add_code(body);
        cb.end_control_flow();
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

#[deny(deprecated)]
impl RendererLang for OCaml {
    fn lower_type_name(
        &self,
        type_name: &crate::type_name::TypeName,
    ) -> Result<crate::code_block::CodeBlock, crate::error::SigilStitchError> {
        crate::lang::type_name_lowering::ocaml(type_name)
    }
    fn file_extension(&self) -> &str {
        &self.extension
    }

    fn reserved_words(&self) -> &[&str] {
        OCAML_RESERVED
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

    fn line_comment_prefix(&self) -> &str {
        "(*"
    }

    fn line_comment_suffix(&self) -> &str {
        " *)"
    }

    fn module_separator(&self) -> Option<&str> {
        Some(".")
    }

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
    fn type_presentation(&self) -> TypePresentationConfig<'_> {
        TypePresentationConfig {
            array: TypePresentation::Postfix { suffix: " list" },
            readonly_array: Some(TypePresentation::Postfix { suffix: " list" }),
            optional: TypePresentation::Postfix { suffix: " option" },
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

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
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

    fn indent_unit(&self) -> &str {
        &self.indent
    }

    fn render_statement_end(&self) -> Result<&str, SigilStitchError> {
        Ok("")
    }

    fn render_block_open(
        &self,
        intent: BlockIntent,
        condition: &str,
    ) -> Result<&str, SigilStitchError> {
        let open = if intent == BlockIntent::Generic {
            ocaml_block_open_from_condition_text(condition)
        } else {
            ocaml_block_open_for_intent(intent)
        };
        Ok(open.unwrap_or(" ="))
    }

    fn render_block_close(
        &self,
        intent: BlockIntent,
        condition: &str,
    ) -> Result<&str, SigilStitchError> {
        let close = if intent == BlockIntent::Generic {
            ocaml_block_close_from_condition_text(condition)
        } else {
            ocaml_block_close_for_intent(intent)
        };
        Ok(close.unwrap_or(""))
    }

    fn render_branch_transition(
        &self,
        intent: BlockIntent,
        condition: &str,
    ) -> Result<String, SigilStitchError> {
        let close = self.render_block_close(intent, condition)?;
        if close.is_empty() {
            Ok(String::new())
        } else {
            Ok(format!("{close} "))
        }
    }

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
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

    fn block_open_for(&self, condition: &str) -> Option<&str> {
        ocaml_block_open_from_condition_text(condition)
    }

    fn block_close_for(&self, condition: &str) -> Option<&str> {
        ocaml_block_close_from_condition_text(condition)
    }

    fn block_open_for_intent(&self, intent: BlockIntent, _condition: &str) -> Option<&str> {
        ocaml_block_open_for_intent(intent)
    }

    fn block_close_for_intent(&self, intent: BlockIntent, _condition: &str) -> Option<&str> {
        ocaml_block_close_for_intent(intent)
    }
}

const OCAML_RECORD_CAPABILITIES: &[TypeCapability] = &[
    // RecordFields = record labels
    TypeCapability::RecordFields,
    // ParametricPolymorphism = type parameters
    TypeCapability::ParametricPolymorphism,
];
const OCAML_VARIANT_CAPABILITIES: &[TypeCapability] = &[
    // ParametricPolymorphism = type parameters
    TypeCapability::ParametricPolymorphism,
    // Variants = constructors
    TypeCapability::Variants,
];
const OCAML_TYPES: &[TypeCapabilityProfile] = &[
    TypeCapabilityProfile::new(TypeKind::Struct, OCAML_RECORD_CAPABILITIES),
    // Class is represented as an OCaml record type.
    TypeCapabilityProfile::new(TypeKind::Class, OCAML_RECORD_CAPABILITIES),
    TypeCapabilityProfile::new(TypeKind::Enum, OCAML_VARIANT_CAPABILITIES),
    TypeCapabilityProfile::new(
        TypeKind::TypeAlias,
        &[
            // ParametricPolymorphism = type parameters
            TypeCapability::ParametricPolymorphism,
        ],
    ),
];

const OCAML_VARIANTS: &[VariantCapabilityProfile] = &[VariantCapabilityProfile::new(
    TypeKind::Enum,
    &[
        VariantCapability::PositionalPayload,
        VariantCapability::RecordPayload,
    ],
)];

const OCAML_FUNCTIONS: &[FunctionCapabilityProfile] = &[FunctionCapabilityProfile::new(
    FunctionContext::TopLevel,
    FunctionForm::Function,
    &[
        FunctionCapability::ExplicitReturnType,
        FunctionCapability::TypedParameters,
    ],
)
.with_body_policy(FunctionBodyPolicy::Required)];

impl CodeLang for OCaml {
    fn validate_resolved_imports(
        &self,
        imports: &crate::import::ImportGroup,
    ) -> Result<(), crate::error::SigilStitchError> {
        crate::lang::import_validation::reject_aliases(self, imports)?;
        if imports.entries().iter().any(|entry| entry.is_side_effect) {
            return Err(crate::error::SigilStitchError::InvalidResolvedImports {
                language: self.file_extension().to_string(),
                reason: "OCaml has no side-effect import form".to_string(),
            });
        }
        Ok(())
    }
    fn capabilities(&self) -> LanguageCapabilities<'_> {
        LanguageCapabilities::strict()
            .with_types(OCAML_TYPES)
            .with_functions(OCAML_FUNCTIONS)
            .with_variants(OCAML_VARIANTS)
            .with_fields(crate::lang::field_lowering::ocaml::PROFILES)
    }

    fn validate_type(&self, type_: crate::lang::TypeIntent<'_>) -> Result<(), SigilStitchError> {
        crate::lang::type_lowering::ocaml::validate(self, type_)
    }

    fn lower_type(
        &self,
        type_: crate::lang::ValidatedType<'_>,
    ) -> Result<Vec<CodeBlock>, SigilStitchError> {
        crate::lang::type_lowering::ocaml::lower(self, type_)
    }

    fn lower_function(
        &self,
        function: crate::spec::fun_spec::ValidatedFunction<'_>,
    ) -> Result<CodeBlock, SigilStitchError> {
        crate::lang::ocaml_function_lowering::lower(self, function)
    }

    fn validate_fields(
        &self,
        fields: crate::lang::FieldSequenceIntent<'_>,
    ) -> Result<(), SigilStitchError> {
        crate::lang::field_lowering::ocaml::validate(self, fields)
    }

    fn collect_field_validation_errors(
        &self,
        fields: crate::lang::FieldSequenceIntent<'_>,
        errors: &mut Vec<SigilStitchError>,
    ) {
        crate::lang::field_lowering::ocaml::collect_validation_errors(self, fields, errors);
    }

    fn lower_fields(
        &self,
        fields: crate::lang::ValidatedFields<'_>,
    ) -> Result<CodeBlock, SigilStitchError> {
        crate::lang::field_lowering::ocaml::lower(self, fields)
    }

    fn validate_variants(
        &self,
        variants: crate::lang::VariantIntent<'_>,
    ) -> Result<(), crate::error::SigilStitchError> {
        crate::lang::variant_lowering::ocaml::validate(self, variants)
    }

    fn collect_variant_validation_errors(
        &self,
        variants: crate::lang::VariantIntent<'_>,
        errors: &mut Vec<SigilStitchError>,
    ) {
        crate::lang::variant_lowering::ocaml::collect_validation_errors(self, variants, errors);
    }

    fn lower_variants(
        &self,
        variants: crate::lang::ValidatedVariants<'_>,
    ) -> Result<crate::code_block::CodeBlock, crate::error::SigilStitchError> {
        crate::lang::variant_lowering::ocaml::lower(self, variants)
    }

    fn render_imports(&self, imports: &ImportGroup) -> String {
        if imports.entries().is_empty() {
            return String::new();
        }

        let mut seen = std::collections::BTreeSet::new();
        let mut lines: Vec<String> = Vec::new();

        for entry in imports.entries() {
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
            crate::spec::modifiers::TypeKind::Struct | crate::spec::modifiers::TypeKind::Class => {
                "{".to_string()
            }
            _ => String::new(),
        }
    }

    fn type_body_suffix(&self, _name: &str, kind: crate::spec::modifiers::TypeKind) -> String {
        match kind {
            crate::spec::modifiers::TypeKind::Struct | crate::spec::modifiers::TypeKind::Class => {
                "}".to_string()
            }
            _ => String::new(),
        }
    }

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
    fn function_syntax(&self) -> FunctionSyntaxConfig<'_> {
        FunctionSyntaxConfig {
            return_type_separator: " : ",
            param_list_style: crate::spec::fun_spec::ParamListStyle::Curried,
            ..Default::default()
        }
    }

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
    fn type_decl_syntax(&self) -> TypeDeclSyntaxConfig<'_> {
        TypeDeclSyntaxConfig {
            type_annotation_separator: " : ",
            ..Default::default()
        }
    }

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
    fn enum_and_annotation(&self) -> EnumAndAnnotationConfig<'_> {
        EnumAndAnnotationConfig::default()
    }
}

#[cfg(test)]
#[expect(deprecated, reason = "0.6.8 compatibility assertions")]
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

    #[test]
    fn test_module_separator() {
        let ml = OCaml::new();
        assert_eq!(ml.module_separator(), Some("."));
    }

    #[test]
    fn test_block_intent_delimiters() {
        let ml = OCaml::new();
        assert_eq!(
            ml.block_open_for_intent(BlockIntent::ModuleType, "module type S"),
            Some(" = sig")
        );
        assert_eq!(
            ml.block_close_for_intent(BlockIntent::ModuleType, "module type S"),
            Some("end")
        );
        assert_eq!(
            ml.block_open_for_intent(BlockIntent::Module, "module Foo"),
            Some(" = struct")
        );
        assert_eq!(
            ml.block_open_for_intent(BlockIntent::Match, "let x = match v with"),
            Some("")
        );
        assert_eq!(
            ml.block_open_for_intent(BlockIntent::Try, "try f x"),
            Some("")
        );
        assert_eq!(
            ml.block_open_for_intent(BlockIntent::If, "if x > 0"),
            Some(" then")
        );
        assert_eq!(
            ml.block_close_for_intent(BlockIntent::For, "for i = 0 to 9"),
            Some("done")
        );
    }

    #[test]
    fn block_intent_near_matches_do_not_select_module_policy() {
        let ml = OCaml::new();
        assert_eq!(
            ml.block_open_for_intent(BlockIntent::Generic, "modular x y"),
            None
        );
        assert_eq!(
            ml.block_open_for_intent(BlockIntent::Generic, "matching x"),
            None
        );
    }

    #[test]
    fn builder_match_after_let_suppresses_default_opener() {
        use crate::code_block::CodeBlock;

        let ml = OCaml::new();
        let mut block = CodeBlock::builder();
        block.begin_control_flow("let describe x = match v with", ());
        block.add_statement("Some(v) -> v", ());
        block.end_control_flow();
        let output = block.build().unwrap().render_standalone(&ml, 80).unwrap();

        assert!(
            output.starts_with("let describe x = match v with\n"),
            "{output}"
        );
        assert!(!output.contains("match v with ="), "{output}");
    }
}
