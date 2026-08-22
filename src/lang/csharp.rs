use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::import::ImportGroup;
use crate::lang::capability::{
    FunctionBodyPolicy, FunctionCapability, FunctionCapabilityProfile, FunctionContext,
    FunctionForm, LanguageCapabilities, TypeCapability, TypeCapabilityProfile, VariantCapability,
    VariantCapabilityProfile,
};
use crate::lang::{CodeLang, RendererLang};
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};

/// C# language implementation.
///
/// C#-specific behaviors:
/// - Type-before-name declarations (`int count`, not `count: int`)
/// - Return type as prefix (`int Add(int a, int b)`)
/// - `using Namespace;` with System/Microsoft/third-party grouping
/// - Semicolons after statements
/// - No function keyword
/// - `class`, `struct`, `interface`, `enum`, `record` keywords
/// - Single `:` for both inheritance and interfaces
/// - Generic bounds via `where T : Constraint` clause
/// - `/// ...` XML doc comments
/// - `readonly` for immutable fields, `async` for coroutines
/// - `?` nullable suffix for optional types
/// - `@` prefix escaping for reserved words
///
/// # Import conventions
///
/// Use [`crate::type_name::TypeName::importable`] with the namespace as module and type name as name:
/// ```text
/// TypeName::importable("System.Collections.Generic", "List")   // using System.Collections.Generic;
/// TypeName::importable("System.Threading.Tasks", "Task")       // using System.Threading.Tasks;
/// TypeName::importable("MyApp.Models", "User")                 // using MyApp.Models;
/// ```
///
/// # Inheritance
///
/// C# uses `:` for both base class and interfaces. Put all supertypes
/// into `super_types()`:
/// ```text
/// tb.super_type(TypeName::primitive("BaseClass"));
/// tb.super_type(TypeName::primitive("ISerializable"));
/// // Emits: class Foo : BaseClass, ISerializable {
/// ```
///
/// # Primary constructors (C# 12+)
///
/// Use `add_primary_constructor_param()` on `TypeSpecBuilder`:
/// ```text
/// let mut tb = TypeSpec::builder("Person", TypeKind::Class);
/// tb.add_primary_constructor_param(ParameterSpec::new("string name", TypeName::empty()));
/// // Emits: class Person(string name) {
/// ```
#[derive(Debug, Clone)]
pub struct CSharp {
    /// Indent with this string (default: "    " — 4 spaces).
    pub indent: String,
    /// File extension (default: "cs").
    pub extension: String,
}

impl Default for CSharp {
    fn default() -> Self {
        Self {
            indent: "    ".to_string(),
            extension: "cs".to_string(),
        }
    }
}

impl CSharp {
    /// Create a new C# language instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the indent string (e.g., `"    "` for 4-space default, `"\t"` for tabs).
    pub fn with_indent(mut self, s: &str) -> Self {
        self.indent = s.to_string();
        self
    }

    /// Set the file extension (default: `"cs"`).
    pub fn with_extension(mut self, s: &str) -> Self {
        self.extension = s.to_string();
        self
    }
}

#[rustfmt::skip]
const CSHARP_RESERVED: &[&str] = &[
    // Keywords
    "abstract", "as", "base", "bool", "break", "byte", "case", "catch",
    "char", "checked", "class", "const", "continue", "decimal", "default",
    "delegate", "do", "double", "else", "enum", "event", "explicit",
    "extern", "false", "finally", "fixed", "float", "for", "foreach",
    "goto", "if", "implicit", "in", "int", "interface", "internal", "is",
    "lock", "long", "namespace", "new", "null", "object", "operator",
    "out", "override", "params", "private", "protected", "public",
    "readonly", "ref", "return", "sbyte", "sealed", "short", "sizeof",
    "stackalloc", "static", "string", "struct", "switch", "this", "throw",
    "true", "try", "typeof", "uint", "ulong", "unchecked", "unsafe",
    "ushort", "using", "virtual", "void", "volatile", "while",
    // Contextual keywords
    "add", "and", "alias", "ascending", "args", "async", "await", "by",
    "descending", "dynamic", "equals", "file", "from", "get", "global",
    "group", "init", "into", "join", "let", "managed", "nameof", "not",
    "notnull", "on", "or", "orderby", "partial", "record", "remove",
    "required", "scoped", "select", "set", "unmanaged", "value", "var",
    "when", "where", "with", "yield",
];

/// Classify an import namespace into a group for ordering.
/// 0 = System.*, 1 = Microsoft.*, 2 = everything else.
fn import_group_order(module: &str) -> u8 {
    if module.starts_with("System.") || module == "System" {
        0
    } else if module.starts_with("Microsoft.") || module == "Microsoft" {
        1
    } else {
        2
    }
}

impl RendererLang for CSharp {
    fn file_extension(&self) -> &str {
        &self.extension
    }

    fn reserved_words(&self) -> &[&str] {
        CSHARP_RESERVED
    }

    fn escape_reserved(&self, name: &str) -> String {
        if self.reserved_words().contains(&name) {
            format!("@{name}")
        } else {
            name.to_string()
        }
    }

    fn render_attribute(&self, text: &str) -> String {
        format!("[{text}]")
    }

    fn render_verbatim_string(&self, s: &str) -> String {
        let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
        format!("$\"{escaped}\"")
    }

    fn line_comment_prefix(&self) -> &str {
        "//"
    }

    fn type_presentation(&self) -> crate::lang::config::TypePresentationConfig<'_> {
        crate::lang::config::TypePresentationConfig {
            array: crate::type_name::TypePresentation::GenericWrap { name: "List" },
            readonly_array: Some(crate::type_name::TypePresentation::GenericWrap {
                name: "IReadOnlyList",
            }),
            optional: crate::type_name::TypePresentation::Postfix { suffix: "?" },
            associated_type: crate::type_name::AssociatedTypeStyle::DotAccess,
            ..Default::default()
        }
    }

    fn generic_syntax(&self) -> crate::lang::config::GenericSyntaxConfig<'_> {
        crate::lang::config::GenericSyntaxConfig {
            constraint_keyword: " : ",
            constraint_separator: ", ",
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
            field_terminator: ";",
            ..Default::default()
        }
    }
}

const CS_CLASS_CAPABILITIES: &[TypeCapability] = &[
    // RecordFields = fields
    TypeCapability::RecordFields,
    // Methods = methods
    TypeCapability::Methods,
    // NominalSubtyping = base classes and interfaces (`:`)
    TypeCapability::NominalSubtyping,
    // ParametricPolymorphism = generic type parameters
    TypeCapability::ParametricPolymorphism,
    // BoundedPolymorphism = generic constraints (`where`)
    TypeCapability::BoundedPolymorphism,
    // Attributes = `[Attribute]`
    TypeCapability::Attributes,
    // OptionalRecordFields = nullable fields
    TypeCapability::OptionalRecordFields,
];
const CS_STRUCT_CAPABILITIES: &[TypeCapability] = &[
    // RecordFields = fields
    TypeCapability::RecordFields,
    // Methods = methods
    TypeCapability::Methods,
    // ParametricPolymorphism = generic type parameters
    TypeCapability::ParametricPolymorphism,
    // Attributes = `[Attribute]`
    TypeCapability::Attributes,
    // OptionalRecordFields = nullable fields
    TypeCapability::OptionalRecordFields,
];
const CS_CONTRACT_CAPABILITIES: &[TypeCapability] = &[
    // Methods = methods
    TypeCapability::Methods,
    // NominalSubtyping = base classes and interfaces (`:`)
    TypeCapability::NominalSubtyping,
    // ParametricPolymorphism = generic type parameters
    TypeCapability::ParametricPolymorphism,
    // BoundedPolymorphism = generic constraints (`where`)
    TypeCapability::BoundedPolymorphism,
    // Attributes = `[Attribute]`
    TypeCapability::Attributes,
];
const CS_TYPES: &[TypeCapabilityProfile] = &[
    TypeCapabilityProfile::new(TypeKind::Class, CS_CLASS_CAPABILITIES),
    TypeCapabilityProfile::new(TypeKind::Struct, CS_STRUCT_CAPABILITIES),
    TypeCapabilityProfile::new(TypeKind::Interface, CS_CONTRACT_CAPABILITIES),
    TypeCapabilityProfile::new(TypeKind::Trait, CS_CONTRACT_CAPABILITIES),
    TypeCapabilityProfile::new(
        TypeKind::Enum,
        &[
            // Variants = enum members
            TypeCapability::Variants,
            // Attributes = `[Attribute]`
            TypeCapability::Attributes,
        ],
    ),
];

const CS_VARIANTS: &[VariantCapabilityProfile] = &[VariantCapabilityProfile::new(
    TypeKind::Enum,
    &[
        VariantCapability::Discriminant,
        VariantCapability::Attributes,
    ],
)];

const CS_MEMBER_FUNCTION_CAPABILITIES: &[FunctionCapability] = &[
    // AbstractMethod = abstract
    FunctionCapability::AbstractMethod,
    // AsyncEffect = async
    FunctionCapability::AsyncEffect,
    // Attributes = [...]
    FunctionCapability::Attributes,
    // BoundedPolymorphism = type parameter constraints
    FunctionCapability::BoundedPolymorphism,
    // DefaultParameters = default parameter values
    FunctionCapability::DefaultParameters,
    // ExplicitReturnType = method result type
    FunctionCapability::ExplicitReturnType,
    // TypedParameters = parameter declarations
    FunctionCapability::TypedParameters,
    // Override = override
    FunctionCapability::Override,
    // ParametricPolymorphism = generic type parameters
    FunctionCapability::ParametricPolymorphism,
    // StaticMethod = static
    FunctionCapability::StaticMethod,
];
const CS_INTERFACE_FUNCTION_CAPABILITIES: &[FunctionCapability] = &[
    FunctionCapability::AbstractMethod,
    FunctionCapability::Attributes,
    FunctionCapability::BoundedPolymorphism,
    FunctionCapability::DefaultParameters,
    FunctionCapability::ExplicitReturnType,
    FunctionCapability::TypedParameters,
    FunctionCapability::ParametricPolymorphism,
    FunctionCapability::StaticMethod,
];
const CS_CONSTRUCTOR_CAPABILITIES: &[FunctionCapability] = &[
    FunctionCapability::Attributes,
    FunctionCapability::DefaultParameters,
    FunctionCapability::StaticConstructor,
    FunctionCapability::TypedParameters,
];
const CS_REQUIRED_FUNCTION_CAPABILITIES: &[FunctionCapability] = &[
    FunctionCapability::ExplicitReturnType,
    FunctionCapability::TypedParameters,
];
const CS_MEMBER_INCOMPATIBILITIES: &[(FunctionCapability, FunctionCapability)] = &[
    (
        FunctionCapability::AbstractMethod,
        FunctionCapability::AsyncEffect,
    ),
    (
        FunctionCapability::AbstractMethod,
        FunctionCapability::StaticMethod,
    ),
    (
        FunctionCapability::StaticMethod,
        FunctionCapability::Override,
    ),
];
const CS_FUNCTIONS: &[FunctionCapabilityProfile] = &[
    FunctionCapabilityProfile::new(
        FunctionContext::Member,
        FunctionForm::Function,
        CS_MEMBER_FUNCTION_CAPABILITIES,
    )
    .with_required_capabilities(CS_REQUIRED_FUNCTION_CAPABILITIES)
    .with_body_policy(FunctionBodyPolicy::Required)
    .with_incompatible_capabilities(CS_MEMBER_INCOMPATIBILITIES),
    FunctionCapabilityProfile::new(
        FunctionContext::Member,
        FunctionForm::Constructor,
        CS_CONSTRUCTOR_CAPABILITIES,
    )
    .with_required_capabilities(&[FunctionCapability::TypedParameters])
    .with_body_policy(FunctionBodyPolicy::Required),
    FunctionCapabilityProfile::new(
        FunctionContext::InterfaceMember,
        FunctionForm::Function,
        CS_INTERFACE_FUNCTION_CAPABILITIES,
    )
    .with_required_capabilities(CS_REQUIRED_FUNCTION_CAPABILITIES),
];

impl CodeLang for CSharp {
    fn capabilities(&self) -> LanguageCapabilities<'_> {
        LanguageCapabilities::strict()
            .with_types(CS_TYPES)
            .with_functions(CS_FUNCTIONS)
            .with_variants(CS_VARIANTS)
    }

    fn lower_variants(
        &self,
        variants: crate::lang::ValidatedVariants<'_>,
    ) -> Result<CodeBlock, SigilStitchError> {
        crate::lang::variant_lowering::csharp::lower(self, variants)
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

    fn constructor_name_matches(&self, name: &str, declaring_type: Option<&str>) -> bool {
        declaring_type.is_some_and(|declaring_type| name == declaring_type)
    }

    fn constructor_name_is_valid(&self, name: &str, declaring_type: Option<&str>) -> bool {
        declaring_type.is_none_or(|declaring_type| name == declaring_type)
    }

    fn abstract_type_modifier_is_valid(&self, kind: TypeKind) -> bool {
        kind == TypeKind::Class
    }

    fn function_body_policy(
        &self,
        context: FunctionContext,
        form: FunctionForm,
        is_static: bool,
    ) -> FunctionBodyPolicy {
        if context == FunctionContext::InterfaceMember
            && form == FunctionForm::Function
            && is_static
        {
            FunctionBodyPolicy::Required
        } else {
            self.capabilities().function_body_policy(context, form)
        }
    }

    fn maximum_function_parameters(
        &self,
        context: FunctionContext,
        form: FunctionForm,
        is_static: bool,
    ) -> Option<usize> {
        if context == FunctionContext::Member && form == FunctionForm::Constructor && is_static {
            Some(0)
        } else {
            self.capabilities()
                .maximum_function_parameters(context, form)
        }
    }

    fn function_visibility_is_valid(
        &self,
        context: FunctionContext,
        form: FunctionForm,
        is_static: bool,
        visibility: Visibility,
    ) -> bool {
        if context == FunctionContext::InterfaceMember {
            return matches!(visibility, Visibility::Inherited | Visibility::Public);
        }
        if matches!(
            visibility,
            Visibility::PublicCrate | Visibility::PublicSuper
        ) {
            return false;
        }
        context != FunctionContext::Member
            || form != FunctionForm::Constructor
            || !is_static
            || visibility == Visibility::Inherited
    }

    fn lower_function(
        &self,
        function: crate::spec::fun_spec::ValidatedFunction<'_>,
    ) -> Result<CodeBlock, SigilStitchError> {
        crate::lang::csharp_function_lowering::lower(self, function)
    }

    fn function_parameters_require_trailing_defaults(
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

        let mut system_imports: Vec<String> = Vec::new();
        let mut microsoft_imports: Vec<String> = Vec::new();
        let mut other_imports: Vec<String> = Vec::new();

        let mut seen = std::collections::BTreeSet::new();
        for entry in imports.entries() {
            if entry.is_side_effect {
                continue;
            }

            let ns = &entry.module;
            if !seen.insert(ns.clone()) {
                continue;
            }

            let line = format!("using {ns};");

            match import_group_order(ns) {
                0 => system_imports.push(line),
                1 => microsoft_imports.push(line),
                _ => other_imports.push(line),
            }
        }

        system_imports.sort();
        microsoft_imports.sort();
        other_imports.sort();

        let groups: Vec<&Vec<String>> = [&system_imports, &microsoft_imports, &other_imports]
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

    fn render_visibility(&self, vis: Visibility, ctx: DeclarationContext) -> &str {
        match ctx {
            // Interface members are implicitly public in C#; no visibility keyword.
            DeclarationContext::InterfaceMember => "",
            _ => match vis {
                Visibility::Public => "public ",
                Visibility::Private => "private ",
                Visibility::Protected => "protected ",
                _ => "internal ",
            },
        }
    }

    fn function_keyword(&self, _ctx: DeclarationContext) -> &str {
        ""
    }

    fn type_keyword(&self, kind: TypeKind) -> &str {
        match kind {
            TypeKind::Class => "class",
            TypeKind::Struct => "struct",
            TypeKind::Interface | TypeKind::Trait => "interface",
            TypeKind::Enum => "enum",
            TypeKind::TypeAlias => "class",
            TypeKind::Newtype => "record",
        }
    }

    fn methods_inside_type_body(&self, _kind: TypeKind) -> bool {
        true
    }

    fn optional_field_style(&self) -> crate::lang::config::OptionalFieldStyle {
        crate::lang::config::OptionalFieldStyle::TypeSuffix("?")
    }

    fn function_syntax(&self) -> crate::lang::config::FunctionSyntaxConfig<'_> {
        crate::lang::config::FunctionSyntaxConfig {
            return_type_separator: " ",
            async_keyword: "async ",
            suppress_async_in_interface: true,
            where_clause_style: crate::spec::where_spec::WhereClauseStyle::SeparateWhere,
            ..Default::default()
        }
    }

    fn type_decl_syntax(&self) -> crate::lang::config::TypeDeclSyntaxConfig<'_> {
        crate::lang::config::TypeDeclSyntaxConfig {
            type_before_name: true,
            return_type_is_prefix: true,
            super_type_keyword: " : ",
            ..Default::default()
        }
    }

    fn enum_and_annotation(&self) -> crate::lang::config::EnumAndAnnotationConfig<'_> {
        crate::lang::config::EnumAndAnnotationConfig {
            annotation_prefix: "[",
            annotation_suffix: "]",
            readonly_keyword: "readonly ",
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
        let cs = CSharp::new();
        assert_eq!(cs.file_extension(), "cs");
    }

    #[test]
    fn test_escape_reserved_at_prefix() {
        let cs = CSharp::new();
        assert_eq!(cs.escape_reserved("class"), "@class");
        assert_eq!(cs.escape_reserved("namespace"), "@namespace");
        assert_eq!(cs.escape_reserved("event"), "@event");
        assert_eq!(cs.escape_reserved("name"), "name");
        assert_eq!(cs.escape_reserved("user"), "user");
    }

    #[test]
    fn test_render_imports_single() {
        let cs = CSharp::new();
        let imports = ImportGroup {
            entries: vec![ImportEntry {
                module: "System.Collections.Generic".into(),
                name: "List".into(),
                alias: None,
                is_type_only: false,
                is_side_effect: false,
                is_wildcard: false,
            }],
        };
        assert_eq!(
            cs.render_imports(&imports),
            "using System.Collections.Generic;"
        );
    }

    #[test]
    fn test_render_imports_grouped() {
        let cs = CSharp::new();
        let imports = ImportGroup {
            entries: vec![
                ImportEntry {
                    module: "MyApp.Models".into(),
                    name: "User".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "System.Collections.Generic".into(),
                    name: "List".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "Microsoft.Extensions.Logging".into(),
                    name: "ILogger".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
            ],
        };
        let output = cs.render_imports(&imports);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines[0], "using System.Collections.Generic;");
        assert_eq!(lines[1], "");
        assert_eq!(lines[2], "using Microsoft.Extensions.Logging;");
        assert_eq!(lines[3], "");
        assert_eq!(lines[4], "using MyApp.Models;");
    }

    #[test]
    fn test_render_imports_sorted_within_group() {
        let cs = CSharp::new();
        let imports = ImportGroup {
            entries: vec![
                ImportEntry {
                    module: "System.Threading.Tasks".into(),
                    name: "Task".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "System.Collections.Generic".into(),
                    name: "List".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "System.Linq".into(),
                    name: "Enumerable".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
            ],
        };
        let output = cs.render_imports(&imports);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines[0], "using System.Collections.Generic;");
        assert_eq!(lines[1], "using System.Linq;");
        assert_eq!(lines[2], "using System.Threading.Tasks;");
    }

    #[test]
    fn test_render_imports_dedup() {
        let cs = CSharp::new();
        let imports = ImportGroup {
            entries: vec![
                ImportEntry {
                    module: "System.Linq".into(),
                    name: "Enumerable".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "System.Linq".into(),
                    name: "Queryable".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
            ],
        };
        assert_eq!(cs.render_imports(&imports), "using System.Linq;");
    }

    #[test]
    fn test_doc_comment_single() {
        let cs = CSharp::new();
        assert_eq!(
            cs.render_doc_comment(&["A brief description."]),
            "/// A brief description."
        );
    }

    #[test]
    fn test_doc_comment_multi() {
        let cs = CSharp::new();
        let doc = cs.render_doc_comment(&[
            "<summary>",
            "Container class.",
            "</summary>",
            "",
            "<typeparam name=\"T\">The element type.</typeparam>",
        ]);
        assert_eq!(
            doc,
            "/// <summary>\n/// Container class.\n/// </summary>\n///\n/// <typeparam name=\"T\">The element type.</typeparam>"
        );
    }

    #[test]
    fn test_string_literal() {
        let cs = CSharp::new();
        assert_eq!(cs.render_string_literal("hello"), "\"hello\"");
        assert_eq!(cs.render_string_literal("it\"s"), "\"it\\\"s\"");
        assert_eq!(cs.render_string_literal("new\nline"), "\"new\\nline\"");
    }

    #[test]
    fn test_type_keyword() {
        let cs = CSharp::new();
        assert_eq!(cs.type_keyword(TypeKind::Class), "class");
        assert_eq!(cs.type_keyword(TypeKind::Struct), "struct");
        assert_eq!(cs.type_keyword(TypeKind::Interface), "interface");
        assert_eq!(cs.type_keyword(TypeKind::Trait), "interface");
        assert_eq!(cs.type_keyword(TypeKind::Enum), "enum");
        assert_eq!(cs.type_keyword(TypeKind::Newtype), "record");
    }

    #[test]
    fn test_visibility() {
        let cs = CSharp::new();
        assert_eq!(
            cs.render_visibility(Visibility::Public, DeclarationContext::TopLevel),
            "public "
        );
        assert_eq!(
            cs.render_visibility(Visibility::Private, DeclarationContext::Member),
            "private "
        );
        assert_eq!(
            cs.render_visibility(Visibility::Protected, DeclarationContext::Member),
            "protected "
        );
    }

    #[test]
    fn test_type_before_name() {
        let cs = CSharp::new();
        assert!(cs.type_decl_syntax().type_before_name);
    }

    #[test]
    fn test_return_type_is_prefix() {
        let cs = CSharp::new();
        assert!(cs.type_decl_syntax().return_type_is_prefix);
    }

    #[test]
    fn test_readonly_keyword() {
        let cs = CSharp::new();
        assert_eq!(cs.enum_and_annotation().readonly_keyword, "readonly ");
    }

    #[test]
    fn test_async_keyword() {
        let cs = CSharp::new();
        assert_eq!(cs.function_syntax().async_keyword, "async ");
    }

    #[test]
    fn test_import_group_order() {
        assert_eq!(import_group_order("System.Collections.Generic"), 0);
        assert_eq!(import_group_order("System.Linq"), 0);
        assert_eq!(import_group_order("System"), 0);
        assert_eq!(import_group_order("Microsoft.Extensions.Logging"), 1);
        assert_eq!(import_group_order("MyApp.Models"), 2);
        assert_eq!(import_group_order("Newtonsoft.Json"), 2);
    }

    #[test]
    fn test_csharp_builder_fluent() {
        let cs = CSharp::new().with_indent("\t").with_extension("csx");
        assert_eq!(cs.file_extension(), "csx");
        assert_eq!(cs.block_syntax().indent_unit, "\t");
    }

    #[test]
    fn test_module_separator() {
        let cs = CSharp::new();
        assert_eq!(cs.module_separator(), Some("."));
    }

    #[test]
    fn test_nullable_suffix() {
        let cs = CSharp::new();
        assert_eq!(
            cs.optional_field_style(),
            crate::lang::config::OptionalFieldStyle::TypeSuffix("?")
        );
    }
}
