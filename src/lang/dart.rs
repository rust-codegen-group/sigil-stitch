//! Dart language implementation.

use crate::error::SigilStitchError;
use crate::import::ImportGroup;
use crate::lang::capability::{
    FunctionBodyPolicy, FunctionCapability, FunctionCapabilityProfile, FunctionContext,
    FunctionForm, LanguageCapabilities, TypeCapability, TypeCapabilityProfile, VariantCapability,
    VariantCapabilityProfile,
};
use crate::lang::{CodeLang, RendererLang};
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};

/// Dart language implementation.
///
/// Dart-specific behaviors:
/// - Type-before-name declarations (`String name`, not `name: String`)
/// - Return type as prefix (`String getName()`, `Future<User> fetch()`)
/// - `import 'package:foo/bar.dart';` with dart/package/relative grouping
/// - Semicolons after statements and field declarations
/// - No function keyword (like Java/C)
/// - No visibility keywords (privacy via `_` prefix naming convention)
/// - `class`, `abstract class`, `enum`, `mixin` keywords
/// - `extends` for superclass, `implements` for interfaces
/// - `final` for readonly fields
/// - `///` dartdoc comments
/// - `<T extends Bound>` generics (same as Java/TS)
/// - `@override`, `@required` annotations via `annotation()`
/// - `async` as a body modifier suffix (`Future<int> foo() async { ... }`)
///
/// # Import conventions
///
/// Use [`crate::type_name::TypeName::importable`] with the import URI as module:
/// ```text
/// TypeName::importable("dart:async", "Future")              // import 'dart:async';
/// TypeName::importable("package:http/http.dart", "Client")   // import 'package:http/http.dart';
/// TypeName::importable("../models/user.dart", "User")        // import '../models/user.dart';
/// ```
///
/// Dart imports entire files, so the module (URI) is what matters for import emission.
///
/// # Mixins
///
/// Dart's `with` keyword for mixin application is not directly in the trait.
/// Include mixins via `TypeName::raw`:
/// ```text
/// tb.extends(TypeName::raw("BaseClass with Mixin1, Mixin2"));
/// // Emits: class Foo extends BaseClass with Mixin1, Mixin2 {
/// ```
///
/// # Async functions
///
/// Dart's `async` is a body modifier (`Future<int> foo() async { ... }`),
/// not a signature prefix. Set `is_async()` on the builder and use
/// `Future<T>` as the return type:
/// ```text
/// fb.returns(TypeName::primitive("Future<User>"))
///   .is_async();
/// ```
#[derive(Debug, Clone)]
pub struct Dart {
    /// Indent with this string (default: "  " — 2 spaces per Dart style guide).
    pub indent: String,
    /// File extension (default: "dart").
    pub extension: String,
}

impl Default for Dart {
    fn default() -> Self {
        Self {
            indent: "  ".to_string(),
            extension: "dart".to_string(),
        }
    }
}

impl Dart {
    /// Create a new Dart language instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the indent string (e.g., `"  "` for 2-space default, `"    "` for 4 spaces).
    pub fn with_indent(mut self, s: &str) -> Self {
        self.indent = s.to_string();
        self
    }

    /// Set the file extension (default: `"dart"`).
    pub fn with_extension(mut self, s: &str) -> Self {
        self.extension = s.to_string();
        self
    }
}

#[rustfmt::skip]
const DART_RESERVED: &[&str] = &[
    // Keywords
    "abstract", "as", "assert", "async", "await", "base", "break", "case",
    "catch", "class", "const", "continue", "covariant", "default", "deferred",
    "do", "dynamic", "else", "enum", "export", "extends", "extension",
    "external", "factory", "false", "final", "finally", "for", "Function",
    "get", "hide", "if", "implements", "import", "in", "interface", "is",
    "late", "library", "mixin", "new", "null", "of", "on", "operator",
    "part", "required", "rethrow", "return", "sealed", "set", "show",
    "static", "super", "switch", "sync", "this", "throw", "true", "try",
    "typedef", "var", "void", "when", "while", "with", "yield",
];

/// Classify a Dart import URI into a group for ordering.
/// 0 = dart:* (SDK), 1 = package:* (pub packages), 2 = relative imports.
fn import_group_order(module: &str) -> u8 {
    if module.starts_with("dart:") {
        0
    } else if module.starts_with("package:") {
        1
    } else {
        2
    }
}

impl RendererLang for Dart {
    fn file_extension(&self) -> &str {
        &self.extension
    }

    fn reserved_words(&self) -> &[&str] {
        DART_RESERVED
    }

    fn render_string_literal(&self, s: &str) -> String {
        // Dart prefers single quotes by convention.
        format!(
            "'{}'",
            s.replace('\\', "\\\\")
                .replace('\'', "\\'")
                .replace('\n', "\\n")
                .replace('\t', "\\t")
                .replace('\r', "\\r")
                .replace('\0', "\\0")
                .replace('$', "\\$")
        )
    }

    fn render_verbatim_string(&self, s: &str) -> String {
        let escaped = s.replace('\\', "\\\\").replace('\'', "\\'");
        format!("'{escaped}'")
    }

    fn line_comment_prefix(&self) -> &str {
        "//"
    }

    fn type_presentation(&self) -> crate::lang::config::TypePresentationConfig<'_> {
        crate::lang::config::TypePresentationConfig {
            array: crate::type_name::TypePresentation::GenericWrap { name: "List" },
            readonly_array: Some(crate::type_name::TypePresentation::GenericWrap { name: "List" }),
            optional: crate::type_name::TypePresentation::Postfix { suffix: "?" },
            function: crate::type_name::FunctionPresentation {
                keyword: " Function",
                params_open: "(",
                params_sep: ", ",
                params_close: ")",
                arrow: "",
                return_first: true,
                curried: false,
                wrapper_open: "",
                wrapper_close: "",
            },
            ..Default::default()
        }
    }

    fn generic_syntax(&self) -> crate::lang::config::GenericSyntaxConfig<'_> {
        crate::lang::config::GenericSyntaxConfig {
            constraint_keyword: " extends ",
            constraint_separator: ", ",
            context_bound_keyword: " extends ",
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

const DART_CLASS_CAPABILITIES: &[TypeCapability] = &[
    // RecordFields = instance fields
    TypeCapability::RecordFields,
    // Methods = methods
    TypeCapability::Methods,
    // NominalSubtyping = `extends`
    TypeCapability::NominalSubtyping,
    // InterfaceImplementation = `implements`
    TypeCapability::InterfaceImplementation,
    // ParametricPolymorphism = generic type parameters
    TypeCapability::ParametricPolymorphism,
    // Attributes = metadata annotations
    TypeCapability::Attributes,
    // OptionalRecordFields = nullable fields
    TypeCapability::OptionalRecordFields,
];
const DART_CONTRACT_CAPABILITIES: &[TypeCapability] = &[
    // Methods = methods
    TypeCapability::Methods,
    // NominalSubtyping = `extends`
    TypeCapability::NominalSubtyping,
    // InterfaceImplementation = `implements`
    TypeCapability::InterfaceImplementation,
    // ParametricPolymorphism = generic type parameters
    TypeCapability::ParametricPolymorphism,
    // Attributes = metadata annotations
    TypeCapability::Attributes,
];
const DART_TYPES: &[TypeCapabilityProfile] = &[
    TypeCapabilityProfile::new(TypeKind::Class, DART_CLASS_CAPABILITIES),
    // Struct is represented as a Dart class.
    TypeCapabilityProfile::new(TypeKind::Struct, DART_CLASS_CAPABILITIES),
    TypeCapabilityProfile::new(TypeKind::Interface, DART_CONTRACT_CAPABILITIES),
    // Trait is represented as a Dart abstract class.
    TypeCapabilityProfile::new(TypeKind::Trait, DART_CONTRACT_CAPABILITIES),
    TypeCapabilityProfile::new(
        TypeKind::Enum,
        &[
            // Variants = enum values
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
];

const DART_VARIANTS: &[VariantCapabilityProfile] = &[VariantCapabilityProfile::new(
    TypeKind::Enum,
    &[VariantCapability::Attributes],
)];

const DART_TOP_LEVEL_FUNCTION_CAPABILITIES: &[FunctionCapability] = &[
    // AsyncEffect = async
    FunctionCapability::AsyncEffect,
    // Attributes = metadata annotations
    FunctionCapability::Attributes,
    // BoundedPolymorphism = generic bounds
    FunctionCapability::BoundedPolymorphism,
    // ExplicitReturnType = function result type
    FunctionCapability::ExplicitReturnType,
    FunctionCapability::TypedParameters,
    // ParametricPolymorphism = generic type parameters
    FunctionCapability::ParametricPolymorphism,
];
const DART_MEMBER_FUNCTION_CAPABILITIES: &[FunctionCapability] = &[
    // AbstractMethod = abstract
    FunctionCapability::AbstractMethod,
    // AsyncEffect = async
    FunctionCapability::AsyncEffect,
    // Attributes = metadata annotations
    FunctionCapability::Attributes,
    // BoundedPolymorphism = generic bounds
    FunctionCapability::BoundedPolymorphism,
    // ExplicitReturnType = method result type
    FunctionCapability::ExplicitReturnType,
    FunctionCapability::TypedParameters,
    // ParametricPolymorphism = generic type parameters
    FunctionCapability::ParametricPolymorphism,
    // StaticMethod = static
    FunctionCapability::StaticMethod,
];
const DART_INTERFACE_FUNCTION_CAPABILITIES: &[FunctionCapability] = &[
    FunctionCapability::AbstractMethod,
    FunctionCapability::AsyncEffect,
    FunctionCapability::Attributes,
    FunctionCapability::BoundedPolymorphism,
    FunctionCapability::ExplicitReturnType,
    FunctionCapability::TypedParameters,
    FunctionCapability::ParametricPolymorphism,
    FunctionCapability::StaticMethod,
];
const DART_CONSTRUCTOR_CAPABILITIES: &[FunctionCapability] = &[
    FunctionCapability::Attributes,
    FunctionCapability::ConstructorDelegation,
    FunctionCapability::TypedParameters,
];
const DART_MEMBER_INCOMPATIBILITIES: &[(FunctionCapability, FunctionCapability)] = &[
    (
        FunctionCapability::AbstractMethod,
        FunctionCapability::AsyncEffect,
    ),
    (
        FunctionCapability::AbstractMethod,
        FunctionCapability::StaticMethod,
    ),
];
const DART_FUNCTIONS: &[FunctionCapabilityProfile] = &[
    FunctionCapabilityProfile::new(
        FunctionContext::TopLevel,
        FunctionForm::Function,
        DART_TOP_LEVEL_FUNCTION_CAPABILITIES,
    )
    .with_body_policy(FunctionBodyPolicy::Required),
    FunctionCapabilityProfile::new(
        FunctionContext::Member,
        FunctionForm::Function,
        DART_MEMBER_FUNCTION_CAPABILITIES,
    )
    .with_incompatible_capabilities(DART_MEMBER_INCOMPATIBILITIES)
    .with_body_policy(FunctionBodyPolicy::Required),
    FunctionCapabilityProfile::new(
        FunctionContext::Member,
        FunctionForm::Constructor,
        DART_CONSTRUCTOR_CAPABILITIES,
    ),
    FunctionCapabilityProfile::new(
        FunctionContext::InterfaceMember,
        FunctionForm::Function,
        DART_INTERFACE_FUNCTION_CAPABILITIES,
    )
    .with_incompatible_capabilities(DART_MEMBER_INCOMPATIBILITIES),
    FunctionCapabilityProfile::new(
        FunctionContext::InterfaceMember,
        FunctionForm::Constructor,
        DART_CONSTRUCTOR_CAPABILITIES,
    ),
];

impl CodeLang for Dart {
    fn capabilities(&self) -> LanguageCapabilities<'_> {
        LanguageCapabilities::strict()
            .with_types(DART_TYPES)
            .with_functions(DART_FUNCTIONS)
            .with_variants(DART_VARIANTS)
    }

    fn validate_variants(
        &self,
        variants: crate::lang::VariantIntent<'_>,
    ) -> Result<(), SigilStitchError> {
        crate::lang::variant_lowering::dart::validate(self, variants)
    }

    fn collect_variant_validation_errors(
        &self,
        variants: crate::lang::VariantIntent<'_>,
        errors: &mut Vec<SigilStitchError>,
    ) {
        crate::lang::variant_lowering::dart::collect_validation_errors(self, variants, errors);
    }

    fn lower_variants(
        &self,
        variants: crate::lang::ValidatedVariants<'_>,
    ) -> Result<crate::code_block::CodeBlock, SigilStitchError> {
        crate::lang::variant_lowering::dart::lower(self, variants)
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
        declaring_type.is_some_and(|declaring_type| {
            name == declaring_type
                || name
                    .strip_prefix(declaring_type)
                    .and_then(|suffix| suffix.strip_prefix('.'))
                    .is_some_and(|constructor| {
                        !constructor.is_empty() && !constructor.contains('.')
                    })
        })
    }

    fn constructor_name_is_valid(&self, name: &str, declaring_type: Option<&str>) -> bool {
        match declaring_type {
            Some(declaring_type) => self.constructor_name_matches(name, Some(declaring_type)),
            None => {
                let mut parts = name.split('.');
                parts.next().is_some_and(|part| !part.is_empty())
                    && parts.next().is_none_or(|part| !part.is_empty())
                    && parts.next().is_none()
            }
        }
    }

    fn abstract_type_modifier_is_valid(&self, kind: TypeKind) -> bool {
        matches!(kind, TypeKind::Class | TypeKind::Struct)
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

    fn render_imports(&self, imports: &ImportGroup) -> String {
        if imports.entries().is_empty() {
            return String::new();
        }

        // Dart imports entire files — deduplicate at the module (URI) level.
        let mut dart_imports: Vec<String> = Vec::new();
        let mut package_imports: Vec<String> = Vec::new();
        let mut relative_imports: Vec<String> = Vec::new();

        let mut seen = std::collections::BTreeSet::new();
        for entry in imports.entries() {
            if !seen.insert(&entry.module) {
                continue;
            }

            let line = format!("import '{}';", entry.module);
            match import_group_order(&entry.module) {
                0 => dart_imports.push(line),
                1 => package_imports.push(line),
                _ => relative_imports.push(line),
            }
        }

        dart_imports.sort();
        package_imports.sort();
        relative_imports.sort();

        let groups: Vec<&Vec<String>> = [&dart_imports, &package_imports, &relative_imports]
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
        // Dartdoc uses /// line-prefix style.
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
        // Dart has no visibility keywords; privacy is via _ prefix naming.
        ""
    }

    fn function_keyword(&self, _ctx: DeclarationContext) -> &str {
        // Dart has no function keyword (like Java/C).
        ""
    }

    fn type_keyword(&self, kind: TypeKind) -> &str {
        match kind {
            TypeKind::Class | TypeKind::Struct => "class",
            TypeKind::Interface | TypeKind::Trait => "abstract class",
            TypeKind::Enum => "enum",
            TypeKind::TypeAlias => "typedef",
            TypeKind::Newtype => "class",
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
            abstract_keyword: "",
            async_keyword: "",
            async_suffix: " async",
            constructor_delegation_style:
                crate::spec::modifiers::ConstructorDelegationStyle::Signature,
            ..Default::default()
        }
    }

    fn type_decl_syntax(&self) -> crate::lang::config::TypeDeclSyntaxConfig<'_> {
        crate::lang::config::TypeDeclSyntaxConfig {
            type_before_name: true,
            return_type_is_prefix: true,
            super_type_keyword: " extends ",
            implements_keyword: " implements ",
            ..Default::default()
        }
    }

    fn enum_and_annotation(&self) -> crate::lang::config::EnumAndAnnotationConfig<'_> {
        crate::lang::config::EnumAndAnnotationConfig {
            readonly_keyword: "final ",
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
        let d = Dart::new();
        assert_eq!(d.file_extension(), "dart");
    }

    #[test]
    fn test_escape_reserved() {
        let d = Dart::new();
        assert_eq!(d.escape_reserved("class"), "class_");
        assert_eq!(d.escape_reserved("import"), "import_");
        assert_eq!(d.escape_reserved("final"), "final_");
        assert_eq!(d.escape_reserved("name"), "name");
    }

    #[test]
    fn test_render_imports_single() {
        let d = Dart::new();
        let imports = ImportGroup {
            entries: vec![ImportEntry {
                module: "dart:async".into(),
                name: "Future".into(),
                alias: None,
                is_type_only: false,
                is_side_effect: false,
                is_wildcard: false,
            }],
        };
        assert_eq!(d.render_imports(&imports), "import 'dart:async';");
    }

    #[test]
    fn test_render_imports_grouped() {
        let d = Dart::new();
        let imports = ImportGroup {
            entries: vec![
                ImportEntry {
                    module: "package:http/http.dart".into(),
                    name: "Client".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "dart:async".into(),
                    name: "Future".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "../models/user.dart".into(),
                    name: "User".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
            ],
        };
        let output = d.render_imports(&imports);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines[0], "import 'dart:async';");
        assert_eq!(lines[1], "");
        assert_eq!(lines[2], "import 'package:http/http.dart';");
        assert_eq!(lines[3], "");
        assert_eq!(lines[4], "import '../models/user.dart';");
    }

    #[test]
    fn test_render_imports_sorted_within_group() {
        let d = Dart::new();
        let imports = ImportGroup {
            entries: vec![
                ImportEntry {
                    module: "dart:io".into(),
                    name: "File".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "dart:async".into(),
                    name: "Future".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "dart:convert".into(),
                    name: "json".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
            ],
        };
        let output = d.render_imports(&imports);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines[0], "import 'dart:async';");
        assert_eq!(lines[1], "import 'dart:convert';");
        assert_eq!(lines[2], "import 'dart:io';");
    }

    #[test]
    fn test_render_imports_dedup() {
        let d = Dart::new();
        let imports = ImportGroup {
            entries: vec![
                ImportEntry {
                    module: "dart:async".into(),
                    name: "Future".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                ImportEntry {
                    module: "dart:async".into(),
                    name: "Stream".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
            ],
        };
        assert_eq!(d.render_imports(&imports), "import 'dart:async';");
    }

    #[test]
    fn test_doc_comment_single() {
        let d = Dart::new();
        assert_eq!(
            d.render_doc_comment(&["A brief description."]),
            "/// A brief description."
        );
    }

    #[test]
    fn test_doc_comment_multi() {
        let d = Dart::new();
        let doc = d.render_doc_comment(&["Container class.", "", "See also [OtherClass]."]);
        assert_eq!(doc, "/// Container class.\n///\n/// See also [OtherClass].");
    }

    #[test]
    fn test_string_literal() {
        let d = Dart::new();
        assert_eq!(d.render_string_literal("hello"), "'hello'");
        assert_eq!(d.render_string_literal("it's"), "'it\\'s'");
        assert_eq!(d.render_string_literal("new\nline"), "'new\\nline'");
        // Dart needs $ escaping for string interpolation.
        assert_eq!(d.render_string_literal("$name"), "'\\$name'");
    }

    #[test]
    fn test_type_keyword() {
        let d = Dart::new();
        assert_eq!(d.type_keyword(TypeKind::Class), "class");
        assert_eq!(d.type_keyword(TypeKind::Struct), "class");
        assert_eq!(d.type_keyword(TypeKind::Interface), "abstract class");
        assert_eq!(d.type_keyword(TypeKind::Trait), "abstract class");
        assert_eq!(d.type_keyword(TypeKind::Enum), "enum");
    }

    #[test]
    fn test_no_visibility_keywords() {
        let d = Dart::new();
        assert_eq!(
            d.render_visibility(Visibility::Public, DeclarationContext::TopLevel),
            ""
        );
        assert_eq!(
            d.render_visibility(Visibility::Private, DeclarationContext::Member),
            ""
        );
        assert_eq!(
            d.render_visibility(Visibility::Protected, DeclarationContext::Member),
            ""
        );
    }

    #[test]
    fn test_type_before_name() {
        let d = Dart::new();
        assert!(d.type_decl_syntax().type_before_name);
    }

    #[test]
    fn test_return_type_is_prefix() {
        let d = Dart::new();
        assert!(d.type_decl_syntax().return_type_is_prefix);
    }

    #[test]
    fn test_readonly_keyword() {
        let d = Dart::new();
        assert_eq!(d.enum_and_annotation().readonly_keyword, "final ");
    }

    #[test]
    fn test_no_async_keyword() {
        let d = Dart::new();
        assert_eq!(d.function_syntax().async_keyword, "");
    }

    #[test]
    fn test_async_suffix() {
        let d = Dart::new();
        assert_eq!(d.function_syntax().async_suffix, " async");
    }

    #[test]
    fn test_import_group_order() {
        assert_eq!(import_group_order("dart:async"), 0);
        assert_eq!(import_group_order("dart:io"), 0);
        assert_eq!(import_group_order("package:http/http.dart"), 1);
        assert_eq!(import_group_order("package:flutter/material.dart"), 1);
        assert_eq!(import_group_order("../models/user.dart"), 2);
        assert_eq!(import_group_order("./config.dart"), 2);
    }

    #[test]
    fn test_dart_builder_fluent() {
        let d = Dart::new().with_indent("    ").with_extension("g.dart");
        assert_eq!(d.file_extension(), "g.dart");
        assert_eq!(d.block_syntax().indent_unit, "    ");
    }

    #[test]
    fn test_module_separator() {
        let d = Dart::new();
        assert_eq!(d.module_separator(), Some("."));
    }
}
