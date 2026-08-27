use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::import::{ImportEntry, ImportGroup};
use crate::lang::capability::{
    FunctionBodyPolicy, FunctionCapability, FunctionCapabilityProfile, FunctionContext,
    FunctionForm, LanguageCapabilities, TypeCapability, TypeCapabilityProfile, VariantCapability,
    VariantCapabilityProfile,
};
#[expect(deprecated, reason = "0.6.8 compatibility implementation")]
use crate::lang::config::{
    BlockSyntaxConfig, EnumAndAnnotationConfig, FunctionSyntaxConfig, GenericSyntaxConfig,
    QuoteStyle, TypeDeclSyntaxConfig, TypePresentationConfig,
};
use crate::lang::{CodeLang, RendererLang};
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};
#[expect(deprecated, reason = "0.6.8 compatibility implementation")]
use crate::type_name::{
    AssociatedTypeStyle, BoundsPresentation, TypePresentation, WildcardPresentation,
};

/// TypeScript language implementation.
///
/// Construct with [`TypeScript::new()`] and customize via the `with_*`
/// methods, e.g. `TypeScript::new().with_quote_style(QuoteStyle::Double)`.
#[derive(Debug, Clone)]
pub struct TypeScript {
    /// Quote style for string literals and import paths.
    #[deprecated(note = "legacy 0.6.8 field; quote selection is language-owned in 0.7")]
    #[expect(deprecated, reason = "0.6.8 compatibility field")]
    pub quote_style: QuoteStyle,
    /// Indent with this string (default: "  ").
    pub indent: String,
    /// Whether to terminate statements with `;` (default: true).
    pub uses_semicolons: bool,
    /// File extension (default: "ts"). Set to "tsx" for JSX/TSX projects.
    pub extension: String,
}

impl Default for TypeScript {
    #[expect(deprecated, reason = "0.6.8 quote-style compatibility bridge")]
    fn default() -> Self {
        Self {
            quote_style: QuoteStyle::Single,
            indent: "  ".to_string(),
            uses_semicolons: true,
            extension: "ts".to_string(),
        }
    }
}

impl TypeScript {
    /// Create a new TypeScript language instance with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the quote style used for string literals and import paths.
    #[deprecated(note = "legacy 0.6.8 setter; use language-local quote selection in 0.7")]
    #[expect(deprecated, reason = "0.6.8 compatibility setter")]
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
        self.uses_semicolons = b;
        self
    }

    /// Set the file extension (e.g., `"ts"` or `"tsx"`).
    pub fn with_extension(mut self, s: &str) -> Self {
        self.extension = s.to_string();
        self
    }

    #[expect(deprecated, reason = "0.6.8 quote compatibility bridge")]
    fn quote_char(&self) -> char {
        match self.quote_style {
            QuoteStyle::Single => '\'',
            QuoteStyle::Double => '"',
        }
    }
}

const TS_RESERVED: &[&str] = &[
    // ECMAScript reserved words
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "debugger",
    "default",
    "delete",
    "do",
    "else",
    "enum",
    "export",
    "extends",
    "false",
    "finally",
    "for",
    "function",
    "if",
    "import",
    "in",
    "instanceof",
    "new",
    "null",
    "return",
    "super",
    "switch",
    "this",
    "throw",
    "true",
    "try",
    "typeof",
    "var",
    "void",
    "while",
    "with",
    // Strict-mode reserved words
    "implements",
    "interface",
    "let",
    "package",
    "private",
    "protected",
    "public",
    "static",
    "yield",
    // Async/await (ES2017+)
    "async",
    "await",
    // TypeScript keywords and contextual keywords
    "abstract",
    "any",
    "as",
    "asserts",
    "assert",
    "bigint",
    "boolean",
    "constructor",
    "declare",
    "from",
    "get",
    "global",
    "infer",
    "intrinsic",
    "is",
    "keyof",
    "module",
    "namespace",
    "never",
    "number",
    "object",
    "of",
    "out",
    "override",
    "readonly",
    "require",
    "satisfies",
    "set",
    "string",
    "symbol",
    "type",
    "undefined",
    "unique",
    "unknown",
    "using",
    // TS 5.5+ contextual keywords
    "accessor",
    "defer",
];

impl RendererLang for TypeScript {
    fn file_extension(&self) -> &str {
        &self.extension
    }

    fn reserved_words(&self) -> &[&str] {
        TS_RESERVED
    }

    fn render_string_literal(&self, s: &str) -> String {
        match self.quote_char() {
            '\'' => {
                format!("'{}'", s.replace('\\', "\\\\").replace('\'', "\\'"))
            }
            '"' => {
                format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
            }
            _ => unreachable!("quote compatibility helper returns only supported delimiters"),
        }
    }

    fn render_verbatim_string(&self, s: &str) -> String {
        let escaped = s.replace('\\', "\\\\").replace('`', "\\`");
        format!("`{escaped}`")
    }

    fn line_comment_prefix(&self) -> &str {
        "//"
    }

    // --- Config struct accessors ---

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
    fn type_presentation(&self) -> TypePresentationConfig<'_> {
        TypePresentationConfig {
            map: TypePresentation::GenericWrap { name: "Record" },
            tuple: TypePresentation::Delimited {
                open: "[",
                sep: ", ",
                close: "]",
            },
            associated_type: AssociatedTypeStyle::IndexAccess {
                open: "[\"",
                close: "\"]",
            },
            impl_trait: BoundsPresentation {
                keyword: "",
                separator: " & ",
            },
            wildcard: WildcardPresentation {
                unbounded: "unknown",
                upper_keyword: "unknown ",
                lower_keyword: "unknown ",
            },
            ..Default::default()
        }
    }

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
    fn generic_syntax(&self) -> GenericSyntaxConfig<'_> {
        GenericSyntaxConfig {
            constraint_keyword: " extends ",
            constraint_separator: " & ",
            ..Default::default()
        }
    }

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
    fn block_syntax(&self) -> BlockSyntaxConfig<'_> {
        BlockSyntaxConfig {
            indent_unit: &self.indent,
            uses_semicolons: self.uses_semicolons,
            field_terminator: ";",
            ..Default::default()
        }
    }
}

const TS_CLASS_CAPABILITIES: &[TypeCapability] = &[
    // RecordFields = properties/fields
    TypeCapability::RecordFields,
    // AccessorMethods = get/set accessors
    TypeCapability::AccessorMethods,
    // Methods = methods
    TypeCapability::Methods,
    // NominalSubtyping = `extends`
    TypeCapability::NominalSubtyping,
    // InterfaceImplementation = `implements`
    TypeCapability::InterfaceImplementation,
    // ParametricPolymorphism = generic type parameters
    TypeCapability::ParametricPolymorphism,
    // BoundedPolymorphism = generic constraints
    TypeCapability::BoundedPolymorphism,
    // Attributes = decorators
    TypeCapability::Attributes,
];
const TS_CONTRACT_CAPABILITIES: &[TypeCapability] = &[
    // RecordFields = properties/fields
    TypeCapability::RecordFields,
    // Methods = methods
    TypeCapability::Methods,
    // NominalSubtyping = `extends`
    TypeCapability::NominalSubtyping,
    // ParametricPolymorphism = generic type parameters
    TypeCapability::ParametricPolymorphism,
    // BoundedPolymorphism = generic constraints
    TypeCapability::BoundedPolymorphism,
];
const TS_TYPES: &[TypeCapabilityProfile] = &[
    TypeCapabilityProfile::new(TypeKind::Class, TS_CLASS_CAPABILITIES),
    // Struct is represented as a TypeScript class.
    TypeCapabilityProfile::new(TypeKind::Struct, TS_CLASS_CAPABILITIES),
    TypeCapabilityProfile::new(TypeKind::Interface, TS_CONTRACT_CAPABILITIES),
    // Trait is represented as a TypeScript interface.
    TypeCapabilityProfile::new(TypeKind::Trait, TS_CONTRACT_CAPABILITIES),
    TypeCapabilityProfile::new(
        TypeKind::Enum,
        &[
            // Variants = enum members
            TypeCapability::Variants,
        ],
    ),
    TypeCapabilityProfile::new(
        TypeKind::TypeAlias,
        &[
            // ParametricPolymorphism = generic type parameters
            TypeCapability::ParametricPolymorphism,
            // BoundedPolymorphism = `extends` constraints on alias parameters
            TypeCapability::BoundedPolymorphism,
        ],
    ),
];

const TS_VARIANTS: &[VariantCapabilityProfile] = &[VariantCapabilityProfile::new(
    TypeKind::Enum,
    &[VariantCapability::Discriminant],
)];

const TS_TOP_LEVEL_FUNCTION_CAPABILITIES: &[FunctionCapability] = &[
    // AsyncEffect = async
    FunctionCapability::AsyncEffect,
    // BoundedPolymorphism = extends constraints
    FunctionCapability::BoundedPolymorphism,
    // DefaultParameters = default parameters
    FunctionCapability::DefaultParameters,
    // ExplicitReturnType = return type annotation
    FunctionCapability::ExplicitReturnType,
    // TypedParameters = optional parameter annotations
    FunctionCapability::TypedParameters,
    // ParametricPolymorphism = generic type parameters
    FunctionCapability::ParametricPolymorphism,
    // VariadicParameters = rest parameters
    FunctionCapability::VariadicParameters,
];
const TS_MEMBER_FUNCTION_CAPABILITIES: &[FunctionCapability] = &[
    // AbstractMethod = abstract
    FunctionCapability::AbstractMethod,
    // AsyncEffect = async
    FunctionCapability::AsyncEffect,
    // Attributes = decorators
    FunctionCapability::Attributes,
    // BoundedPolymorphism = extends constraints
    FunctionCapability::BoundedPolymorphism,
    // DefaultParameters = default parameters
    FunctionCapability::DefaultParameters,
    // ExplicitReturnType = return type annotation
    FunctionCapability::ExplicitReturnType,
    // TypedParameters = optional parameter annotations
    FunctionCapability::TypedParameters,
    // Override = override
    FunctionCapability::Override,
    // ParametricPolymorphism = generic type parameters
    FunctionCapability::ParametricPolymorphism,
    // StaticMethod = static
    FunctionCapability::StaticMethod,
    // VariadicParameters = rest parameters
    FunctionCapability::VariadicParameters,
];
const TS_INTERFACE_FUNCTION_CAPABILITIES: &[FunctionCapability] = &[
    FunctionCapability::BoundedPolymorphism,
    FunctionCapability::ExplicitReturnType,
    FunctionCapability::ParametricPolymorphism,
    FunctionCapability::TypedParameters,
    FunctionCapability::VariadicParameters,
];
const TS_CONSTRUCTOR_CAPABILITIES: &[FunctionCapability] = &[
    FunctionCapability::ConstructorDelegation,
    FunctionCapability::ConstructorProperties,
    FunctionCapability::DefaultParameters,
    FunctionCapability::TypedParameters,
    FunctionCapability::VariadicParameters,
];
const TS_MEMBER_INCOMPATIBILITIES: &[(FunctionCapability, FunctionCapability)] = &[
    (
        FunctionCapability::AbstractMethod,
        FunctionCapability::AsyncEffect,
    ),
    (
        FunctionCapability::AbstractMethod,
        FunctionCapability::StaticMethod,
    ),
    (
        FunctionCapability::AbstractMethod,
        FunctionCapability::DefaultParameters,
    ),
];
const TS_FUNCTIONS: &[FunctionCapabilityProfile] = &[
    FunctionCapabilityProfile::new(
        FunctionContext::TopLevel,
        FunctionForm::Function,
        TS_TOP_LEVEL_FUNCTION_CAPABILITIES,
    )
    .with_body_policy(FunctionBodyPolicy::Required),
    FunctionCapabilityProfile::new(
        FunctionContext::Member,
        FunctionForm::Function,
        TS_MEMBER_FUNCTION_CAPABILITIES,
    )
    .with_incompatible_capabilities(TS_MEMBER_INCOMPATIBILITIES)
    .with_body_policy(FunctionBodyPolicy::Required),
    FunctionCapabilityProfile::new(
        FunctionContext::Member,
        FunctionForm::Constructor,
        TS_CONSTRUCTOR_CAPABILITIES,
    )
    .with_body_policy(FunctionBodyPolicy::Required),
    FunctionCapabilityProfile::new(
        FunctionContext::InterfaceMember,
        FunctionForm::Function,
        TS_INTERFACE_FUNCTION_CAPABILITIES,
    )
    .with_body_policy(FunctionBodyPolicy::Forbidden),
];

impl CodeLang for TypeScript {
    fn capabilities(&self) -> LanguageCapabilities<'_> {
        LanguageCapabilities::strict()
            .with_types(TS_TYPES)
            .with_functions(TS_FUNCTIONS)
            .with_variants(TS_VARIANTS)
            .with_fields(crate::lang::field_lowering::typescript::PROFILES)
            .with_properties(crate::lang::property_lowering::typescript::PROFILES)
    }

    fn validate_type(&self, type_: crate::lang::TypeIntent<'_>) -> Result<(), SigilStitchError> {
        crate::lang::type_lowering::typescript::validate(self, type_)
    }

    fn lower_type(
        &self,
        type_: crate::lang::ValidatedType<'_>,
    ) -> Result<Vec<CodeBlock>, SigilStitchError> {
        crate::lang::type_lowering::typescript::lower(self, type_)
    }

    fn validate_fields(
        &self,
        fields: crate::lang::FieldSequenceIntent<'_>,
    ) -> Result<(), SigilStitchError> {
        crate::lang::field_lowering::typescript::validate(self, fields)
    }

    fn collect_field_validation_errors(
        &self,
        fields: crate::lang::FieldSequenceIntent<'_>,
        errors: &mut Vec<SigilStitchError>,
    ) {
        crate::lang::field_lowering::typescript::collect_validation_errors(self, fields, errors);
    }

    fn lower_fields(
        &self,
        fields: crate::lang::ValidatedFields<'_>,
    ) -> Result<CodeBlock, SigilStitchError> {
        crate::lang::field_lowering::typescript::lower(self, fields)
    }

    fn validate_property(
        &self,
        property: crate::lang::PropertyIntent<'_>,
    ) -> Result<(), SigilStitchError> {
        crate::lang::property_lowering::typescript::validate(self, property)
    }

    fn collect_property_validation_errors(
        &self,
        property: crate::lang::PropertyIntent<'_>,
        errors: &mut Vec<SigilStitchError>,
    ) {
        crate::lang::property_lowering::typescript::collect_validation_errors(
            self, property, errors,
        );
    }

    fn lower_property(
        &self,
        property: crate::lang::ValidatedProperty<'_>,
    ) -> Result<Vec<CodeBlock>, SigilStitchError> {
        crate::lang::property_lowering::typescript::lower(self, property)
    }

    fn validate_type_members(
        &self,
        members: crate::lang::TypeMembersIntent<'_>,
    ) -> Result<(), SigilStitchError> {
        crate::lang::type_members_validation::typescript::validate(self, members)
    }

    fn collect_type_members_validation_errors(
        &self,
        members: crate::lang::TypeMembersIntent<'_>,
        errors: &mut Vec<SigilStitchError>,
    ) {
        crate::lang::type_members_validation::typescript::collect_validation_errors(
            self, members, errors,
        );
    }

    fn lower_variants(
        &self,
        variants: crate::lang::ValidatedVariants<'_>,
    ) -> Result<CodeBlock, SigilStitchError> {
        crate::lang::variant_lowering::typescript::lower(self, variants)
    }

    fn validate_function_type_constraints(
        &self,
        function_name: &str,
        type_params: &[crate::spec::where_spec::TypeParamSpec],
        constraints: &[crate::spec::where_spec::WhereConstraint],
    ) -> Result<(), SigilStitchError> {
        crate::lang::function_lowering::validate_constraints_target_declared_type_params(
            self.file_extension(),
            function_name,
            type_params,
            constraints,
        )
    }

    fn constructor_name_matches(&self, name: &str, _declaring_type: Option<&str>) -> bool {
        name == "constructor"
    }

    fn static_constructor_name_matches(&self, _name: &str, _declaring_type: Option<&str>) -> bool {
        false
    }

    fn abstract_type_modifier_is_valid(&self, kind: TypeKind) -> bool {
        matches!(kind, TypeKind::Class | TypeKind::Struct)
    }

    fn function_visibility_is_valid(
        &self,
        context: FunctionContext,
        _form: FunctionForm,
        _is_static: bool,
        visibility: Visibility,
    ) -> bool {
        match context {
            FunctionContext::TopLevel => {
                matches!(
                    visibility,
                    Visibility::Inherited | Visibility::Public | Visibility::Private
                )
            }
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

    fn function_parameters_require_trailing_defaults(
        &self,
        _context: FunctionContext,
        _form: FunctionForm,
    ) -> bool {
        true
    }

    fn lower_function(
        &self,
        function: crate::spec::fun_spec::ValidatedFunction<'_>,
    ) -> Result<CodeBlock, SigilStitchError> {
        crate::lang::typescript_function_lowering::lower(self, function)
    }

    fn constructor_name_is_valid(&self, name: &str, _declaring_type: Option<&str>) -> bool {
        name == "constructor"
    }

    fn escape_field_name(&self, name: &str) -> String {
        name.to_string()
    }

    fn render_imports(&self, imports: &ImportGroup) -> String {
        let mut lines = Vec::new();
        let quote = self.quote_char();
        let term = if self.uses_semicolons { ";" } else { "" };

        // Group entries by module path.
        let mut by_module: std::collections::BTreeMap<&str, Vec<&ImportEntry>> =
            std::collections::BTreeMap::new();
        for entry in imports.entries() {
            if entry.is_side_effect {
                lines.push(format!("import {quote}{}{quote}{term}", entry.module));
                continue;
            }
            if entry.is_wildcard {
                // TS wildcard: import * as Module from "module";
                // Use module_to_alias to generate a reasonable namespace name.
                let alias = super::module_to_alias(&entry.module);
                lines.push(format!(
                    "import * as {} from {quote}{}{quote}{term}",
                    alias, entry.module,
                ));
                continue;
            }
            by_module.entry(&entry.module).or_default().push(entry);
        }

        for (module, entries) in &by_module {
            // Separate type-only and value imports.
            let mut type_names: Vec<String> = Vec::new();
            let mut value_names: Vec<String> = Vec::new();

            for entry in entries {
                let spec = if let Some(alias) = &entry.alias {
                    format!("{} as {}", entry.name, alias)
                } else {
                    entry.name.clone()
                };
                if entry.is_type_only {
                    type_names.push(spec);
                } else {
                    value_names.push(spec);
                }
            }

            type_names.sort();
            value_names.sort();

            if !type_names.is_empty() {
                lines.push(format!(
                    "import type {{ {} }} from {quote}{}{quote}{term}",
                    type_names.join(", "),
                    module,
                ));
            }
            if !value_names.is_empty() {
                lines.push(format!(
                    "import {{ {} }} from {quote}{}{quote}{term}",
                    value_names.join(", "),
                    module,
                ));
            }
        }

        lines.join("\n")
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

    fn render_visibility(&self, vis: Visibility, ctx: DeclarationContext) -> &str {
        match ctx {
            DeclarationContext::TopLevel => match vis {
                Visibility::Public => "export ",
                _ => "",
            },
            DeclarationContext::Member | DeclarationContext::InterfaceMember => match vis {
                Visibility::Public => "public ",
                Visibility::Private => "private ",
                Visibility::Protected => "protected ",
                _ => "",
            },
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
            TypeKind::Interface | TypeKind::Trait => "interface",
            TypeKind::Enum => "enum",
            TypeKind::TypeAlias | TypeKind::Newtype => "type",
        }
    }

    fn methods_inside_type_body(&self, _kind: TypeKind) -> bool {
        true
    }

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
    fn optional_field_style(&self) -> crate::lang::config::OptionalFieldStyle {
        crate::lang::config::OptionalFieldStyle::NameSuffix("?")
    }

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
    fn function_syntax(&self) -> FunctionSyntaxConfig<'_> {
        FunctionSyntaxConfig::default()
    }

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
    fn type_decl_syntax(&self) -> TypeDeclSyntaxConfig<'_> {
        TypeDeclSyntaxConfig {
            super_type_keyword: " extends ",
            implements_keyword: " implements ",
            ..Default::default()
        }
    }

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
    fn enum_and_annotation(&self) -> EnumAndAnnotationConfig<'_> {
        EnumAndAnnotationConfig {
            readonly_keyword: "readonly ",
            variant_trailing_separator: true,
            ..Default::default()
        }
    }
}

#[cfg(test)]
#[expect(deprecated, reason = "0.6.8 compatibility assertions")]
mod tests {
    use super::*;

    #[test]
    fn test_string_literal_single_quotes() {
        let ts = TypeScript::new();
        assert_eq!(ts.render_string_literal("hello"), "'hello'");
        assert_eq!(ts.render_string_literal("it's"), "'it\\'s'");
    }

    #[test]
    fn test_string_literal_double_quotes() {
        let ts = TypeScript::new().with_quote_style(QuoteStyle::Double);
        assert_eq!(ts.render_string_literal("hello"), "\"hello\"");
    }

    #[test]
    fn test_typescript_builder_semicolons_and_extension() {
        let ts = TypeScript::new()
            .with_semicolons(false)
            .with_extension("tsx")
            .with_indent("    ");
        assert!(!ts.block_syntax().uses_semicolons);
        assert_eq!(ts.file_extension(), "tsx");
        assert_eq!(ts.block_syntax().indent_unit, "    ");

        let imports = ImportGroup {
            entries: vec![ImportEntry {
                module: "./models".to_string(),
                name: "User".to_string(),
                alias: None,
                is_type_only: false,
                is_side_effect: false,
                is_wildcard: false,
            }],
        };
        let output = ts.render_imports(&imports);
        assert!(output.contains("import { User } from './models'"));
        assert!(!output.contains(";"));
    }

    #[test]
    fn test_reserved_word_escaping() {
        let ts = TypeScript::new();
        assert_eq!(ts.escape_reserved("class"), "class_");
        assert_eq!(ts.escape_reserved("myVar"), "myVar");
        // TS 4.9+: satisfies is reserved
        assert_eq!(ts.escape_reserved("satisfies"), "satisfies_");
        // TS 5.2+: using is reserved
        assert_eq!(ts.escape_reserved("using"), "using_");
        // TS 5.5+: accessor and defer
        assert_eq!(ts.escape_reserved("accessor"), "accessor_");
        assert_eq!(ts.escape_reserved("defer"), "defer_");
        // async/await
        assert_eq!(ts.escape_reserved("async"), "async_");
        assert_eq!(ts.escape_reserved("await"), "await_");
    }

    #[test]
    fn test_render_imports() {
        let ts = TypeScript::new();
        let imports = ImportGroup {
            entries: vec![
                ImportEntry {
                    module: "./models".to_string(),
                    name: "User".to_string(),
                    alias: None,
                    is_type_only: true,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "./models".to_string(),
                    name: "UserFromJSON".to_string(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
            ],
        };
        let output = ts.render_imports(&imports);
        assert!(output.contains("import type { User } from './models'"));
        assert!(output.contains("import { UserFromJSON } from './models'"));
    }

    #[test]
    fn test_render_imports_with_alias() {
        let ts = TypeScript::new();
        let imports = ImportGroup {
            entries: vec![
                ImportEntry {
                    module: "./models".to_string(),
                    name: "User".to_string(),
                    alias: None,
                    is_type_only: true,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "./other".to_string(),
                    name: "User".to_string(),
                    alias: Some("OtherUser".to_string()),
                    is_type_only: true,
                    is_side_effect: false,
                    is_wildcard: false,
                },
            ],
        };
        let output = ts.render_imports(&imports);
        assert!(output.contains("import type { User } from './models'"));
        assert!(output.contains("import type { User as OtherUser } from './other'"));
    }

    #[test]
    fn test_doc_comment() {
        let ts = TypeScript::new();
        let doc = ts.render_doc_comment(&["Get the user by ID.", "", "Returns null if not found."]);
        assert!(doc.starts_with("/**\n"));
        assert!(doc.contains(" * Get the user by ID.\n"));
        assert!(doc.contains(" *\n"));
        assert!(doc.ends_with(" */"));
    }

    #[test]
    fn test_module_separator() {
        let ts = TypeScript::new();
        assert_eq!(ts.module_separator(), None);
    }
}
