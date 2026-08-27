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
/// methods, e.g. `TypeScript::new().with_double_quotes()`.
#[derive(Debug, Clone)]
pub struct TypeScript {
    /// Quote style for string literals and import paths.
    #[deprecated(
        note = "legacy 0.6.8 field; use TypeScript::with_single_quotes() or TypeScript::with_double_quotes()"
    )]
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
    #[deprecated(
        note = "legacy 0.6.8 setter; use TypeScript::with_single_quotes() or TypeScript::with_double_quotes()"
    )]
    #[expect(deprecated, reason = "0.6.8 compatibility setter")]
    pub fn with_quote_style(mut self, qs: QuoteStyle) -> Self {
        self.quote_style = qs;
        self
    }

    /// Use single quotes for string literals and import paths.
    #[expect(deprecated, reason = "updates the 0.6.8 compatibility field")]
    pub fn with_single_quotes(mut self) -> Self {
        self.quote_style = QuoteStyle::Single;
        self
    }

    /// Use double quotes for string literals and import paths.
    #[expect(deprecated, reason = "updates the 0.6.8 compatibility field")]
    pub fn with_double_quotes(mut self) -> Self {
        self.quote_style = QuoteStyle::Double;
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

fn is_valid_import_binding(binding: &str) -> bool {
    let mut characters = binding.chars();
    characters.next().is_some_and(|character| {
        character == '_' || character == '$' || unicode_id_start::is_id_start(character)
    }) && characters.all(|character| {
        character == '$'
            || character == '\u{200c}'
            || character == '\u{200d}'
            || unicode_id_start::is_id_continue(character)
    }) && !TS_RESERVED.contains(&binding)
}

fn module_to_namespace_alias(module: &str) -> String {
    let last_segment = module
        .rsplit(['/', ':', '.', '\\'])
        .find(|segment| !segment.is_empty())
        .unwrap_or(module);

    let mut characters = last_segment.chars();
    match characters.next() {
        None => "Module".to_string(),
        Some(first) => {
            let upper: String = first.to_uppercase().collect();
            format!("{upper}{}", characters.as_str())
        }
    }
}

fn validate_import_bindings(
    lang: &TypeScript,
    imports: &ImportGroup,
) -> Result<(), SigilStitchError> {
    let mut bindings = std::collections::HashSet::new();
    for entry in imports.entries() {
        if entry.is_side_effect {
            continue;
        }
        let binding = if entry.is_wildcard {
            module_to_namespace_alias(&entry.module)
        } else {
            entry.resolved_name().to_string()
        };
        if !is_valid_import_binding(&binding) {
            return Err(SigilStitchError::InvalidResolvedImports {
                language: lang.file_extension().to_string(),
                reason: format!("TypeScript import binding {binding:?} is not a valid identifier"),
            });
        }
        if !bindings.insert(binding.clone()) {
            return Err(SigilStitchError::InvalidResolvedImports {
                language: lang.file_extension().to_string(),
                reason: format!(
                    "multiple TypeScript imports produce the local binding {binding:?}"
                ),
            });
        }
    }
    Ok(())
}

impl RendererLang for TypeScript {
    fn lower_type_name(
        &self,
        type_name: &crate::type_name::TypeName,
    ) -> Result<crate::code_block::CodeBlock, crate::error::SigilStitchError> {
        crate::lang::type_name_lowering::typescript(type_name)
    }
    fn file_extension(&self) -> &str {
        &self.extension
    }

    fn reserved_words(&self) -> &[&str] {
        TS_RESERVED
    }

    fn render_string_literal(&self, s: &str) -> String {
        let quote = self.quote_char();
        let mut escaped = String::with_capacity(s.len() + 2);
        escaped.push(quote);
        for ch in s.chars() {
            match ch {
                '\\' => escaped.push_str("\\\\"),
                value if value == quote => {
                    escaped.push('\\');
                    escaped.push(value);
                }
                '\u{0008}' => escaped.push_str("\\b"),
                '\t' => escaped.push_str("\\t"),
                '\n' => escaped.push_str("\\n"),
                '\u{000B}' => escaped.push_str("\\v"),
                '\u{000C}' => escaped.push_str("\\f"),
                '\r' => escaped.push_str("\\r"),
                value @ ('\u{0000}'..='\u{001F}' | '\u{007F}'..='\u{009F}') => {
                    escaped.push_str(&format!("\\x{:02X}", value as u32));
                }
                value @ ('\u{2028}' | '\u{2029}') => {
                    escaped.push_str(&format!("\\u{:04X}", value as u32));
                }
                value => escaped.push(value),
            }
        }
        escaped.push(quote);
        escaped
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
    fn validate_resolved_imports(
        &self,
        imports: &crate::import::ImportGroup,
    ) -> Result<(), crate::error::SigilStitchError> {
        validate_import_bindings(self, imports)
    }
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
        if let Some(parameter) = type_params.iter().find(|parameter| {
            parameter.is_lifetime()
                || !crate::lang::type_lowering::typescript::is_identifier(parameter.name())
                || self.reserved_words().contains(&parameter.name())
        }) {
            return Err(SigilStitchError::InvalidFunctionTypeParameter {
                language: self.file_extension().to_string(),
                function_name: function_name.to_string(),
                parameter_name: parameter.name().to_string(),
                reason: "TypeScript type parameters require an ordinary non-keyword identifier"
                    .to_string(),
            });
        }
        if let Some(parameter) = type_params
            .iter()
            .find(|parameter| !parameter.context_bounds().is_empty())
        {
            return Err(SigilStitchError::InvalidFunctionTypeParameter {
                language: self.file_extension().to_string(),
                function_name: function_name.to_string(),
                parameter_name: parameter.name().to_string(),
                reason: "TypeScript function type parameters do not support context bounds"
                    .to_string(),
            });
        }
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
        let term = if self.uses_semicolons { ";" } else { "" };

        // Group entries by module path.
        let mut by_module: std::collections::BTreeMap<&str, Vec<&ImportEntry>> =
            std::collections::BTreeMap::new();
        for entry in imports.entries() {
            if entry.is_side_effect {
                lines.push(format!(
                    "import {}{term}",
                    self.render_string_literal(&entry.module)
                ));
                continue;
            }
            if entry.is_wildcard {
                // TS wildcard: import * as Module from "module";
                let alias = module_to_namespace_alias(&entry.module);
                lines.push(format!(
                    "import * as {} from {}{term}",
                    alias,
                    self.render_string_literal(&entry.module),
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
                    "import type {{ {} }} from {}{term}",
                    type_names.join(", "),
                    self.render_string_literal(module),
                ));
            }
            if !value_names.is_empty() {
                lines.push(format!(
                    "import {{ {} }} from {}{term}",
                    value_names.join(", "),
                    self.render_string_literal(module),
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
        let ts = TypeScript::new().with_double_quotes();
        assert_eq!(ts.render_string_literal("hello"), "\"hello\"");
    }

    #[test]
    fn quote_selection_apis_are_equivalent() {
        let input = "'\"\\\n";
        let legacy_setter = TypeScript::new().with_quote_style(QuoteStyle::Double);
        let mut legacy_field = TypeScript::new();
        legacy_field.quote_style = QuoteStyle::Double;
        let convenience = TypeScript::new().with_double_quotes();

        assert_eq!(
            legacy_setter.render_string_literal(input),
            legacy_field.render_string_literal(input)
        );
        assert_eq!(
            legacy_setter.render_string_literal(input),
            convenience.render_string_literal(input)
        );
        assert_eq!(
            TypeScript::new()
                .with_single_quotes()
                .render_string_literal(input),
            TypeScript::new().render_string_literal(input)
        );
    }

    #[test]
    fn string_literals_escape_source_controls_without_rewriting_unicode() {
        let ts = TypeScript::new();
        assert_eq!(ts.render_string_literal(""), "''");
        assert_eq!(ts.render_string_literal("\0"), "'\\x00'");
        assert_eq!(
            ts.render_string_literal("\0\u{0001}\u{0008}\t\n\u{000B}\u{000C}\r\u{001F}\u{007F}\u{0085}\u{2028}\u{2029}雪😀"),
            "'\\x00\\x01\\b\\t\\n\\v\\f\\r\\x1F\\x7F\\x85\\u2028\\u2029雪😀'"
        );
        assert_eq!(
            ts.render_string_literal("\0\u{0037}\u{0001}A$#{value}"),
            "'\\x007\\x01A$#{value}'"
        );
        assert_eq!(
            TypeScript::new()
                .with_double_quotes()
                .render_string_literal("'\"\\\r"),
            "\"'\\\"\\\\\\r\""
        );
    }

    #[test]
    fn string_singleton_types_preserve_precedence_and_escaping() {
        let ts = TypeScript::new();
        let literal = crate::type_name::TypeName::string_literal("a'\\\n");
        let ordinary = CodeBlock::of(
            "%S",
            (crate::code_block::StringLitArg("a'\\\n".to_string()),),
        )
        .unwrap()
        .render_standalone(&ts, 80)
        .unwrap();
        let singleton = CodeBlock::of("%T", (literal.clone(),))
            .unwrap()
            .render_standalone(&ts, 80)
            .unwrap();
        assert_eq!(singleton, ordinary);
        assert_eq!(singleton, "'a\\'\\\\\\n'");

        let union = crate::type_name::TypeName::union(vec![
            crate::type_name::TypeName::string_literal("active"),
            crate::type_name::TypeName::string_literal("inactive"),
        ]);
        assert_eq!(
            CodeBlock::of("%T", (union,))
                .unwrap()
                .render_standalone(&ts, 80)
                .unwrap(),
            "'active' | 'inactive'"
        );

        let array =
            crate::type_name::TypeName::array(crate::type_name::TypeName::string_literal("active"));
        assert_eq!(
            CodeBlock::of("%T", (array,))
                .unwrap()
                .render_standalone(&ts, 80)
                .unwrap(),
            "('active')[]"
        );

        let function = crate::type_name::TypeName::function(
            vec![crate::type_name::TypeName::string_literal("input")],
            crate::type_name::TypeName::string_literal("output"),
        );
        assert_eq!(
            CodeBlock::of("%T", (function,))
                .unwrap()
                .render_standalone(&ts, 80)
                .unwrap(),
            "(arg0: 'input') => 'output'"
        );

        let member = crate::type_name::TypeName::member_type(
            crate::type_name::TypeName::primitive("Value"),
            "type'\\path",
        );
        assert_eq!(
            CodeBlock::of("%T", (member,))
                .unwrap()
                .render_standalone(&ts, 80)
                .unwrap(),
            "Value['type\\'\\\\path']"
        );
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
    fn import_paths_use_language_owned_string_escaping() {
        let ts = TypeScript::new().with_double_quotes();
        let imports = ImportGroup {
            entries: vec![ImportEntry {
                module: "./path\\segment\t\u{2028}".into(),
                name: "Value".into(),
                alias: None,
                is_type_only: true,
                is_side_effect: false,
                is_wildcard: false,
            }],
        };
        assert_eq!(
            ts.render_imports(&imports),
            "import type { Value } from \"./path\\\\segment\\t\\u2028\";"
        );
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
