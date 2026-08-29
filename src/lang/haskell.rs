//! Haskell language implementation.

use crate::code_block::{Arg, CodeBlock};
use crate::code_node::{BlockIntent, CodeNode};
use crate::error::SigilStitchError;
use crate::import::{ImportEntry, ImportGroup};
use crate::lang::capability::{
    FunctionBodyPolicy, FunctionCapability, FunctionCapabilityProfile, FunctionContext,
    FunctionForm, LanguageCapabilities, TypeCapability, TypeCapabilityProfile, VariantCapability,
    VariantCapabilityProfile,
};
use crate::lang::{CodeLang, RendererLang};
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};
use crate::spec::where_spec::TypeParamSpec;
use crate::type_name::TypeName;

fn haskell_block_open_for_intent(intent: BlockIntent) -> Option<&'static str> {
    match intent {
        BlockIntent::Class | BlockIntent::Instance => Some(" where"),
        BlockIntent::Do => Some(""),
        BlockIntent::If | BlockIntent::ElseIf => Some(" then"),
        BlockIntent::Else => Some(""),
        BlockIntent::Case => Some(" of"),
        _ => None,
    }
}

fn haskell_block_open_from_condition_text(condition: &str) -> Option<&'static str> {
    let trimmed = condition.trim();
    if trimmed.starts_with("class ") || trimmed.starts_with("instance ") {
        Some(" where")
    } else if trimmed == "do" || trimmed.ends_with(" do") {
        Some("")
    } else if trimmed.starts_with("if ") || trimmed.starts_with("else if ") {
        Some(" then")
    } else if trimmed == "else" {
        Some("")
    } else if trimmed.starts_with("case ") {
        Some(" of")
    } else {
        None
    }
}

/// Haskell language implementation.
///
/// Haskell-specific behaviors:
/// - Prefix generic application (juxtaposition): `Maybe Int`, `Either String Int`
/// - No function keyword (type signatures use `::`, definitions have no keyword)
/// - `import Module (Name1, Name2)` for imports, with `qualified` imports for
///   conflicting names
/// - No semicolons (indentation-based)
/// - `data` for structs/classes/enums, `class` for type classes, `type` for aliases,
///   `newtype` for newtypes
/// - Record fields terminated with `,`
/// - Haddock doc comments: `-- | line1` / `--   line2`
/// - Line comments with `--`
/// - Curried function types: `Int -> String -> Bool`
/// - List type: `[Int]`
/// - Visibility controlled via module exports, not keywords
///
/// # Import conventions
///
/// Use [`crate::type_name::TypeName::importable`] with the module and name:
/// ```text
/// TypeName::importable("Data.Map", "Map")
/// TypeName::importable("Data.Text", "Text")
/// TypeName::importable("Control.Monad", "when")
/// ```
///
/// # Prefix generics
///
/// Haskell uses prefix generic application (juxtaposition):
/// - `Maybe Int`, `Either String Int`
/// - `Map String (Maybe Int)`
///
/// This is handled automatically via `generic_application_style() -> PrefixJuxtaposition`.
///
/// # Known limitations
///
/// - [`RendererLang::render_block_open`] uses `" ="` as its generic fallback for
///   function definitions and type aliases, while class and instance intents
///   get `" where"` through Haskell's complete renderer-event implementation.
/// - Complex multi-param type class constraints (e.g., `MonadReader Env m`) are not
///   directly modeled. Use `TypeName::primitive("(MonadIO m, MonadReader Env m) => m String")`
///   for complex constrained return types.
#[derive(Debug, Clone)]
pub struct Haskell {
    /// Indent with this string (default: "  " — 2 spaces).
    pub indent: String,
    /// File extension (default: "hs").
    pub extension: String,
}

impl Default for Haskell {
    fn default() -> Self {
        Self {
            indent: "  ".to_string(),
            extension: "hs".to_string(),
        }
    }
}

impl Haskell {
    /// Create a new Haskell language instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the indent string (e.g., `"  "` for 2-space default, `"\t"` for tabs).
    pub fn with_indent(mut self, s: &str) -> Self {
        self.indent = s.to_string();
        self
    }

    /// Set the file extension (default: `"hs"`). Set to `"lhs"` for literate Haskell.
    pub fn with_extension(mut self, s: &str) -> Self {
        self.extension = s.to_string();
        self
    }

    /// Fix `$` operator spacing: `$word` → `$ word`.
    ///
    /// The `$$` escape in `sigil_quote!` emits a literal `$` with space suppression
    /// (designed for `$1` in shell). In Haskell, `$` is an infix operator needing
    /// a space after it. This pass inserts the missing space when `$` is directly
    /// followed by a word character.
    #[allow(clippy::ptr_arg)]
    fn rewrite_dollar_spacing(nodes: &mut Vec<CodeNode>) {
        for node in nodes.iter_mut() {
            let text = match node {
                CodeNode::Literal(s) => s,
                CodeNode::InlineLiteral(s) => s,
                _ => continue,
            };
            if !text.contains('$') {
                continue;
            }
            let mut result = String::with_capacity(text.len() + 4);
            let chars: Vec<char> = text.chars().collect();
            let mut in_string = false;
            for (i, &ch) in chars.iter().enumerate() {
                result.push(ch);
                if ch == '"' {
                    in_string = !in_string;
                    continue;
                }
                if in_string || ch != '$' || i + 1 >= chars.len() {
                    continue;
                }
                let after = chars[i + 1];
                if after.is_alphanumeric() || after == '_' || after == '(' {
                    result.push(' ');
                }
            }
            if result != *text {
                *text = result;
            }
        }
    }
}

#[rustfmt::skip]
const HASKELL_RESERVED: &[&str] = &[
    "as", "case", "class", "data", "default", "deriving", "do", "else",
    "forall", "foreign", "hiding", "if", "import", "in", "infix",
    "infixl", "infixr", "instance", "let", "module", "newtype", "of",
    "qualified", "then", "type", "where",
];

fn is_valid_import_alias(alias: &str) -> bool {
    let mut characters = alias.chars();
    characters.next().is_some_and(char::is_uppercase)
        && characters.all(|character| {
            character == '_' || character == '\'' || unicode_ident::is_xid_continue(character)
        })
        && !HASKELL_RESERVED.contains(&alias)
}

/// Classify an import module for ordering.
/// 0 = base/Prelude, 1 = standard libs (Data.*, Control.*, System.*), 2 = everything else.
fn import_group_order(module: &str) -> u8 {
    if module == "Prelude"
        || module.starts_with("Prelude.")
        || module == "GHC.Base"
        || module.starts_with("GHC.")
    {
        0
    } else if module.starts_with("Data.")
        || module.starts_with("Control.")
        || module.starts_with("System.")
    {
        1
    } else {
        2
    }
}

#[deny(deprecated)]
impl RendererLang for Haskell {
    fn lower_type_name(
        &self,
        type_name: &crate::type_name::TypeName,
    ) -> Result<crate::code_block::CodeBlock, crate::error::SigilStitchError> {
        crate::lang::type_name_lowering::haskell(type_name)
    }
    fn file_extension(&self) -> &str {
        &self.extension
    }

    fn reserved_words(&self) -> &[&str] {
        HASKELL_RESERVED
    }

    fn escape_reserved(&self, name: &str) -> String {
        if self.reserved_words().contains(&name) {
            format!("{name}'")
        } else {
            name.to_string()
        }
    }

    fn render_string_literal(&self, s: &str) -> String {
        format!(
            "\"{}\"",
            s.replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', "\\n")
                .replace('\t', "\\t")
        )
    }

    fn line_comment_prefix(&self) -> &str {
        "--"
    }

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
    fn type_presentation(&self) -> crate::lang::config::TypePresentationConfig<'_> {
        crate::lang::config::TypePresentationConfig {
            array: crate::type_name::TypePresentation::Delimited {
                open: "[",
                sep: "",
                close: "]",
            },
            readonly_array: Some(crate::type_name::TypePresentation::Delimited {
                open: "[",
                sep: "",
                close: "]",
            }),
            optional: crate::type_name::TypePresentation::GenericWrap { name: "Maybe" },
            function: crate::type_name::FunctionPresentation {
                params_open: "",
                params_sep: " -> ",
                params_close: "",
                arrow: " -> ",
                curried: true,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
    fn generic_syntax(&self) -> crate::lang::config::GenericSyntaxConfig<'_> {
        crate::lang::config::GenericSyntaxConfig {
            open: "",
            close: "",
            application_style: crate::type_name::GenericApplicationStyle::PrefixJuxtaposition,
            constraint_keyword: "",
            constraint_separator: "",
            context_bound_keyword: "",
        }
    }

    fn module_separator(&self) -> Option<&str> {
        Some(".")
    }

    fn qualify_import_reference(&self, module: &str, name: &str, resolved_name: &str) -> String {
        if resolved_name == name {
            resolved_name.to_string()
        } else {
            format!("{module}.{name}")
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
            haskell_block_open_from_condition_text(condition)
        } else {
            haskell_block_open_for_intent(intent)
        };
        Ok(open.unwrap_or(" ="))
    }

    fn render_block_close(
        &self,
        _intent: BlockIntent,
        _condition: &str,
    ) -> Result<&str, SigilStitchError> {
        Ok("")
    }

    fn render_branch_transition(
        &self,
        _intent: BlockIntent,
        _condition: &str,
    ) -> Result<String, SigilStitchError> {
        Ok(String::new())
    }

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
    fn block_syntax(&self) -> crate::lang::config::BlockSyntaxConfig<'_> {
        crate::lang::config::BlockSyntaxConfig {
            block_open: " =",
            block_close: "",
            indent_unit: &self.indent,
            uses_semicolons: false,
            field_terminator: ",",
            ..Default::default()
        }
    }

    fn block_open_for(&self, condition: &str) -> Option<&str> {
        haskell_block_open_from_condition_text(condition)
    }

    fn block_open_for_intent(&self, intent: BlockIntent, _condition: &str) -> Option<&str> {
        haskell_block_open_for_intent(intent)
    }

    fn rewrite_nodes(&self, nodes: &mut Vec<CodeNode>) {
        crate::lang::rewrite::walk_nodes_mut(nodes, &Self::rewrite_dollar_spacing);
    }
}

const HASKELL_DATA_CAPABILITIES: &[TypeCapability] = &[
    // RecordFields = record fields
    TypeCapability::RecordFields,
    // ParametricPolymorphism = type variables
    TypeCapability::ParametricPolymorphism,
    // BoundedPolymorphism = class contexts / constraints
    TypeCapability::BoundedPolymorphism,
    // Variants = data constructors
    TypeCapability::Variants,
    // impl_types render as `deriving (...)`.
    TypeCapability::InterfaceImplementation,
];
const HASKELL_CONTRACT_CAPABILITIES: &[TypeCapability] = &[
    // Methods = class methods
    TypeCapability::Methods,
    // ParametricPolymorphism = type variables
    TypeCapability::ParametricPolymorphism,
    // BoundedPolymorphism = class contexts / constraints
    TypeCapability::BoundedPolymorphism,
];
const HASKELL_TYPES: &[TypeCapabilityProfile] = &[
    TypeCapabilityProfile::new(TypeKind::Struct, HASKELL_DATA_CAPABILITIES),
    // Class is represented as a Haskell data declaration.
    TypeCapabilityProfile::new(TypeKind::Class, HASKELL_DATA_CAPABILITIES),
    TypeCapabilityProfile::new(TypeKind::Trait, HASKELL_CONTRACT_CAPABILITIES),
    // Interface is represented as a Haskell class.
    TypeCapabilityProfile::new(TypeKind::Interface, HASKELL_CONTRACT_CAPABILITIES),
    TypeCapabilityProfile::new(
        TypeKind::Enum,
        &[
            // ParametricPolymorphism = type variables
            TypeCapability::ParametricPolymorphism,
            // BoundedPolymorphism = class contexts / constraints
            TypeCapability::BoundedPolymorphism,
            // Variants = data constructors
            TypeCapability::Variants,
            TypeCapability::ClosedSum,
            TypeCapability::InterfaceImplementation,
        ],
    ),
    TypeCapabilityProfile::new(
        TypeKind::TypeAlias,
        &[
            // ParametricPolymorphism = type variables
            TypeCapability::ParametricPolymorphism,
        ],
    ),
    TypeCapabilityProfile::new(
        TypeKind::Newtype,
        &[
            // ParametricPolymorphism = type variables
            TypeCapability::ParametricPolymorphism,
            // BoundedPolymorphism = class contexts / constraints
            TypeCapability::BoundedPolymorphism,
            TypeCapability::InterfaceImplementation,
        ],
    ),
];

const HASKELL_VARIANTS: &[VariantCapabilityProfile] = &[VariantCapabilityProfile::new(
    TypeKind::Enum,
    &[
        VariantCapability::PositionalPayload,
        VariantCapability::RecordPayload,
    ],
)];

const HASKELL_FUNCTION_CAPABILITIES: &[FunctionCapability] = &[
    // BoundedPolymorphism = class constraints
    FunctionCapability::BoundedPolymorphism,
    // ExplicitReturnType = result in the type signature
    FunctionCapability::ExplicitReturnType,
    FunctionCapability::TypedParameters,
    // ParametricPolymorphism = type variables
    FunctionCapability::ParametricPolymorphism,
];
const HASKELL_FUNCTIONS: &[FunctionCapabilityProfile] = &[
    FunctionCapabilityProfile::new(
        FunctionContext::TopLevel,
        FunctionForm::Function,
        HASKELL_FUNCTION_CAPABILITIES,
    )
    .with_body_policy(FunctionBodyPolicy::Required),
    FunctionCapabilityProfile::new(
        FunctionContext::Member,
        FunctionForm::Function,
        HASKELL_FUNCTION_CAPABILITIES,
    )
    .with_body_policy(FunctionBodyPolicy::Required),
    FunctionCapabilityProfile::new(
        FunctionContext::InterfaceMember,
        FunctionForm::Function,
        HASKELL_FUNCTION_CAPABILITIES,
    )
    .with_required_capabilities(&[
        FunctionCapability::ExplicitReturnType,
        FunctionCapability::TypedParameters,
    ]),
];

impl CodeLang for Haskell {
    fn validate_resolved_imports(
        &self,
        imports: &crate::import::ImportGroup,
    ) -> Result<(), crate::error::SigilStitchError> {
        crate::lang::import_validation::validate_identifier_aliases(
            self,
            imports,
            is_valid_import_alias,
        )?;
        if imports.entries().iter().any(|entry| entry.is_side_effect) {
            return Err(crate::error::SigilStitchError::InvalidResolvedImports {
                language: self.file_extension().to_string(),
                reason: "Haskell has no side-effect import form".to_string(),
            });
        }
        Ok(())
    }
    fn capabilities(&self) -> LanguageCapabilities<'_> {
        LanguageCapabilities::strict()
            .with_types(HASKELL_TYPES)
            .with_functions(HASKELL_FUNCTIONS)
            .with_variants(HASKELL_VARIANTS)
            .with_fields(crate::lang::field_lowering::haskell::PROFILES)
    }

    fn validate_type(&self, type_: crate::lang::TypeIntent<'_>) -> Result<(), SigilStitchError> {
        crate::lang::type_lowering::haskell::validate(self, type_)
    }

    fn lower_type(
        &self,
        type_: crate::lang::ValidatedType<'_>,
    ) -> Result<Vec<CodeBlock>, SigilStitchError> {
        crate::lang::type_lowering::haskell::lower(self, type_)
    }

    fn lower_function(
        &self,
        function: crate::spec::fun_spec::ValidatedFunction<'_>,
    ) -> Result<CodeBlock, SigilStitchError> {
        crate::lang::haskell_function_lowering::lower(self, function)
    }

    fn validate_fields(
        &self,
        fields: crate::lang::FieldSequenceIntent<'_>,
    ) -> Result<(), SigilStitchError> {
        crate::lang::field_lowering::haskell::validate(self, fields)
    }

    fn collect_field_validation_errors(
        &self,
        fields: crate::lang::FieldSequenceIntent<'_>,
        errors: &mut Vec<SigilStitchError>,
    ) {
        crate::lang::field_lowering::haskell::collect_validation_errors(self, fields, errors);
    }

    fn lower_fields(
        &self,
        fields: crate::lang::ValidatedFields<'_>,
    ) -> Result<CodeBlock, SigilStitchError> {
        crate::lang::field_lowering::haskell::lower(self, fields)
    }

    fn validate_variants(
        &self,
        variants: crate::lang::VariantIntent<'_>,
    ) -> Result<(), SigilStitchError> {
        crate::lang::variant_lowering::haskell::validate(self, variants)
    }

    fn collect_variant_validation_errors(
        &self,
        variants: crate::lang::VariantIntent<'_>,
        errors: &mut Vec<SigilStitchError>,
    ) {
        crate::lang::variant_lowering::haskell::collect_validation_errors(self, variants, errors);
    }

    fn lower_variants(
        &self,
        variants: crate::lang::ValidatedVariants<'_>,
    ) -> Result<CodeBlock, SigilStitchError> {
        crate::lang::variant_lowering::haskell::lower(self, variants)
    }

    fn validate_function_type_constraints(
        &self,
        function_name: &str,
        type_params: &[crate::spec::where_spec::TypeParamSpec],
        constraints: &[crate::spec::where_spec::WhereConstraint],
    ) -> Result<(), SigilStitchError> {
        if let Some(parameter) = type_params.iter().find(|parameter| {
            parameter.is_lifetime()
                || !crate::lang::type_lowering::haskell::starts_lowercase_identifier(
                    parameter.name(),
                )
                || self.reserved_words().contains(&parameter.name())
        }) {
            return Err(SigilStitchError::InvalidFunctionTypeParameter {
                language: self.file_extension().to_string(),
                function_name: function_name.to_string(),
                parameter_name: parameter.name().to_string(),
                reason: "Haskell type variables require a lowercase non-keyword identifier"
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

    fn validate_function(
        &self,
        function: crate::lang::FunctionIntent<'_>,
    ) -> Result<(), SigilStitchError> {
        if let Some(parameter) = function.type_params().iter().find(|parameter| {
            !function
                .parameters()
                .iter()
                .map(|parameter| parameter.param_type())
                .chain(function.return_type())
                .any(|type_name| type_name_contains_parameter(type_name, parameter.name()))
        }) {
            return Err(SigilStitchError::InvalidFunctionTypeParameter {
                language: self.file_extension().to_string(),
                function_name: function.name().to_string(),
                parameter_name: parameter.name().to_string(),
                reason: "Haskell function type variables must occur in a parameter or result type"
                    .to_string(),
            });
        }
        Ok(())
    }

    fn requires_complete_function_type_information(
        &self,
        _context: FunctionContext,
        _form: FunctionForm,
    ) -> bool {
        true
    }

    fn render_imports(&self, imports: &ImportGroup) -> String {
        if imports.entries().is_empty() {
            return String::new();
        }

        // Group names by module.
        let mut by_module: std::collections::BTreeMap<&str, Vec<&ImportEntry>> =
            std::collections::BTreeMap::new();
        for entry in imports.entries() {
            if entry.is_side_effect {
                continue;
            }
            by_module.entry(&entry.module).or_default().push(entry);
        }

        let mut base_imports: Vec<String> = Vec::new();
        let mut std_imports: Vec<String> = Vec::new();
        let mut other_imports: Vec<String> = Vec::new();

        for (module, entries) in &by_module {
            let has_wildcard = entries.iter().any(|e| e.is_wildcard);
            let lines = if has_wildcard {
                vec![format!("import {module}")]
            } else {
                let mut unqualified: Vec<&str> = entries
                    .iter()
                    .filter(|e| e.alias.is_none())
                    .map(|e| e.name.as_str())
                    .collect();
                unqualified.sort();
                unqualified.dedup();

                let mut qualified: Vec<&str> = entries
                    .iter()
                    .filter(|e| e.alias.is_some())
                    .map(|e| e.name.as_str())
                    .collect();
                qualified.sort();
                qualified.dedup();

                let mut lines = Vec::new();
                if !unqualified.is_empty() {
                    lines.push(format!("import {module} ({})", unqualified.join(", ")));
                }
                if !qualified.is_empty() {
                    lines.push(format!(
                        "import qualified {module} ({})",
                        qualified.join(", ")
                    ));
                }
                lines
            };

            for line in lines {
                match import_group_order(module) {
                    0 => base_imports.push(line),
                    1 => std_imports.push(line),
                    _ => other_imports.push(line),
                }
            }
        }

        let groups: Vec<&Vec<String>> = [&base_imports, &std_imports, &other_imports]
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

    fn render_doc_comment(&self, lines: &[&str]) -> String {
        let mut result = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            if i == 0 {
                if line.is_empty() {
                    result.push("-- |".to_string());
                } else {
                    result.push(format!("-- | {line}"));
                }
            } else if line.is_empty() {
                result.push("--".to_string());
            } else {
                result.push(format!("--   {line}"));
            }
        }
        result.join("\n")
    }

    fn render_visibility(&self, _vis: Visibility, _ctx: DeclarationContext) -> &str {
        ""
    }

    fn function_keyword(&self, _ctx: DeclarationContext) -> &str {
        ""
    }

    fn type_keyword(&self, kind: TypeKind) -> &str {
        match kind {
            TypeKind::Struct | TypeKind::Class => "data",
            TypeKind::Trait | TypeKind::Interface => "class",
            TypeKind::Enum => "data",
            TypeKind::TypeAlias => "type",
            TypeKind::Newtype => "newtype",
        }
    }

    fn methods_inside_type_body(&self, kind: TypeKind) -> bool {
        matches!(kind, TypeKind::Trait | TypeKind::Interface)
    }

    fn type_header_block_open(&self, kind: crate::spec::modifiers::TypeKind) -> &str {
        match kind {
            TypeKind::Trait | TypeKind::Interface => " where",
            _ => " =",
        }
    }

    fn fun_block_open(&self) -> &str {
        " ="
    }

    fn render_newtype_line(&self, _visibility: &str, name: &str, inner: &str) -> String {
        format!("newtype {name} = {name} {inner}")
    }

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
    fn render_type_context(&self, type_params: &[TypeParamSpec]) -> String {
        let resolve = |_module: &str, name: &str| name.to_string();
        let mut constraints = Vec::new();
        for type_param in type_params {
            for bound in &type_param.bounds {
                let bound = bound.render(80, &resolve).unwrap_or_default();
                constraints.push(format!("{bound} {}", type_param.name));
            }
        }
        if constraints.is_empty() {
            return String::new();
        }
        if constraints.len() == 1 {
            format!("{} => ", constraints[0])
        } else {
            format!("({}) => ", constraints.join(", "))
        }
    }

    fn render_type_close_suffix(&self, _kind: TypeKind, impl_types: &[String]) -> String {
        if impl_types.is_empty() {
            return String::new();
        }
        format!("  deriving ({})", impl_types.join(", "))
    }

    fn emit_newtype_decl(
        &self,
        _visibility: &str,
        name: &str,
        type_params: &[TypeParamSpec],
        inner: &TypeName,
    ) -> Result<CodeBlock, SigilStitchError> {
        let mut cb = CodeBlock::builder();
        cb.add("newtype ", ());
        if let Some(context) = self.emit_type_context(type_params)? {
            cb.add_code(context);
        }
        cb.add(name, ());
        for type_param in type_params {
            cb.add(&format!(" {}", type_param.name), ());
        }
        if crate::type_name_render::is_compound_type(inner) {
            cb.add(&format!(" = {name} (%T)"), inner.clone());
        } else {
            cb.add(&format!(" = {name} %T"), inner.clone());
        }
        cb.build()
    }

    fn emit_type_context(
        &self,
        type_params: &[TypeParamSpec],
    ) -> Result<Option<CodeBlock>, SigilStitchError> {
        let mut constraints = Vec::new();
        for tp in type_params {
            for bound in &tp.bounds {
                constraints.push((bound.clone(), tp.name.as_str()));
            }
        }
        if constraints.is_empty() {
            return Ok(None);
        }

        let mut format = String::new();
        let mut args = Vec::with_capacity(constraints.len());
        if constraints.len() > 1 {
            format.push('(');
        }
        for (index, (bound, param_name)) in constraints.into_iter().enumerate() {
            if index > 0 {
                format.push_str(", ");
            }
            format.push_str("%T ");
            format.push_str(param_name);
            args.push(Arg::TypeName(bound));
        }
        if args.len() > 1 {
            format.push(')');
        }
        format.push_str(" => ");

        CodeBlock::of(&format, args).map(Some)
    }

    fn type_body_prefix(&self, name: &str, kind: crate::spec::modifiers::TypeKind) -> String {
        match kind {
            TypeKind::Struct | TypeKind::Class => format!("{name} {{"),
            _ => String::new(),
        }
    }

    fn type_body_suffix(&self, _name: &str, kind: crate::spec::modifiers::TypeKind) -> String {
        match kind {
            TypeKind::Struct | TypeKind::Class => "}".to_string(),
            _ => String::new(),
        }
    }

    fn emit_type_close_suffix(
        &self,
        _kind: crate::spec::modifiers::TypeKind,
        impl_types: &[TypeName],
    ) -> Result<Option<CodeBlock>, SigilStitchError> {
        if impl_types.is_empty() {
            return Ok(None);
        }

        let mut format = String::from("deriving (");
        let mut args = Vec::with_capacity(impl_types.len());
        for (index, impl_type) in impl_types.iter().enumerate() {
            if index > 0 {
                format.push_str(", ");
            }
            format.push_str("%T");
            args.push(Arg::TypeName(impl_type.clone()));
        }
        format.push(')');
        CodeBlock::of(&format, args).map(Some)
    }

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
    fn function_syntax(&self) -> crate::lang::config::FunctionSyntaxConfig<'_> {
        crate::lang::config::FunctionSyntaxConfig {
            return_type_separator: " -> ",
            function_signature_style: crate::spec::fun_spec::FunctionSignatureStyle::Split,
            ..Default::default()
        }
    }

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
    fn type_decl_syntax(&self) -> crate::lang::config::TypeDeclSyntaxConfig<'_> {
        crate::lang::config::TypeDeclSyntaxConfig {
            type_annotation_separator: " :: ",
            ..Default::default()
        }
    }

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
    fn enum_and_annotation(&self) -> crate::lang::config::EnumAndAnnotationConfig<'_> {
        crate::lang::config::EnumAndAnnotationConfig {
            variant_prefix: "| ",
            variant_prefix_first: Some(""),
            variant_separator: "",
            ..Default::default()
        }
    }
}

fn type_name_contains_parameter(type_name: &TypeName, parameter_name: &str) -> bool {
    match type_name {
        TypeName::Primitive(name) | TypeName::Raw(name) => name == parameter_name,
        TypeName::Array(inner)
        | TypeName::ReadonlyArray(inner)
        | TypeName::Pointer(inner)
        | TypeName::Slice(inner)
        | TypeName::Optional(inner)
        | TypeName::Reference { inner, .. } => type_name_contains_parameter(inner, parameter_name),
        TypeName::Generic { base, params } => {
            type_name_contains_parameter(base, parameter_name)
                || params
                    .iter()
                    .any(|parameter| type_name_contains_parameter(parameter, parameter_name))
        }
        TypeName::Union(types)
        | TypeName::Intersection(types)
        | TypeName::Tuple(types)
        | TypeName::ImplTrait { bounds: types }
        | TypeName::DynTrait { bounds: types } => types
            .iter()
            .any(|type_name| type_name_contains_parameter(type_name, parameter_name)),
        TypeName::Map { key, value } => {
            type_name_contains_parameter(key, parameter_name)
                || type_name_contains_parameter(value, parameter_name)
        }
        TypeName::Function {
            params,
            return_type,
        } => {
            params
                .iter()
                .any(|parameter| type_name_contains_parameter(parameter, parameter_name))
                || type_name_contains_parameter(return_type, parameter_name)
        }
        TypeName::AssociatedType {
            base, qualifier, ..
        } => {
            type_name_contains_parameter(base, parameter_name)
                || qualifier.as_deref().is_some_and(|qualifier| {
                    type_name_contains_parameter(qualifier, parameter_name)
                })
        }
        TypeName::Wildcard {
            upper_bound,
            lower_bound,
        } => {
            upper_bound
                .as_deref()
                .is_some_and(|bound| type_name_contains_parameter(bound, parameter_name))
                || lower_bound
                    .as_deref()
                    .is_some_and(|bound| type_name_contains_parameter(bound, parameter_name))
        }
        TypeName::Importable { .. } | TypeName::StringLiteral(_) => false,
    }
}

#[cfg(test)]
#[expect(deprecated, reason = "0.6.8 compatibility assertions")]
mod tests {
    use super::*;

    #[test]
    fn type_parameter_occurrence_walks_complete_type_name_structure() {
        let parameter = || TypeName::primitive("a");
        let other = || TypeName::primitive("Value");
        let containing = [
            parameter(),
            TypeName::raw("a"),
            TypeName::array(parameter()),
            TypeName::readonly_array(parameter()),
            TypeName::pointer(parameter()),
            TypeName::slice(parameter()),
            TypeName::optional(parameter()),
            TypeName::reference(parameter()),
            TypeName::generic(other(), vec![parameter()]),
            TypeName::union(vec![other(), parameter()]),
            TypeName::intersection(vec![other(), parameter()]),
            TypeName::tuple(vec![other(), parameter()]),
            TypeName::impl_trait(vec![other(), parameter()]),
            TypeName::dyn_trait(vec![other(), parameter()]),
            TypeName::map(other(), parameter()),
            TypeName::function(vec![other()], parameter()),
            TypeName::associated_type(other(), Some(parameter()), "Member"),
            TypeName::wildcard_extends(parameter()),
            TypeName::wildcard_super(parameter()),
        ];
        for type_name in containing {
            assert!(
                type_name_contains_parameter(&type_name, "a"),
                "{type_name:?}"
            );
        }

        for type_name in [
            other(),
            TypeName::raw("Value"),
            TypeName::importable("Data.Text", "Text"),
            TypeName::string_literal("a"),
            TypeName::wildcard(),
        ] {
            assert!(
                !type_name_contains_parameter(&type_name, "a"),
                "{type_name:?}"
            );
        }
    }

    #[test]
    fn test_file_extension() {
        let hs = Haskell::new();
        assert_eq!(hs.file_extension(), "hs");
    }

    #[test]
    fn test_escape_reserved() {
        let hs = Haskell::new();
        assert_eq!(hs.escape_reserved("type"), "type'");
        assert_eq!(hs.escape_reserved("data"), "data'");
        assert_eq!(hs.escape_reserved("name"), "name");
    }

    #[test]
    fn test_render_imports_single() {
        let hs = Haskell::new();
        let imports = ImportGroup {
            entries: vec![ImportEntry {
                module: "Data.Map".into(),
                name: "Map".into(),
                alias: None,
                is_type_only: false,
                is_side_effect: false,
                is_wildcard: false,
            }],
        };
        assert_eq!(hs.render_imports(&imports), "import Data.Map (Map)");
    }

    #[test]
    fn test_render_imports_grouped() {
        let hs = Haskell::new();
        let imports = ImportGroup {
            entries: vec![
                ImportEntry {
                    module: "Data.Map".into(),
                    name: "Map".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "Data.Map".into(),
                    name: "fromList".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "MyApp.Types".into(),
                    name: "User".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
            ],
        };
        let output = hs.render_imports(&imports);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines[0], "import Data.Map (Map, fromList)");
        assert_eq!(lines[1], "");
        assert_eq!(lines[2], "import MyApp.Types (User)");
    }

    #[test]
    fn test_render_imports_uses_qualified_form_for_resolved_aliases() {
        let hs = Haskell::new();
        let imports = ImportGroup {
            entries: vec![
                ImportEntry {
                    module: "Domain.Input".into(),
                    name: "Value".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "Domain.Output".into(),
                    name: "Value".into(),
                    alias: Some("OutputValue".into()),
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
            ],
        };

        assert_eq!(
            hs.render_imports(&imports),
            "import Domain.Input (Value)\nimport qualified Domain.Output (Value)"
        );
    }

    #[test]
    fn test_render_imports_wildcard() {
        let hs = Haskell::new();
        let imports = ImportGroup {
            entries: vec![ImportEntry {
                module: "Data.List".into(),
                name: "".into(),
                alias: None,
                is_type_only: false,
                is_side_effect: false,
                is_wildcard: true,
            }],
        };
        assert_eq!(hs.render_imports(&imports), "import Data.List");
    }

    #[test]
    fn test_doc_comment_single() {
        let hs = Haskell::new();
        assert_eq!(
            hs.render_doc_comment(&["A brief description."]),
            "-- | A brief description."
        );
    }

    #[test]
    fn test_doc_comment_multi() {
        let hs = Haskell::new();
        let doc = hs.render_doc_comment(&["Get the user.", "", "Returns Nothing if not found."]);
        assert_eq!(
            doc,
            "-- | Get the user.\n--\n--   Returns Nothing if not found."
        );
    }

    #[test]
    fn test_string_literal() {
        let hs = Haskell::new();
        assert_eq!(hs.render_string_literal("hello"), "\"hello\"");
        assert_eq!(hs.render_string_literal("it\"s"), "\"it\\\"s\"");
        assert_eq!(hs.render_string_literal("new\nline"), "\"new\\nline\"");
    }

    #[test]
    fn test_type_keyword() {
        let hs = Haskell::new();
        assert_eq!(hs.type_keyword(TypeKind::Struct), "data");
        assert_eq!(hs.type_keyword(TypeKind::Class), "data");
        assert_eq!(hs.type_keyword(TypeKind::Trait), "class");
        assert_eq!(hs.type_keyword(TypeKind::Enum), "data");
        assert_eq!(hs.type_keyword(TypeKind::TypeAlias), "type");
        assert_eq!(hs.type_keyword(TypeKind::Newtype), "newtype");
    }

    #[test]
    fn test_visibility_always_empty() {
        let hs = Haskell::new();
        assert_eq!(
            hs.render_visibility(Visibility::Public, DeclarationContext::TopLevel),
            ""
        );
        assert_eq!(
            hs.render_visibility(Visibility::Private, DeclarationContext::TopLevel),
            ""
        );
    }

    #[test]
    fn test_no_semicolons() {
        let hs = Haskell::new();
        assert!(!hs.block_syntax().uses_semicolons);
    }

    #[test]
    fn test_generic_application_style() {
        let hs = Haskell::new();
        assert!(matches!(
            hs.generic_syntax().application_style,
            crate::type_name::GenericApplicationStyle::PrefixJuxtaposition
        ));
    }

    #[test]
    fn test_type_annotation_separator() {
        let hs = Haskell::new();
        assert_eq!(hs.type_decl_syntax().type_annotation_separator, " :: ");
    }

    #[test]
    fn test_haskell_builder_fluent() {
        let hs = Haskell::new().with_indent("    ").with_extension("lhs");
        assert_eq!(hs.file_extension(), "lhs");
        assert_eq!(hs.block_syntax().indent_unit, "    ");
    }

    #[test]
    fn test_import_group_order() {
        assert_eq!(import_group_order("Prelude"), 0);
        assert_eq!(import_group_order("GHC.Base"), 0);
        assert_eq!(import_group_order("Data.Map"), 1);
        assert_eq!(import_group_order("Control.Monad"), 1);
        assert_eq!(import_group_order("System.IO"), 1);
        assert_eq!(import_group_order("MyApp.Types"), 2);
    }

    #[test]
    fn test_emit_type_context_empty() {
        let hs = Haskell::new();
        let params: Vec<crate::spec::where_spec::TypeParamSpec> = vec![];
        assert!(hs.emit_type_context(&params).unwrap().is_none());
    }

    #[test]
    fn test_emit_type_context_single() {
        let hs = Haskell::new();
        let params = vec![
            crate::spec::where_spec::TypeParamSpec::new("a")
                .with_bound(crate::type_name::TypeName::primitive("Show")),
        ];
        let context = hs.emit_type_context(&params).unwrap().unwrap();
        assert_eq!(context.render_standalone(&hs, 80).unwrap(), "Show a => ");
    }

    #[test]
    fn test_emit_type_context_multiple() {
        let hs = Haskell::new();
        let params = vec![
            crate::spec::where_spec::TypeParamSpec::new("a")
                .with_bound(crate::type_name::TypeName::primitive("Show"))
                .with_bound(crate::type_name::TypeName::primitive("Eq")),
        ];
        let context = hs.emit_type_context(&params).unwrap().unwrap();
        assert_eq!(
            context.render_standalone(&hs, 80).unwrap(),
            "(Show a, Eq a) => "
        );
    }

    #[test]
    fn test_type_body_prefix_struct() {
        let hs = Haskell::new();
        assert_eq!(hs.type_body_prefix("Person", TypeKind::Struct), "Person {");
    }

    #[test]
    fn test_type_body_prefix_trait() {
        let hs = Haskell::new();
        assert_eq!(hs.type_body_prefix("Functor", TypeKind::Trait), "");
    }

    #[test]
    fn test_type_body_suffix_struct() {
        let hs = Haskell::new();
        assert_eq!(hs.type_body_suffix("Person", TypeKind::Struct), "}");
    }

    #[test]
    fn test_emit_type_close_suffix_empty() {
        let hs = Haskell::new();
        assert!(
            hs.emit_type_close_suffix(TypeKind::Enum, &[])
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn test_emit_type_close_suffix_deriving() {
        let hs = Haskell::new();
        let types = vec![TypeName::primitive("Show"), TypeName::primitive("Eq")];
        let suffix = hs
            .emit_type_close_suffix(TypeKind::Enum, &types)
            .unwrap()
            .unwrap();
        assert_eq!(
            suffix.render_standalone(&hs, 80).unwrap(),
            "deriving (Show, Eq)"
        );
    }

    #[test]
    fn test_emit_newtype_decl() {
        let hs = Haskell::new();
        let declaration = hs
            .emit_newtype_decl("", "Meters", &[], &TypeName::primitive("f64"))
            .unwrap();
        assert_eq!(
            declaration.render_standalone(&hs, 80).unwrap(),
            "newtype Meters = Meters f64"
        );
    }

    #[test]
    fn test_function_signature_style() {
        let hs = Haskell::new();
        assert_eq!(
            hs.function_syntax().function_signature_style,
            crate::spec::fun_spec::FunctionSignatureStyle::Split
        );
    }

    #[test]
    fn test_module_separator() {
        let hs = Haskell::new();
        assert_eq!(hs.module_separator(), Some("."));
    }

    #[test]
    fn test_alias_uses_qualified_original_name() {
        let hs = Haskell::new();
        assert_eq!(
            hs.qualify_import_reference("Domain.Json", "ToJSON", "JsonToJSON"),
            "Domain.Json.ToJSON"
        );
    }

    #[test]
    fn test_block_open_for_if_else_case() {
        let hs = Haskell::new();
        assert_eq!(
            hs.block_open_for_intent(BlockIntent::If, "if x > 0"),
            Some(" then")
        );
        assert_eq!(
            hs.block_open_for_intent(BlockIntent::ElseIf, "else if x < 0"),
            Some(" then")
        );
        assert_eq!(
            hs.block_open_for_intent(BlockIntent::Else, "else"),
            Some("")
        );
        assert_eq!(
            hs.block_open_for_intent(BlockIntent::Case, "case x"),
            Some(" of")
        );
        assert_eq!(
            hs.block_open_for_intent(BlockIntent::Class, "class Eq a"),
            Some(" where")
        );
        assert_eq!(hs.block_open_for_intent(BlockIntent::Do, "do"), Some(""));
        assert_eq!(
            hs.block_open_for_intent(BlockIntent::Generic, "let x = 5"),
            None
        );
    }

    #[test]
    fn dollar_spacing_rewrites_code_but_not_string_contents() {
        let mut nodes = vec![
            CodeNode::Literal("apply $value".to_string()),
            CodeNode::Literal("name = \"cost: $value\"".to_string()),
        ];
        Haskell::rewrite_dollar_spacing(&mut nodes);
        assert!(matches!(&nodes[0], CodeNode::Literal(s) if s == "apply $ value"));
        assert!(
            matches!(&nodes[1], CodeNode::Literal(s) if s == "name = \"cost: $value\""),
            "dollar signs inside string literals must not be rewritten"
        );
    }
}
