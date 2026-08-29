//! Ruby language implementation.
//!
//! Ruby characteristics:
//! - Dynamically typed (no type annotations)
//! - `def` keyword, `end` for block close
//! - `#` line comments
//! - `class` and `module` type keywords
//! - `public`/`private`/`protected` visibility (method-call style)
//! - `require "module"` imports
//! - `@` instance variables, `@@` class variables
//! - `attr_reader`/`attr_writer`/`attr_accessor` for properties
//! - 2-space indent by convention
//! - `nil` absent literal
//! - `self.` prefix for class methods
//!
//! # Interpolation
//!
//! Ruby uses `#{expr}` for string interpolation. Since `$` is sigil_quote's
//! interpolation marker, every `$` in Ruby code (for global vars like `$stdout`)
//! must be written as `$$` inside `sigil_quote!(Ruby { ... })`.
//!
//! # Blocks
//!
//! Ruby uses `end` to close blocks, same as Lua. Braces `{ }` in
//! `sigil_quote!` map to indent/dedent + `end` in the output.

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::import::ImportGroup;
use crate::lang::capability::{
    FunctionBodyPolicy, FunctionCapability, FunctionCapabilityProfile, FunctionContext,
    FunctionForm, LanguageCapabilities, TypeCapability, TypeCapabilityProfile, VariantCapability,
    VariantCapabilityProfile,
};
#[expect(deprecated, reason = "0.6.8 compatibility implementation")]
use crate::lang::config::{
    BlockSyntaxConfig, EnumAndAnnotationConfig, FunctionSyntaxConfig, TypeDeclSyntaxConfig,
    TypePresentationConfig,
};
use crate::lang::{CodeLang, RendererLang};
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};

/// Ruby language implementation.
///
/// Configurable indent (default: 2 spaces by convention) and file extension
/// (default: `"rb"`).
#[derive(Debug, Clone)]
pub struct Ruby {
    /// Indent with this string (default: `"  "` — 2 spaces by convention).
    pub indent: String,
    /// File extension (default: `"rb"`).
    pub extension: String,
}

impl Default for Ruby {
    fn default() -> Self {
        Self {
            indent: "  ".to_string(),
            extension: "rb".to_string(),
        }
    }
}

impl Ruby {
    /// Create a new Ruby language instance with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the indent string (e.g., `"  "` for 2-space, `"    "` for 4-space).
    pub fn with_indent(mut self, s: &str) -> Self {
        self.indent = s.to_string();
        self
    }

    /// Set the file extension (default: `"rb"`).
    pub fn with_extension(mut self, s: &str) -> Self {
        self.extension = s.to_string();
        self
    }
}

#[rustfmt::skip]
const RUBY_RESERVED: &[&str] = &[
    "__ENCODING__", "__LINE__", "__FILE__", "BEGIN", "END",
    "alias", "and", "begin", "break", "case", "class", "def",
    "defined?", "do", "else", "elsif", "end", "ensure", "false",
    "for", "if", "in", "module", "next", "nil", "not", "or",
    "redo", "rescue", "retry", "return", "self", "super", "then",
    "true", "undef", "unless", "until", "when", "while", "yield",
];

#[deny(deprecated)]
impl RendererLang for Ruby {
    fn lower_type_name(
        &self,
        type_name: &crate::type_name::TypeName,
    ) -> Result<crate::code_block::CodeBlock, crate::error::SigilStitchError> {
        crate::lang::type_name_lowering::ruby(type_name)
    }
    fn file_extension(&self) -> &str {
        &self.extension
    }

    fn line_comment_prefix(&self) -> &str {
        "#"
    }

    fn render_string_literal(&self, s: &str) -> String {
        let escaped = s
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('#', "\\#")
            .replace('\n', "\\n")
            .replace('\t', "\\t")
            .replace('\r', "\\r");
        format!("\"{escaped}\"")
    }

    fn render_verbatim_string(&self, s: &str) -> String {
        // Use single quotes for verbatim strings — no interpolation risk.
        let escaped = s.replace('\\', "\\\\").replace('\'', "\\'");
        format!("'{escaped}'")
    }

    fn reserved_words(&self) -> &[&str] {
        RUBY_RESERVED
    }

    fn module_separator(&self) -> Option<&str> {
        Some("::")
    }

    fn indent_unit(&self) -> &str {
        &self.indent
    }

    fn render_statement_end(&self) -> Result<&str, crate::error::SigilStitchError> {
        Ok("")
    }

    fn render_block_open(
        &self,
        _intent: crate::code_node::BlockIntent,
        _condition: &str,
    ) -> Result<&str, crate::error::SigilStitchError> {
        Ok("")
    }

    fn render_block_close(
        &self,
        _intent: crate::code_node::BlockIntent,
        _condition: &str,
    ) -> Result<&str, crate::error::SigilStitchError> {
        Ok("end")
    }

    fn render_branch_transition(
        &self,
        _intent: crate::code_node::BlockIntent,
        _condition: &str,
    ) -> Result<String, crate::error::SigilStitchError> {
        Ok(String::new())
    }

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
    fn block_syntax(&self) -> BlockSyntaxConfig<'_> {
        BlockSyntaxConfig {
            block_open: "",
            block_close: "end",
            close_on_transition: false,
            indent_unit: &self.indent,
            uses_semicolons: false,
            field_terminator: "",
            ..Default::default()
        }
    }

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
    fn type_presentation(&self) -> TypePresentationConfig<'_> {
        TypePresentationConfig {
            optional_absent_literal: "nil",
            ..Default::default()
        }
    }
}

const RUBY_CLASS_CAPABILITIES: &[TypeCapability] = &[
    // Methods = methods
    TypeCapability::Methods,
    // NominalSubtyping = `<`
    TypeCapability::NominalSubtyping,
    // Attributes = annotations are not native; raw blocks remain available
    TypeCapability::Attributes,
];
const RUBY_TYPES: &[TypeCapabilityProfile] = &[
    TypeCapabilityProfile::new(TypeKind::Class, RUBY_CLASS_CAPABILITIES),
    // Struct is represented as a Ruby class.
    TypeCapabilityProfile::new(TypeKind::Struct, RUBY_CLASS_CAPABILITIES),
    TypeCapabilityProfile::new(
        TypeKind::Interface,
        &[
            // Methods = methods
            TypeCapability::Methods,
            // Attributes = annotations are not native; raw blocks remain available
            TypeCapability::Attributes,
        ],
    ),
    TypeCapabilityProfile::new(
        TypeKind::Trait,
        &[
            // Methods = methods
            TypeCapability::Methods,
            // Attributes = annotations are not native; raw blocks remain available
            TypeCapability::Attributes,
        ],
    ),
    TypeCapabilityProfile::new(
        TypeKind::Enum,
        &[
            // Variants = constants on the class object
            TypeCapability::Variants,
            // Attributes = annotations are not native; raw blocks remain available
            TypeCapability::Attributes,
        ],
    ),
];

const RUBY_VARIANTS: &[VariantCapabilityProfile] = &[VariantCapabilityProfile::new(
    TypeKind::Enum,
    &[
        VariantCapability::Discriminant,
        VariantCapability::Attributes,
    ],
)];

const RUBY_FUNCTION_CAPABILITIES: &[FunctionCapability] = &[
    // DefaultParameters = default parameters
    FunctionCapability::DefaultParameters,
];
const RUBY_FUNCTIONS: &[FunctionCapabilityProfile] = &[
    FunctionCapabilityProfile::new(
        FunctionContext::TopLevel,
        FunctionForm::Function,
        RUBY_FUNCTION_CAPABILITIES,
    )
    .with_body_policy(FunctionBodyPolicy::Required),
    FunctionCapabilityProfile::new(
        FunctionContext::Member,
        FunctionForm::Function,
        RUBY_FUNCTION_CAPABILITIES,
    )
    .with_body_policy(FunctionBodyPolicy::Required),
    FunctionCapabilityProfile::new(
        FunctionContext::InterfaceMember,
        FunctionForm::Function,
        RUBY_FUNCTION_CAPABILITIES,
    )
    .with_body_policy(FunctionBodyPolicy::Required),
];

impl CodeLang for Ruby {
    fn validate_resolved_imports(
        &self,
        imports: &crate::import::ImportGroup,
    ) -> Result<(), crate::error::SigilStitchError> {
        crate::lang::import_validation::reject_aliases(self, imports)
    }
    fn type_member_declaration_context(&self, _kind: TypeKind) -> DeclarationContext {
        DeclarationContext::Member
    }

    fn capabilities(&self) -> LanguageCapabilities<'_> {
        LanguageCapabilities::strict()
            .with_types(RUBY_TYPES)
            .with_functions(RUBY_FUNCTIONS)
            .with_variants(RUBY_VARIANTS)
    }

    fn validate_type(&self, type_: crate::lang::TypeIntent<'_>) -> Result<(), SigilStitchError> {
        crate::lang::type_lowering::ruby::validate(self, type_)
    }

    fn lower_type(
        &self,
        type_: crate::lang::ValidatedType<'_>,
    ) -> Result<Vec<CodeBlock>, SigilStitchError> {
        crate::lang::type_lowering::ruby::lower(self, type_)
    }

    fn lower_function(
        &self,
        function: crate::spec::fun_spec::ValidatedFunction<'_>,
    ) -> Result<crate::code_block::CodeBlock, crate::error::SigilStitchError> {
        crate::lang::ruby_function_lowering::lower(self, function)
    }

    fn validate_variants(
        &self,
        variants: crate::lang::VariantIntent<'_>,
    ) -> Result<(), SigilStitchError> {
        crate::lang::variant_lowering::ruby::validate(self, variants)
    }

    fn collect_variant_validation_errors(
        &self,
        variants: crate::lang::VariantIntent<'_>,
        errors: &mut Vec<SigilStitchError>,
    ) {
        crate::lang::variant_lowering::ruby::collect_validation_errors(self, variants, errors);
    }

    fn lower_variants(
        &self,
        variants: crate::lang::ValidatedVariants<'_>,
    ) -> Result<crate::code_block::CodeBlock, crate::error::SigilStitchError> {
        crate::lang::variant_lowering::ruby::lower(self, variants)
    }

    fn function_visibility_is_valid(
        &self,
        context: FunctionContext,
        _form: FunctionForm,
        _is_static: bool,
        visibility: Visibility,
    ) -> bool {
        match context {
            FunctionContext::TopLevel => visibility == Visibility::Inherited,
            FunctionContext::Member => matches!(
                visibility,
                Visibility::Inherited
                    | Visibility::Public
                    | Visibility::Private
                    | Visibility::Protected
            ),
            FunctionContext::InterfaceMember => {
                matches!(visibility, Visibility::Inherited | Visibility::Public)
            }
            FunctionContext::ReceiverMethod => false,
        }
    }

    fn render_visibility(&self, vis: Visibility, ctx: DeclarationContext) -> &str {
        match ctx {
            DeclarationContext::TopLevel => "",
            DeclarationContext::Member => match vis {
                Visibility::Public => "public\n",
                Visibility::Private => "private\n",
                Visibility::Protected => "protected\n",
                Visibility::Inherited => "public\n",
                Visibility::PublicCrate | Visibility::PublicSuper => "public\n",
            },
            DeclarationContext::InterfaceMember => "",
        }
    }

    fn function_keyword(&self, _ctx: DeclarationContext) -> &str {
        "def"
    }

    fn type_keyword(&self, kind: TypeKind) -> &str {
        match kind {
            TypeKind::Class | TypeKind::Struct => "class",
            TypeKind::Trait | TypeKind::Interface => "module",
            TypeKind::Enum => "class",
            TypeKind::TypeAlias | TypeKind::Newtype => "class",
        }
    }

    fn methods_inside_type_body(&self, _kind: TypeKind) -> bool {
        true
    }

    fn render_imports(&self, imports: &ImportGroup) -> String {
        let mut lines = Vec::new();
        for entry in imports.entries() {
            if entry.is_side_effect || entry.name.is_empty() || entry.is_wildcard {
                lines.push(format!("require '{}'", entry.module));
            } else {
                let module_path = entry.module.replace("::", "/");
                let name = entry.resolved_name();
                if entry.module.is_empty() {
                    lines.push(format!("require '{}'", name));
                } else {
                    lines.push(format!("require '{module_path}'",));
                }
            }
        }
        lines.sort();
        lines.dedup();
        lines.join("\n")
    }

    fn render_doc_comment(&self, lines: &[&str]) -> String {
        let mut out = String::new();
        for line in lines {
            if line.is_empty() {
                out.push_str("#\n");
            } else {
                out.push_str(&format!("# {line}\n"));
            }
        }
        out
    }

    fn fun_block_open(&self) -> &str {
        "" // Method bodies start on next line after `def`
    }

    fn type_header_block_open(&self, _kind: TypeKind) -> &str {
        "" // Class/module bodies start on next line
    }

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
    fn type_decl_syntax(&self) -> TypeDeclSyntaxConfig<'_> {
        TypeDeclSyntaxConfig {
            type_before_name: false,
            return_type_is_prefix: false,
            type_annotation_separator: "", // Ruby has no type annotations
            super_type_keyword: " < ",
            ..Default::default()
        }
    }

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
    fn function_syntax(&self) -> FunctionSyntaxConfig<'_> {
        FunctionSyntaxConfig {
            return_type_separator: "", // No return type syntax in Ruby
            empty_body: "",
            static_keyword: "self.",
            ..Default::default()
        }
    }

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
    fn enum_and_annotation(&self) -> EnumAndAnnotationConfig<'_> {
        EnumAndAnnotationConfig {
            readonly_keyword: "",
            annotation_prefix: "# ",
            annotation_suffix: "",
            ..Default::default()
        }
    }
}

#[cfg(test)]
#[expect(deprecated, reason = "0.6.8 compatibility assertions")]
mod tests {
    use crate::import::ImportEntry;

    use super::*;

    #[test]
    fn test_file_extension() {
        let ruby = Ruby::new();
        assert_eq!(ruby.file_extension(), "rb");
    }

    #[test]
    fn test_line_comment() {
        let ruby = Ruby::new();
        assert_eq!(ruby.line_comment_prefix(), "#");
    }

    #[test]
    fn test_render_string_literal() {
        let ruby = Ruby::new();
        assert_eq!(ruby.render_string_literal("hello"), "\"hello\"");
        assert_eq!(
            ruby.render_string_literal("say \"hi\""),
            "\"say \\\"hi\\\"\""
        );
        assert_eq!(
            ruby.render_string_literal("interp #{x}"),
            "\"interp \\#{x}\""
        );
        assert_eq!(
            ruby.render_string_literal("line1\nline2"),
            "\"line1\\nline2\""
        );
    }

    #[test]
    fn test_render_verbatim_string() {
        let ruby = Ruby::new();
        let out = ruby.render_verbatim_string("Hello #{name}");
        assert_eq!(out, "'Hello #{name}'");
    }

    #[test]
    fn test_escape_reserved() {
        let ruby = Ruby::new();
        assert_eq!(ruby.escape_reserved("foo"), "foo");
        assert_eq!(ruby.escape_reserved("class"), "class_");
        assert_eq!(ruby.escape_reserved("end"), "end_");
        assert_eq!(ruby.escape_reserved("def"), "def_");
    }

    #[test]
    fn test_render_imports() {
        let ruby = Ruby::new();
        let entries = vec![
            ImportEntry {
                module: "json".to_string(),
                name: "JSON".to_string(),
                alias: None,
                is_type_only: false,
                is_side_effect: false,
                is_wildcard: false,
            },
            ImportEntry {
                module: "net/http".to_string(),
                name: String::new(),
                alias: None,
                is_type_only: false,
                is_side_effect: true,
                is_wildcard: false,
            },
        ];
        let group = ImportGroup::from(entries);
        let out = ruby.render_imports(&group);
        assert!(out.contains("require 'json'"));
        assert!(out.contains("require 'net/http'"));
    }

    #[test]
    fn test_render_doc_comment() {
        let ruby = Ruby::new();
        let out = ruby.render_doc_comment(&["Says hello.", "", "@param name [String]"]);
        assert!(out.contains("# Says hello.\n"));
        assert!(out.contains("#\n"));
        assert!(out.contains("# @param name [String]\n"));
    }

    #[test]
    fn test_block_syntax() {
        let ruby = Ruby::new();
        let bs = ruby.block_syntax();
        assert_eq!(bs.block_open, "");
        assert_eq!(bs.block_close, "end");
        assert!(!bs.uses_semicolons);
        assert!(!bs.close_on_transition);
    }

    #[test]
    fn test_visibility() {
        let ruby = Ruby::new();
        assert_eq!(
            ruby.render_visibility(Visibility::Public, DeclarationContext::Member),
            "public\n"
        );
        assert_eq!(
            ruby.render_visibility(Visibility::Private, DeclarationContext::Member),
            "private\n"
        );
        assert_eq!(
            ruby.render_visibility(Visibility::Public, DeclarationContext::TopLevel),
            ""
        );
    }

    #[test]
    fn test_function_keyword() {
        let ruby = Ruby::new();
        assert_eq!(ruby.function_keyword(DeclarationContext::TopLevel), "def");
    }

    #[test]
    fn test_type_keyword() {
        let ruby = Ruby::new();
        assert_eq!(ruby.type_keyword(TypeKind::Class), "class");
        assert_eq!(ruby.type_keyword(TypeKind::Interface), "module");
        assert_eq!(ruby.type_keyword(TypeKind::Trait), "module");
    }

    #[test]
    fn test_module_separator() {
        let ruby = Ruby::new();
        assert_eq!(ruby.module_separator(), Some("::"));
    }

    #[test]
    fn test_static_keyword() {
        let ruby = Ruby::new();
        assert_eq!(ruby.function_syntax().static_keyword, "self.");
    }

    #[test]
    fn test_builder_fluent() {
        let ruby = Ruby::new().with_indent("\t").with_extension("rb");
        assert_eq!(ruby.file_extension(), "rb");
        assert_eq!(ruby.block_syntax().indent_unit, "\t");
    }
}
