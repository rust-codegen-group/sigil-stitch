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
    TypeDeclSyntaxConfig, TypePresentationConfig,
};
use crate::lang::{CodeLang, RendererLang};
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};
#[expect(deprecated, reason = "0.6.8 compatibility implementation")]
use crate::spec::where_spec::WhereClauseStyle;
#[expect(deprecated, reason = "0.6.8 compatibility implementation")]
use crate::type_name::{FunctionPresentation, TypePresentation, WildcardPresentation};

/// Rust language implementation.
#[derive(Debug, Clone)]
pub struct Rust {
    /// Indent with this string (default: "    ").
    pub indent: String,
    /// File extension (default: "rs").
    pub extension: String,
}

impl Default for Rust {
    fn default() -> Self {
        Self {
            indent: "    ".to_string(),
            extension: "rs".to_string(),
        }
    }
}

impl Rust {
    /// Create a new Rust language instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the indent string (e.g., `"    "` for 4-space rustfmt default, `"\t"` for tabs).
    pub fn with_indent(mut self, s: &str) -> Self {
        self.indent = s.to_string();
        self
    }

    /// Set the file extension (default: `"rs"`).
    pub fn with_extension(mut self, s: &str) -> Self {
        self.extension = s.to_string();
        self
    }
}

const RUST_RESERVED: &[&str] = &[
    // Strict keywords (2024 edition)
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern",
    "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub",
    "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true", "type",
    "unsafe", "use", "where", "while",
    // Reserved keywords (cannot be used as identifiers)
    "abstract", "become", "box", "do", "final", "gen", "macro", "override", "priv", "try", "typeof",
    "unsized", "virtual", "yield",
];

pub(crate) fn is_valid_lifetime_parameter_name(name: &str) -> bool {
    let Some(identifier) = name.strip_prefix('\'') else {
        return false;
    };
    identifier != "_"
        && identifier != "static"
        && crate::lang::type_lowering::is_identifier(identifier)
        && !RUST_RESERVED.contains(&identifier)
}

pub(crate) fn is_valid_lifetime_bound(
    bound: &crate::type_name::TypeName,
    type_params: &[crate::spec::where_spec::TypeParamSpec],
) -> bool {
    let name = match bound {
        crate::type_name::TypeName::Primitive(name) | crate::type_name::TypeName::Raw(name) => name,
        _ => return false,
    };
    name == "'static"
        || type_params.iter().any(|parameter| {
            parameter.is_lifetime()
                && parameter.name() == name
                && is_valid_lifetime_parameter_name(parameter.name())
        })
}

pub(crate) fn lifetime_constraint_subject_name(
    subject: &crate::type_name::TypeName,
) -> Result<Option<&str>, ()> {
    use crate::type_name::TypeName;

    match subject {
        TypeName::Primitive(name) | TypeName::Raw(name) if name.starts_with('\'') => Ok(Some(name)),
        _ if lifetime_head_name(subject).is_some() => Err(()),
        _ => Ok(None),
    }
}

fn lifetime_head_name(type_name: &crate::type_name::TypeName) -> Option<&str> {
    use crate::type_name::TypeName;

    match type_name {
        TypeName::Primitive(name) | TypeName::Raw(name) | TypeName::Importable { name, .. }
            if name.starts_with('\'') =>
        {
            Some(name)
        }
        TypeName::Generic { base, .. } => lifetime_head_name(base),
        _ => None,
    }
}

fn is_valid_import_alias(alias: &str) -> bool {
    let mut characters = alias.chars();
    alias != "_"
        && characters
            .next()
            .is_some_and(|character| character == '_' || unicode_ident::is_xid_start(character))
        && characters.all(unicode_ident::is_xid_continue)
        && !RUST_RESERVED.contains(&alias)
}

impl RendererLang for Rust {
    fn lower_type_name(
        &self,
        type_name: &crate::type_name::TypeName,
    ) -> Result<crate::code_block::CodeBlock, crate::error::SigilStitchError> {
        crate::lang::type_name_lowering::rust(type_name)
    }
    fn file_extension(&self) -> &str {
        &self.extension
    }

    fn reserved_words(&self) -> &[&str] {
        RUST_RESERVED
    }

    fn render_string_literal(&self, s: &str) -> String {
        format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
    }

    fn line_comment_prefix(&self) -> &str {
        "//"
    }

    fn escape_reserved(&self, name: &str) -> String {
        if self.reserved_words().contains(&name) {
            format!("r#{name}")
        } else {
            name.to_string()
        }
    }

    fn render_attribute(&self, text: &str) -> String {
        format!("#[{text}]")
    }

    fn module_separator(&self) -> Option<&str> {
        Some("::")
    }

    // --- Config struct accessors ---

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
    fn type_presentation(&self) -> TypePresentationConfig<'_> {
        TypePresentationConfig {
            array: TypePresentation::GenericWrap { name: "Vec" },
            readonly_array: Some(TypePresentation::GenericWrap { name: "Vec" }),
            optional: TypePresentation::GenericWrap { name: "Option" },
            map: TypePresentation::GenericWrap { name: "HashMap" },
            intersection: TypePresentation::Infix { sep: " + " },
            pointer: TypePresentation::Prefix { prefix: "*const " },
            slice: TypePresentation::Delimited {
                open: "&[",
                sep: "",
                close: "]",
            },
            reference: TypePresentation::Prefix { prefix: "&" },
            reference_mut: TypePresentation::Prefix { prefix: "&mut " },
            function: FunctionPresentation {
                keyword: "fn",
                params_open: "(",
                params_sep: ", ",
                params_close: ")",
                arrow: " -> ",
                return_first: false,
                curried: false,
                wrapper_open: "",
                wrapper_close: "",
            },
            wildcard: WildcardPresentation {
                unbounded: "_",
                upper_keyword: "impl ",
                lower_keyword: "impl ",
            },
            ..Default::default()
        }
    }

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
    fn generic_syntax(&self) -> GenericSyntaxConfig<'_> {
        GenericSyntaxConfig::default()
    }

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
    fn block_syntax(&self) -> BlockSyntaxConfig<'_> {
        BlockSyntaxConfig {
            indent_unit: &self.indent,
            ..Default::default()
        }
    }
}

const RUST_RECORD_CAPABILITIES: &[TypeCapability] = &[
    // RecordFields = struct fields
    TypeCapability::RecordFields,
    // Methods = impl methods
    TypeCapability::Methods,
    // ParametricPolymorphism = generic type parameters
    TypeCapability::ParametricPolymorphism,
    // BoundedPolymorphism = trait bounds
    TypeCapability::BoundedPolymorphism,
    // Attributes = `#[attr]`
    TypeCapability::Attributes,
];
const RUST_CONTRACT_CAPABILITIES: &[TypeCapability] = &[
    // Methods = impl methods
    TypeCapability::Methods,
    // ParametricPolymorphism = generic type parameters
    TypeCapability::ParametricPolymorphism,
    // BoundedPolymorphism = trait bounds
    TypeCapability::BoundedPolymorphism,
    // Attributes = `#[attr]`
    TypeCapability::Attributes,
];
const RUST_TYPES: &[TypeCapabilityProfile] = &[
    TypeCapabilityProfile::new(TypeKind::Struct, RUST_RECORD_CAPABILITIES),
    // Class is represented as a Rust struct plus an impl block.
    TypeCapabilityProfile::new(TypeKind::Class, RUST_RECORD_CAPABILITIES),
    TypeCapabilityProfile::new(TypeKind::Trait, RUST_CONTRACT_CAPABILITIES),
    // Interface is represented as a Rust trait.
    TypeCapabilityProfile::new(TypeKind::Interface, RUST_CONTRACT_CAPABILITIES),
    TypeCapabilityProfile::new(
        TypeKind::Enum,
        &[
            // Methods = impl methods
            TypeCapability::Methods,
            // ParametricPolymorphism = generic type parameters
            TypeCapability::ParametricPolymorphism,
            // BoundedPolymorphism = trait bounds
            TypeCapability::BoundedPolymorphism,
            // Attributes = `#[attr]`
            TypeCapability::Attributes,
            // Variants = enum variants
            TypeCapability::Variants,
        ],
    ),
    TypeCapabilityProfile::new(
        TypeKind::TypeAlias,
        &[
            // ParametricPolymorphism = generic type parameters
            TypeCapability::ParametricPolymorphism,
        ],
    ),
    TypeCapabilityProfile::new(
        TypeKind::Newtype,
        &[
            // ParametricPolymorphism = generic type parameters
            TypeCapability::ParametricPolymorphism,
            // BoundedPolymorphism = trait bounds
            TypeCapability::BoundedPolymorphism,
            // Attributes = `#[attr]`
            TypeCapability::Attributes,
        ],
    ),
];

const RUST_VARIANTS: &[VariantCapabilityProfile] = &[VariantCapabilityProfile::new(
    TypeKind::Enum,
    &[
        VariantCapability::Discriminant,
        VariantCapability::PositionalPayload,
        VariantCapability::RecordPayload,
        VariantCapability::Attributes,
    ],
)];

const RUST_TOP_LEVEL_FUNCTION_CAPABILITIES: &[FunctionCapability] = &[
    // AsyncEffect = async fn
    FunctionCapability::AsyncEffect,
    // Attributes = attributes #[...]
    FunctionCapability::Attributes,
    // BoundedPolymorphism = trait bounds
    FunctionCapability::BoundedPolymorphism,
    // ExplicitReturnType = function result type
    FunctionCapability::ExplicitReturnType,
    FunctionCapability::TypedParameters,
    // ParametricPolymorphism = generic type parameters
    FunctionCapability::ParametricPolymorphism,
];
const RUST_MEMBER_FUNCTION_CAPABILITIES: &[FunctionCapability] =
    RUST_TOP_LEVEL_FUNCTION_CAPABILITIES;
const RUST_FUNCTIONS: &[FunctionCapabilityProfile] = &[
    FunctionCapabilityProfile::new(
        FunctionContext::TopLevel,
        FunctionForm::Function,
        RUST_TOP_LEVEL_FUNCTION_CAPABILITIES,
    )
    .with_required_capabilities(&[FunctionCapability::TypedParameters])
    .with_body_policy(FunctionBodyPolicy::Required),
    FunctionCapabilityProfile::new(
        FunctionContext::TopLevel,
        FunctionForm::Constructor,
        RUST_TOP_LEVEL_FUNCTION_CAPABILITIES,
    )
    .with_required_capabilities(&[FunctionCapability::TypedParameters])
    .with_body_policy(FunctionBodyPolicy::Required),
    FunctionCapabilityProfile::new(
        FunctionContext::Member,
        FunctionForm::Function,
        RUST_MEMBER_FUNCTION_CAPABILITIES,
    )
    .with_required_capabilities(&[FunctionCapability::TypedParameters])
    .with_body_policy(FunctionBodyPolicy::Required),
    FunctionCapabilityProfile::new(
        FunctionContext::Member,
        FunctionForm::Constructor,
        RUST_MEMBER_FUNCTION_CAPABILITIES,
    )
    .with_required_capabilities(&[FunctionCapability::TypedParameters])
    .with_body_policy(FunctionBodyPolicy::Required),
    FunctionCapabilityProfile::new(
        FunctionContext::InterfaceMember,
        FunctionForm::Function,
        RUST_MEMBER_FUNCTION_CAPABILITIES,
    )
    .with_required_capabilities(&[FunctionCapability::TypedParameters]),
    FunctionCapabilityProfile::new(
        FunctionContext::InterfaceMember,
        FunctionForm::Constructor,
        RUST_MEMBER_FUNCTION_CAPABILITIES,
    )
    .with_required_capabilities(&[FunctionCapability::TypedParameters]),
];

impl CodeLang for Rust {
    fn validate_resolved_imports(
        &self,
        imports: &crate::import::ImportGroup,
    ) -> Result<(), crate::error::SigilStitchError> {
        crate::lang::import_validation::validate_identifier_aliases(
            self,
            imports,
            is_valid_import_alias,
        )
    }
    fn capabilities(&self) -> LanguageCapabilities<'_> {
        LanguageCapabilities::strict()
            .with_types(RUST_TYPES)
            .with_functions(RUST_FUNCTIONS)
            .with_variants(RUST_VARIANTS)
            .with_fields(crate::lang::field_lowering::rust::PROFILES)
    }

    fn validate_type(&self, type_: crate::lang::TypeIntent<'_>) -> Result<(), SigilStitchError> {
        crate::lang::type_lowering::rust::validate(self, type_)
    }

    fn lower_type(
        &self,
        type_: crate::lang::ValidatedType<'_>,
    ) -> Result<Vec<CodeBlock>, SigilStitchError> {
        crate::lang::type_lowering::rust::lower(self, type_)
    }

    fn lower_function(
        &self,
        function: crate::spec::fun_spec::ValidatedFunction<'_>,
    ) -> Result<CodeBlock, SigilStitchError> {
        crate::lang::rust_function_lowering::lower(self, function)
    }

    fn validate_function_type_constraints(
        &self,
        function_name: &str,
        type_params: &[crate::spec::where_spec::TypeParamSpec],
        constraints: &[crate::spec::where_spec::WhereConstraint],
    ) -> Result<(), SigilStitchError> {
        for parameter in type_params {
            if parameter.is_lifetime() {
                if !is_valid_lifetime_parameter_name(parameter.name()) {
                    return Err(SigilStitchError::InvalidFunctionTypeParameter {
                        language: self.file_extension().to_string(),
                        function_name: function_name.to_string(),
                        parameter_name: parameter.name().to_string(),
                        reason:
                            "Rust lifetime parameters require a valid non-keyword declared name"
                                .to_string(),
                    });
                }
                if parameter
                    .bounds()
                    .iter()
                    .any(|bound| !is_valid_lifetime_bound(bound, type_params))
                {
                    return Err(SigilStitchError::InvalidFunctionTypeParameter {
                        language: self.file_extension().to_string(),
                        function_name: function_name.to_string(),
                        parameter_name: parameter.name().to_string(),
                        reason: "Rust lifetime parameters accept only declared lifetime or 'static bounds"
                            .to_string(),
                    });
                }
            } else if !crate::lang::type_lowering::is_identifier(parameter.name())
                || parameter.name().starts_with('\'')
                || self.reserved_words().contains(&parameter.name())
            {
                return Err(SigilStitchError::InvalidFunctionTypeParameter {
                    language: self.file_extension().to_string(),
                    function_name: function_name.to_string(),
                    parameter_name: parameter.name().to_string(),
                    reason: "Rust type parameters require an ordinary non-keyword identifier"
                        .to_string(),
                });
            }
            if !parameter.context_bounds().is_empty() {
                return Err(SigilStitchError::InvalidFunctionTypeParameter {
                    language: self.file_extension().to_string(),
                    function_name: function_name.to_string(),
                    parameter_name: parameter.name().to_string(),
                    reason:
                        "Rust function type parameters do not support Scala-style context bounds"
                            .to_string(),
                });
            }
        }
        for constraint in constraints {
            let subject = match lifetime_constraint_subject_name(constraint.subject()) {
                Ok(Some(subject)) => subject,
                Ok(None) => continue,
                Err(()) => {
                    return Err(SigilStitchError::InvalidFunctionConstraintSubject {
                        language: self.file_extension().to_string(),
                        function_name: function_name.to_string(),
                        subject: format!("{:?}", constraint.subject()),
                    });
                }
            };
            if !type_params.iter().any(|parameter| {
                parameter.is_lifetime()
                    && parameter.name() == subject
                    && is_valid_lifetime_parameter_name(parameter.name())
            }) {
                return Err(SigilStitchError::InvalidFunctionConstraintSubject {
                    language: self.file_extension().to_string(),
                    function_name: function_name.to_string(),
                    subject: subject.to_string(),
                });
            }
            if constraint
                .bounds()
                .iter()
                .any(|bound| !is_valid_lifetime_bound(bound, type_params))
            {
                return Err(SigilStitchError::InvalidFunctionTypeParameter {
                    language: self.file_extension().to_string(),
                    function_name: function_name.to_string(),
                    parameter_name: subject.to_string(),
                    reason:
                        "Rust lifetime constraints accept only declared lifetime or 'static bounds"
                            .to_string(),
                });
            }
        }
        Ok(())
    }

    fn validate_fields(
        &self,
        fields: crate::lang::FieldSequenceIntent<'_>,
    ) -> Result<(), crate::error::SigilStitchError> {
        crate::lang::field_lowering::rust::validate(self, fields)
    }

    fn collect_field_validation_errors(
        &self,
        fields: crate::lang::FieldSequenceIntent<'_>,
        errors: &mut Vec<SigilStitchError>,
    ) {
        crate::lang::field_lowering::rust::collect_validation_errors(self, fields, errors);
    }

    fn lower_fields(
        &self,
        fields: crate::lang::ValidatedFields<'_>,
    ) -> Result<CodeBlock, SigilStitchError> {
        crate::lang::field_lowering::rust::lower(self, fields)
    }

    fn validate_variants(
        &self,
        variants: crate::lang::VariantIntent<'_>,
    ) -> Result<(), crate::error::SigilStitchError> {
        crate::lang::variant_lowering::rust::validate(self, variants)
    }

    fn collect_variant_validation_errors(
        &self,
        variants: crate::lang::VariantIntent<'_>,
        errors: &mut Vec<crate::error::SigilStitchError>,
    ) {
        crate::lang::variant_lowering::rust::collect_validation_errors(variants, errors);
    }

    fn lower_variants(
        &self,
        variants: crate::lang::ValidatedVariants<'_>,
    ) -> Result<crate::code_block::CodeBlock, crate::error::SigilStitchError> {
        crate::lang::variant_lowering::rust::lower(self, variants)
    }

    fn function_visibility_is_valid(
        &self,
        context: FunctionContext,
        _form: FunctionForm,
        _is_static: bool,
        visibility: Visibility,
    ) -> bool {
        match context {
            FunctionContext::TopLevel | FunctionContext::Member => {
                !matches!(visibility, Visibility::Protected)
            }
            FunctionContext::InterfaceMember => {
                matches!(visibility, Visibility::Inherited | Visibility::Public)
            }
            FunctionContext::ReceiverMethod => false,
        }
    }

    fn function_parameters_are_typed(
        &self,
        parameters: &[crate::spec::parameter_spec::ParameterSpec],
        context: FunctionContext,
        _form: FunctionForm,
    ) -> bool {
        parameters.iter().all(|parameter| {
            !parameter.param_type().is_empty()
                || (matches!(
                    context,
                    FunctionContext::Member | FunctionContext::InterfaceMember
                ) && matches!(
                    parameter.name(),
                    "self" | "mut self" | "&self" | "&mut self"
                ))
        })
    }

    fn render_imports(&self, imports: &ImportGroup) -> String {
        if imports.entries().is_empty() {
            return String::new();
        }

        let mut lines = Vec::new();

        // Handle side-effect and wildcard imports first.
        for entry in imports.entries() {
            if entry.is_wildcard {
                lines.push(format!("use {}::*;", entry.module));
            } else if entry.is_side_effect {
                lines.push(format!("use {};", entry.module));
            }
        }

        // Group named imports by crate origin: std/core first, then external, then crate::.
        let mut std_imports: Vec<&ImportEntry> = Vec::new();
        let mut external_imports: Vec<&ImportEntry> = Vec::new();
        let mut crate_imports: Vec<&ImportEntry> = Vec::new();

        for entry in imports.entries() {
            if entry.is_side_effect || entry.is_wildcard {
                continue;
            }
            if entry.module.starts_with("std::")
                || entry.module.starts_with("core::")
                || entry.module == "std"
                || entry.module == "core"
            {
                std_imports.push(entry);
            } else if entry.module.starts_with("crate::")
                || entry.module.starts_with("super::")
                || entry.module.starts_with("self::")
            {
                crate_imports.push(entry);
            } else {
                external_imports.push(entry);
            }
        }

        // Group imports from the same module into `use mod::{A, B}` form.
        fn render_group(entries: &[&ImportEntry], lines: &mut Vec<String>) {
            let mut by_module: std::collections::BTreeMap<&str, Vec<&ImportEntry>> =
                std::collections::BTreeMap::new();
            for entry in entries {
                by_module.entry(&entry.module).or_default().push(entry);
            }
            for (module, items) in &by_module {
                if items.len() == 1 {
                    let entry = items[0];
                    if let Some(alias) = &entry.alias {
                        lines.push(format!("use {module}::{} as {alias};", entry.name));
                    } else {
                        lines.push(format!("use {module}::{};", entry.name));
                    }
                } else {
                    let mut specs: Vec<String> = items
                        .iter()
                        .map(|e| {
                            if let Some(alias) = &e.alias {
                                format!("{} as {alias}", e.name)
                            } else {
                                e.name.clone()
                            }
                        })
                        .collect();
                    specs.sort();
                    lines.push(format!("use {module}::{{{}}};", specs.join(", ")));
                }
            }
        }

        if !std_imports.is_empty() {
            render_group(&std_imports, &mut lines);
        }
        if !external_imports.is_empty() {
            if !lines.is_empty() {
                lines.push(String::new());
            }
            render_group(&external_imports, &mut lines);
        }
        if !crate_imports.is_empty() {
            if !lines.is_empty() {
                lines.push(String::new());
            }
            render_group(&crate_imports, &mut lines);
        }

        lines.join("\n")
    }

    fn render_doc_comment(&self, lines: &[&str]) -> String {
        lines
            .iter()
            .map(|line| {
                if line.is_empty() {
                    "///".to_string()
                } else {
                    format!("/// {line}")
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn render_visibility(&self, vis: Visibility, ctx: DeclarationContext) -> &str {
        if ctx == DeclarationContext::InterfaceMember {
            return "";
        }
        match vis {
            Visibility::Inherited => "",
            Visibility::Public => "pub ",
            Visibility::PublicCrate => "pub(crate) ",
            Visibility::PublicSuper => "pub(super) ",
            // Rust has no private/protected keyword; absence of pub = private.
            Visibility::Private | Visibility::Protected => "",
        }
    }

    fn function_keyword(&self, _ctx: DeclarationContext) -> &str {
        "fn"
    }

    fn type_keyword(&self, kind: TypeKind) -> &str {
        match kind {
            TypeKind::Struct | TypeKind::Class => "struct",
            TypeKind::Trait | TypeKind::Interface => "trait",
            TypeKind::Enum => "enum",
            TypeKind::TypeAlias => "type",
            TypeKind::Newtype => "struct",
        }
    }

    fn methods_inside_type_body(&self, kind: TypeKind) -> bool {
        match kind {
            TypeKind::Trait | TypeKind::Interface => true,
            TypeKind::Struct
            | TypeKind::Class
            | TypeKind::Enum
            | TypeKind::TypeAlias
            | TypeKind::Newtype => false,
        }
    }

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
    fn optional_field_style(&self) -> crate::lang::config::OptionalFieldStyle {
        crate::lang::config::OptionalFieldStyle::TypeWrap {
            open: "Option<",
            close: ">",
        }
    }

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
    fn function_syntax(&self) -> FunctionSyntaxConfig<'_> {
        FunctionSyntaxConfig {
            return_type_separator: " -> ",
            constructor_keyword: "fn",
            where_clause_style: WhereClauseStyle::WhereBlock,
            ..Default::default()
        }
    }

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
    fn type_decl_syntax(&self) -> TypeDeclSyntaxConfig<'_> {
        TypeDeclSyntaxConfig::default()
    }

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
    fn enum_and_annotation(&self) -> EnumAndAnnotationConfig<'_> {
        EnumAndAnnotationConfig {
            variant_trailing_separator: true,
            annotation_prefix: "#[",
            annotation_suffix: "]",
            ..Default::default()
        }
    }
}

#[cfg(test)]
#[expect(deprecated, reason = "0.6.8 compatibility assertions")]
mod tests {
    use super::*;

    #[test]
    fn test_file_extension() {
        let rs = Rust::new();
        assert_eq!(rs.file_extension(), "rs");
    }

    #[test]
    fn test_escape_reserved() {
        let rs = Rust::new();
        assert_eq!(rs.escape_reserved("type"), "r#type");
        assert_eq!(rs.escape_reserved("my_var"), "my_var");
        // 2024 edition: gen is reserved
        assert_eq!(rs.escape_reserved("gen"), "r#gen");
    }

    #[test]
    fn test_render_imports_grouped() {
        let rs = Rust::new();
        let imports = ImportGroup {
            entries: vec![
                ImportEntry {
                    module: "std::collections".into(),
                    name: "HashMap".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "std::collections".into(),
                    name: "BTreeMap".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "serde".into(),
                    name: "Serialize".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "crate::models".into(),
                    name: "User".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
            ],
        };
        let output = rs.render_imports(&imports);
        let lines: Vec<&str> = output.lines().collect();
        // std group first
        assert_eq!(lines[0], "use std::collections::{BTreeMap, HashMap};");
        // blank line
        assert_eq!(lines[1], "");
        // external
        assert_eq!(lines[2], "use serde::Serialize;");
        // blank line
        assert_eq!(lines[3], "");
        // crate
        assert_eq!(lines[4], "use crate::models::User;");
    }

    #[test]
    fn test_render_imports_with_alias() {
        let rs = Rust::new();
        let imports = ImportGroup {
            entries: vec![ImportEntry {
                module: "models".into(),
                name: "User".into(),
                alias: Some("ModelsUser".into()),
                is_type_only: false,
                is_side_effect: false,
                is_wildcard: false,
            }],
        };
        let output = rs.render_imports(&imports);
        assert_eq!(output, "use models::User as ModelsUser;");
    }

    #[test]
    fn test_doc_comment() {
        let rs = Rust::new();
        let doc = rs.render_doc_comment(&["Get the user.", "", "Returns None if not found."]);
        assert!(doc.contains("/// Get the user."));
        assert!(doc.contains("///\n"));
        assert!(doc.contains("/// Returns None if not found."));
    }

    #[test]
    fn test_string_literal() {
        let rs = Rust::new();
        assert_eq!(rs.render_string_literal("hello"), "\"hello\"");
        assert_eq!(rs.render_string_literal("it\"s"), "\"it\\\"s\"");
    }

    #[test]
    fn test_rust_builder_fluent() {
        let rs = Rust::new().with_indent("\t").with_extension("rsi");
        assert_eq!(rs.file_extension(), "rsi");
        assert_eq!(rs.block_syntax().indent_unit, "\t");
    }

    #[test]
    fn test_module_separator() {
        let rs = Rust::new();
        assert_eq!(rs.module_separator(), Some("::"));
    }
}
