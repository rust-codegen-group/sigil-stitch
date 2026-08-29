//! Scala language implementation.

use crate::code_block::{Arg, CodeBlock};
use crate::error::SigilStitchError;
use crate::import::ImportGroup;
use crate::lang::capability::{
    FunctionBodyPolicy, FunctionCapability, FunctionCapabilityProfile, FunctionContext,
    FunctionForm, LanguageCapabilities, TypeCapability, TypeCapabilityProfile, VariantCapability,
    VariantCapabilityProfile,
};
use crate::lang::{CodeLang, RendererLang};
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};
use crate::spec::where_spec::{TypeParamKind, TypeParamSpec, render_type_params_for};
use crate::type_name::TypeName;

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
/// Scala uses `extends` for the optional superclass and `with` for implemented
/// traits. Preserve that distinction in the builder:
/// ```text
/// let tb = TypeSpec::builder("Foo", TypeKind::Class)
///     .extends(TypeName::primitive("Base"))
///     .implements(TypeName::primitive("Serializable"));
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
/// tb.add_primary_constructor_param(
///     ParameterSpec::builder("name", TypeName::primitive("String")).is_property().build()?
/// );
/// tb.add_primary_constructor_param(
///     ParameterSpec::builder("age", TypeName::primitive("Int")).is_property().build()?
/// );
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

fn is_valid_raw_type_parameter_kind(raw: &str) -> bool {
    let raw = raw.trim();
    if !raw.starts_with('[') || !raw.ends_with(']') || raw[1..raw.len() - 1].trim().is_empty() {
        return false;
    }

    let mut depth = 0usize;
    for character in raw.chars() {
        match character {
            '[' => depth += 1,
            ']' if depth == 0 => return false,
            ']' => depth -= 1,
            '\n' | '\r' => return false,
            _ if depth == 0 => return false,
            _ => {}
        }
    }
    depth == 0
}

pub(crate) fn invalid_raw_type_parameter(type_params: &[TypeParamSpec]) -> Option<&TypeParamSpec> {
    type_params.iter().find(|parameter| {
        matches!(parameter.kind(), Some(TypeParamKind::Raw(raw)) if !is_valid_raw_type_parameter_kind(raw))
    })
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

#[deny(deprecated)]
impl RendererLang for Scala {
    fn lower_type_name(
        &self,
        type_name: &crate::type_name::TypeName,
    ) -> Result<crate::code_block::CodeBlock, crate::error::SigilStitchError> {
        crate::lang::type_name_lowering::scala(type_name)
    }
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

    fn render_verbatim_string(&self, s: &str) -> String {
        let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
        format!("s\"{escaped}\"")
    }

    fn line_comment_prefix(&self) -> &str {
        "//"
    }

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
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

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
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
        Ok(" {")
    }

    fn render_block_close(
        &self,
        _intent: crate::code_node::BlockIntent,
        _condition: &str,
    ) -> Result<&str, crate::error::SigilStitchError> {
        Ok("}")
    }

    fn render_branch_transition(
        &self,
        _intent: crate::code_node::BlockIntent,
        _condition: &str,
    ) -> Result<String, crate::error::SigilStitchError> {
        Ok("} ".to_string())
    }

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
    fn block_syntax(&self) -> crate::lang::config::BlockSyntaxConfig<'_> {
        crate::lang::config::BlockSyntaxConfig {
            indent_unit: &self.indent,
            uses_semicolons: false,
            field_terminator: "",
            ..Default::default()
        }
    }
}

const SCALA_CLASS_CAPABILITIES: &[TypeCapability] = &[
    // RecordFields = fields
    TypeCapability::RecordFields,
    // AccessorMethods = accessors
    TypeCapability::AccessorMethods,
    // Methods = methods
    TypeCapability::Methods,
    // NominalSubtyping = `extends`
    TypeCapability::NominalSubtyping,
    // InterfaceImplementation = `with`
    TypeCapability::InterfaceImplementation,
    // ParametricPolymorphism = type parameters
    TypeCapability::ParametricPolymorphism,
    TypeCapability::HigherKindedPolymorphism,
    // BoundedPolymorphism = context/view bounds and type bounds
    TypeCapability::BoundedPolymorphism,
    // PrimaryConstructorParameters = primary constructor parameters
    TypeCapability::PrimaryConstructorParameters,
    // Attributes = annotations
    TypeCapability::Attributes,
];
const SCALA_CONTRACT_CAPABILITIES: &[TypeCapability] = &[
    // Methods = methods
    TypeCapability::Methods,
    // NominalSubtyping = `extends`
    TypeCapability::NominalSubtyping,
    // ParametricPolymorphism = type parameters
    TypeCapability::ParametricPolymorphism,
    TypeCapability::HigherKindedPolymorphism,
    // BoundedPolymorphism = context/view bounds and type bounds
    TypeCapability::BoundedPolymorphism,
    // Attributes = annotations
    TypeCapability::Attributes,
];
const SCALA_TYPES: &[TypeCapabilityProfile] = &[
    TypeCapabilityProfile::new(TypeKind::Class, SCALA_CLASS_CAPABILITIES),
    // Struct is represented as a Scala case class.
    TypeCapabilityProfile::new(TypeKind::Struct, SCALA_CLASS_CAPABILITIES),
    TypeCapabilityProfile::new(TypeKind::Trait, SCALA_CONTRACT_CAPABILITIES),
    // Interface is represented as a Scala trait.
    TypeCapabilityProfile::new(TypeKind::Interface, SCALA_CONTRACT_CAPABILITIES),
    TypeCapabilityProfile::new(
        TypeKind::Enum,
        &[
            // RecordFields = fields
            TypeCapability::RecordFields,
            // AccessorMethods = accessors
            TypeCapability::AccessorMethods,
            // Methods = methods
            TypeCapability::Methods,
            // ParametricPolymorphism = type parameters
            TypeCapability::ParametricPolymorphism,
            TypeCapability::HigherKindedPolymorphism,
            // Attributes = annotations
            TypeCapability::Attributes,
            // Variants = enum cases
            TypeCapability::Variants,
            TypeCapability::ClosedSum,
        ],
    ),
    TypeCapabilityProfile::new(
        TypeKind::TypeAlias,
        &[
            // ParametricPolymorphism = type parameters
            TypeCapability::ParametricPolymorphism,
            // BoundedPolymorphism = inline upper bounds
            TypeCapability::BoundedPolymorphism,
            TypeCapability::HigherKindedPolymorphism,
        ],
    ),
    TypeCapabilityProfile::new(
        TypeKind::Newtype,
        &[
            // ParametricPolymorphism = type parameters
            TypeCapability::ParametricPolymorphism,
            // BoundedPolymorphism = inline upper bounds
            TypeCapability::BoundedPolymorphism,
            TypeCapability::HigherKindedPolymorphism,
            // Attributes = annotations
            TypeCapability::Attributes,
        ],
    ),
];

const SCALA_VARIANTS: &[VariantCapabilityProfile] = &[VariantCapabilityProfile::new(
    TypeKind::Enum,
    &[VariantCapability::Attributes],
)];

const SCALA_TOP_LEVEL_FUNCTION_CAPABILITIES: &[FunctionCapability] = &[
    // Attributes = annotations
    FunctionCapability::Attributes,
    // BoundedPolymorphism = context/upper bounds
    FunctionCapability::BoundedPolymorphism,
    // DefaultParameters = default parameters
    FunctionCapability::DefaultParameters,
    // ExplicitReturnType = result annotation
    FunctionCapability::ExplicitReturnType,
    FunctionCapability::TypedParameters,
    // ParametricPolymorphism = type parameters
    FunctionCapability::ParametricPolymorphism,
    FunctionCapability::HigherKindedPolymorphism,
];
const SCALA_MEMBER_FUNCTION_CAPABILITIES: &[FunctionCapability] = &[
    // AbstractMethod = abstract members
    FunctionCapability::AbstractMethod,
    // Attributes = annotations
    FunctionCapability::Attributes,
    // BoundedPolymorphism = context/upper bounds
    FunctionCapability::BoundedPolymorphism,
    // DefaultParameters = default parameters
    FunctionCapability::DefaultParameters,
    // ExplicitReturnType = result annotation
    FunctionCapability::ExplicitReturnType,
    FunctionCapability::TypedParameters,
    // Override = override
    FunctionCapability::Override,
    // ParametricPolymorphism = type parameters
    FunctionCapability::ParametricPolymorphism,
    FunctionCapability::HigherKindedPolymorphism,
];
const SCALA_FUNCTIONS: &[FunctionCapabilityProfile] = &[
    FunctionCapabilityProfile::new(
        FunctionContext::TopLevel,
        FunctionForm::Function,
        SCALA_TOP_LEVEL_FUNCTION_CAPABILITIES,
    )
    .with_required_capabilities(&[FunctionCapability::TypedParameters])
    .with_body_policy(FunctionBodyPolicy::Required),
    FunctionCapabilityProfile::new(
        FunctionContext::Member,
        FunctionForm::Function,
        SCALA_MEMBER_FUNCTION_CAPABILITIES,
    )
    .with_required_capabilities(&[FunctionCapability::TypedParameters])
    .with_body_policy(FunctionBodyPolicy::Required),
    FunctionCapabilityProfile::new(
        FunctionContext::InterfaceMember,
        FunctionForm::Function,
        SCALA_MEMBER_FUNCTION_CAPABILITIES,
    )
    .with_required_capabilities(&[FunctionCapability::TypedParameters]),
];

impl CodeLang for Scala {
    fn validate_resolved_imports(
        &self,
        imports: &crate::import::ImportGroup,
    ) -> Result<(), crate::error::SigilStitchError> {
        crate::lang::import_validation::reject_aliases(self, imports)?;
        if imports.entries().iter().any(|entry| entry.is_side_effect) {
            return Err(crate::error::SigilStitchError::InvalidResolvedImports {
                language: self.file_extension().to_string(),
                reason: "Scala has no side-effect import form".to_string(),
            });
        }
        Ok(())
    }
    fn capabilities(&self) -> LanguageCapabilities<'_> {
        LanguageCapabilities::strict()
            .with_types(SCALA_TYPES)
            .with_functions(SCALA_FUNCTIONS)
            .with_variants(SCALA_VARIANTS)
            .with_fields(crate::lang::field_lowering::scala::PROFILES)
            .with_properties(crate::lang::property_lowering::scala::PROFILES)
    }

    fn validate_type(&self, type_: crate::lang::TypeIntent<'_>) -> Result<(), SigilStitchError> {
        crate::lang::type_lowering::scala::validate(self, type_)
    }

    fn lower_type(
        &self,
        type_: crate::lang::ValidatedType<'_>,
    ) -> Result<Vec<CodeBlock>, SigilStitchError> {
        crate::lang::type_lowering::scala::lower(self, type_)
    }

    fn lower_function(
        &self,
        function: crate::spec::fun_spec::ValidatedFunction<'_>,
    ) -> Result<CodeBlock, SigilStitchError> {
        crate::lang::scala_function_lowering::lower(self, function)
    }

    fn validate_fields(
        &self,
        fields: crate::lang::FieldSequenceIntent<'_>,
    ) -> Result<(), SigilStitchError> {
        crate::lang::field_lowering::scala::validate(self, fields)
    }

    fn collect_field_validation_errors(
        &self,
        fields: crate::lang::FieldSequenceIntent<'_>,
        errors: &mut Vec<SigilStitchError>,
    ) {
        crate::lang::field_lowering::scala::collect_validation_errors(self, fields, errors);
    }

    fn lower_fields(
        &self,
        fields: crate::lang::ValidatedFields<'_>,
    ) -> Result<CodeBlock, SigilStitchError> {
        crate::lang::field_lowering::scala::lower(self, fields)
    }

    fn validate_property(
        &self,
        property: crate::lang::PropertyIntent<'_>,
    ) -> Result<(), SigilStitchError> {
        crate::lang::property_lowering::scala::validate(self, property)
    }

    fn collect_property_validation_errors(
        &self,
        property: crate::lang::PropertyIntent<'_>,
        errors: &mut Vec<SigilStitchError>,
    ) {
        crate::lang::property_lowering::scala::collect_validation_errors(self, property, errors);
    }

    fn lower_property(
        &self,
        property: crate::lang::ValidatedProperty<'_>,
    ) -> Result<Vec<CodeBlock>, SigilStitchError> {
        crate::lang::property_lowering::scala::lower(self, property)
    }

    fn validate_type_members(
        &self,
        members: crate::lang::TypeMembersIntent<'_>,
    ) -> Result<(), SigilStitchError> {
        crate::lang::type_members_validation::scala::validate(self, members)
    }

    fn collect_type_members_validation_errors(
        &self,
        members: crate::lang::TypeMembersIntent<'_>,
        errors: &mut Vec<SigilStitchError>,
    ) {
        crate::lang::type_members_validation::scala::collect_validation_errors(
            self, members, errors,
        );
    }

    fn validate_variants(
        &self,
        variants: crate::lang::VariantIntent<'_>,
    ) -> Result<(), SigilStitchError> {
        crate::lang::variant_lowering::scala::validate(self, variants)
    }

    fn collect_variant_validation_errors(
        &self,
        variants: crate::lang::VariantIntent<'_>,
        errors: &mut Vec<SigilStitchError>,
    ) {
        crate::lang::variant_lowering::scala::collect_validation_errors(self, variants, errors);
    }

    fn lower_variants(
        &self,
        variants: crate::lang::ValidatedVariants<'_>,
    ) -> Result<CodeBlock, SigilStitchError> {
        crate::lang::variant_lowering::scala::lower(self, variants)
    }

    fn validate_function_type_constraints(
        &self,
        function_name: &str,
        type_params: &[crate::spec::where_spec::TypeParamSpec],
        constraints: &[crate::spec::where_spec::WhereConstraint],
    ) -> Result<(), SigilStitchError> {
        if let Some(parameter) = type_params.iter().find(|parameter| {
            parameter.is_lifetime()
                || !crate::lang::type_lowering::scala::is_identifier(parameter.name())
                || self.reserved_words().contains(&parameter.name())
        }) {
            return Err(SigilStitchError::InvalidFunctionTypeParameter {
                language: self.file_extension().to_string(),
                function_name: function_name.to_string(),
                parameter_name: parameter.name().to_string(),
                reason: "Scala type parameters require an ordinary non-keyword identifier"
                    .to_string(),
            });
        }
        if let Some(parameter) = invalid_raw_type_parameter(type_params) {
            return Err(SigilStitchError::InvalidFunctionTypeParameter {
                language: self.file_extension().to_string(),
                function_name: function_name.to_string(),
                parameter_name: parameter.name().to_string(),
                reason: "Scala higher-kinded parameter syntax must be a non-empty balanced bracket suffix"
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

    fn function_visibility_is_valid(
        &self,
        context: FunctionContext,
        _form: FunctionForm,
        _is_static: bool,
        visibility: Visibility,
    ) -> bool {
        match context {
            FunctionContext::TopLevel => matches!(
                visibility,
                Visibility::Inherited | Visibility::Public | Visibility::Private
            ),
            FunctionContext::Member | FunctionContext::InterfaceMember => matches!(
                visibility,
                Visibility::Inherited
                    | Visibility::Public
                    | Visibility::Private
                    | Visibility::Protected
            ),
            FunctionContext::ReceiverMethod => false,
        }
    }

    fn abstract_type_modifier_is_valid(&self, kind: TypeKind) -> bool {
        kind == TypeKind::Class
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

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
    fn optional_field_style(&self) -> crate::lang::config::OptionalFieldStyle {
        crate::lang::config::OptionalFieldStyle::TypeWrap {
            open: "Option[",
            close: "]",
        }
    }

    fn render_type_param_kind(&self, kind: &crate::spec::where_spec::TypeParamKind) -> String {
        match kind {
            crate::spec::where_spec::TypeParamKind::Constructor1 => "[_]".to_string(),
            crate::spec::where_spec::TypeParamKind::Constructor2 => "[_, _]".to_string(),
            crate::spec::where_spec::TypeParamKind::Raw(s) => s.clone(),
        }
    }

    fn render_newtype_line(&self, visibility: &str, name: &str, inner: &str) -> String {
        format!("{visibility}class {name}(val value: {inner})")
    }

    fn emit_newtype_decl(
        &self,
        visibility: &str,
        name: &str,
        type_params: &[TypeParamSpec],
        inner: &TypeName,
    ) -> Result<CodeBlock, SigilStitchError> {
        let mut args = Vec::new();
        let type_params = render_type_params_for(type_params, self, &mut args);
        args.push(Arg::TypeName(inner.clone()));
        CodeBlock::of(
            &format!("{visibility}class {name}{type_params}(val value: %T)"),
            args,
        )
    }

    fn fun_block_open(&self) -> &str {
        " = {"
    }

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
    fn function_syntax(&self) -> crate::lang::config::FunctionSyntaxConfig<'_> {
        crate::lang::config::FunctionSyntaxConfig {
            abstract_keyword: "",
            ..Default::default()
        }
    }

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
    fn type_decl_syntax(&self) -> crate::lang::config::TypeDeclSyntaxConfig<'_> {
        crate::lang::config::TypeDeclSyntaxConfig {
            super_type_keyword: " extends ",
            super_type_subsequent_separator: Some(" with "),
            implements_keyword: " with ",
            supports_primary_constructor: true,
            ..Default::default()
        }
    }

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
    fn enum_and_annotation(&self) -> crate::lang::config::EnumAndAnnotationConfig<'_> {
        crate::lang::config::EnumAndAnnotationConfig {
            readonly_keyword: "val ",
            mutable_field_keyword: "var ",
            ..Default::default()
        }
    }
}

#[cfg(test)]
#[expect(deprecated, reason = "0.6.8 compatibility assertions")]
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
        use crate::spec::where_spec::TypeParamKind;
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
    fn test_emit_newtype_decl() {
        let sc = Scala::new();
        let declaration = sc
            .emit_newtype_decl("", "Meters", &[], &TypeName::primitive("Double"))
            .unwrap();
        assert_eq!(
            declaration.render_standalone(&sc, 80).unwrap(),
            "class Meters(val value: Double)"
        );
    }

    #[test]
    fn test_module_separator() {
        let sc = Scala::new();
        assert_eq!(sc.module_separator(), Some("."));
    }
}
