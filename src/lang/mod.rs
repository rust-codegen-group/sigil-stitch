/// Bash shell language support.
pub mod bash;
/// C language support.
pub mod c;
/// Language capability declarations for spec emission.
pub mod capability;
/// Shared configuration types (quote style, optional-field rendering).
pub mod config;
/// C++ language support.
pub mod cpp;
/// C# language support.
pub mod csharp;
/// Dart language support.
pub mod dart;
/// Go language support.
pub mod go;
/// Haskell language support.
pub mod haskell;
/// Java language support.
pub mod java;
/// JavaScript language support.
pub mod javascript;
/// Kotlin language support.
pub mod kotlin;
/// Lua language support.
pub mod lua;
/// OCaml language support.
pub mod ocaml;
/// PHP language support.
pub mod php;
/// Python language support.
pub mod python;
/// Ruby language support.
pub mod ruby;
/// Rust language support.
pub mod rust;
/// Scala language support.
pub mod scala;
/// Swift language support.
pub mod swift;
/// TypeScript language support.
pub mod typescript;
/// Zsh shell language support.
pub mod zsh;

/// Helpers for implementing language-specific node rewrite passes.
pub mod rewrite;

mod bash_function_lowering;
mod c_function_lowering;
mod compatibility_markers;
mod cpp_function_lowering;
mod csharp_function_lowering;
mod dart_function_lowering;
pub(crate) mod field_lowering;
mod function_lowering;
mod go_function_lowering;
mod haskell_function_lowering;
mod java_function_lowering;
mod javascript_function_lowering;
mod kotlin_function_lowering;
mod lua_function_lowering;
mod ocaml_function_lowering;
mod php_function_lowering;
pub(crate) mod property_lowering;
mod python_function_lowering;
mod ruby_function_lowering;
mod rust_function_lowering;
mod scala_function_lowering;
mod swift_function_lowering;
pub(crate) mod type_lowering;
pub(crate) mod type_members_validation;
mod typescript_function_lowering;
pub(crate) mod variant_lowering;
mod zsh_function_lowering;

use crate::code_block::CodeBlock;
use crate::code_node::BlockIntent;
use crate::error::SigilStitchError;
use crate::import::ImportGroup;
use crate::lang::capability::{
    FunctionBodyPolicy, FunctionCapability, FunctionContext, FunctionForm, LanguageCapabilities,
};
pub use crate::spec::enum_variant_spec::{ValidatedVariants, VariantIntent};
pub use crate::spec::field_spec::{FieldSequenceIntent, ValidatedFields};
pub use crate::spec::fun_spec::{FunctionIntent, ValidatedFunction};
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};
use crate::spec::parameter_spec::ParameterSpec;
pub use crate::spec::property_spec::{PropertyIntent, ValidatedProperty};
pub use crate::spec::type_members_intent::TypeMembersIntent;
pub use crate::spec::type_spec::{TypeIntent, ValidatedType};
use crate::spec::where_spec::{TypeParamSpec, WhereConstraint};
use crate::type_name::TypeName;
use compatibility_markers::LegacyTypeMarkers;

/// Narrow trait for `CodeRenderer` and `TypeName` rendering.
///
/// Implementors must provide only:
/// - [`file_extension`](Self::file_extension)
/// - [`line_comment_prefix`](Self::line_comment_prefix)
///
/// Every other method has a default. Some defaults preserve the frozen 0.6.8
/// external-adapter surface rather than defining a recommended grammar model:
/// [`type_presentation`](Self::type_presentation),
/// [`generic_syntax`](Self::generic_syntax), and
/// [`block_syntax`](Self::block_syntax) are deprecated compatibility accessors.
/// New adapters should follow the language-owned `lower_type_name()`,
/// `indent_unit()`, and renderer-event design in the architecture and
/// language-author guides. Those replacement interfaces land in their own
/// behavior-specific cutovers; do not extend the compatibility accessors with
/// new syntax dimensions.
pub trait RendererLang: std::fmt::Debug + 'static {
    /// File extension for this language (e.g., "ts", "go", "rs").
    fn file_extension(&self) -> &str;

    /// Render a string literal with language-appropriate quoting and escaping.
    ///
    /// Default: double-quote with C-style escaping (`\\`, `\"`, `\n`, `\t`,
    /// `\r`, `\0`). Override for languages with different quoting
    /// conventions (shell, JS/TS, Python, Kotlin, Dart, Go, Haskell,
    /// Rust, OCaml, Lua).
    fn render_string_literal(&self, s: &str) -> String {
        format!(
            "\"{}\"",
            s.replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', "\\n")
                .replace('\t', "\\t")
                .replace('\r', "\\r")
                .replace('\0', "\\0")
        )
    }

    /// Render a verbatim string literal with minimal escaping.
    ///
    /// Only escapes characters that would structurally break the string delimiter,
    /// preserving interpolation sigils (`$`, `` ` ``, `{`, etc.) as-is.
    /// For languages without string interpolation, falls back to full escaping.
    ///
    /// Default: delegates to [`render_string_literal`](Self::render_string_literal).
    fn render_verbatim_string(&self, s: &str) -> String {
        self.render_string_literal(s)
    }

    /// Single-line comment prefix (e.g., "//", "#").
    fn line_comment_prefix(&self) -> &str;

    /// Suffix appended after a single-line comment.
    ///
    /// Default: `""` (no suffix — most languages use line comments like `//`).
    /// OCaml overrides to `" *)"` to close `(* comment *)` block comments.
    fn line_comment_suffix(&self) -> &str {
        ""
    }

    /// Render an attribute / annotation with language-specific syntax.
    ///
    /// Default: `"@{text}"` (Java/Python/Kotlin/TypeScript decorator style).
    /// Override for Rust (`#[text]`), C++ (`[[text]]`), C# (`[text]`), etc.
    fn render_attribute(&self, text: &str) -> String {
        format!("@{text}")
    }

    /// Reserved words that need escaping.
    fn reserved_words(&self) -> &[&str] {
        &[]
    }

    /// Escape a name if it collides with a reserved word.
    /// Default: append underscore.
    fn escape_reserved(&self, name: &str) -> String {
        if self.reserved_words().contains(&name) {
            format!("{name}_")
        } else {
            name.to_string()
        }
    }

    /// Frozen 0.6.8 block and statement configuration.
    ///
    /// This default preserves external-adapter source behavior. It is not the
    /// extension model for new target grammar.
    #[deprecated(
        note = "legacy 0.6.8 renderer grammar; implement the language-owned renderer event methods instead"
    )]
    #[expect(deprecated, reason = "0.6.8 compatibility default")]
    fn block_syntax(&self) -> config::BlockSyntaxConfig<'_> {
        config::BlockSyntaxConfig::default()
    }

    /// Map a control-flow condition to its block-opening delimiter.
    ///
    /// Legacy path used by old serialized/external string-only block nodes.
    /// New nodes use [`RendererLang::block_open_for_intent`].
    #[deprecated(note = "use block_open_for_intent")]
    fn block_open_for(&self, _condition: &str) -> Option<&str> {
        None
    }

    /// Map a control-flow condition to its block-closing delimiter.
    ///
    /// Legacy path used by old serialized/external string-only block nodes.
    /// New nodes use [`RendererLang::block_close_for_intent`].
    #[deprecated(note = "use block_close_for_intent")]
    fn block_close_for(&self, _condition: &str) -> Option<&str> {
        None
    }

    /// Map a control-flow block intent and condition to its block-opening
    /// delimiter.
    ///
    /// Called at render time for
    /// [`crate::code_node::CodeNode::BlockOpenIntent`] nodes. Return
    /// `Some("...")` to override the default `block_syntax().block_open`.
    ///
    /// The default delegates to the legacy [`RendererLang::block_open_for`]
    /// so existing external adapters keep working for both node forms.
    #[expect(deprecated, reason = "0.6.8 compatibility bridge")]
    fn block_open_for_intent(&self, _intent: BlockIntent, condition: &str) -> Option<&str> {
        self.block_open_for(condition)
    }

    /// Map a control-flow block intent and condition to its block-closing
    /// delimiter.
    ///
    /// Called at render time for
    /// [`crate::code_node::CodeNode::BlockCloseIntent`] and
    /// [`crate::code_node::CodeNode::BranchCloseIntent`] nodes. Return
    /// `Some("...")` to override the default `block_syntax().block_close`.
    ///
    /// The default delegates to the legacy [`RendererLang::block_close_for`]
    /// so existing external adapters keep working for both node forms.
    #[expect(deprecated, reason = "0.6.8 compatibility bridge")]
    fn block_close_for_intent(&self, _intent: BlockIntent, condition: &str) -> Option<&str> {
        self.block_close_for(condition)
    }

    /// Rewrite the node tree before rendering. Called automatically by the
    /// renderer. Default is no-op.
    fn rewrite_nodes(&self, _nodes: &mut Vec<crate::code_node::CodeNode>) {}

    /// Qualify an import name for rendering in code.
    ///
    /// Default: return the resolved name as-is.
    /// Go overrides this to prefix the package name (e.g., `"http.Server"`).
    #[deprecated(note = "legacy 0.6.8 hook; use qualify_import_reference")]
    fn qualify_import_name(&self, _module: &str, resolved_name: &str) -> String {
        resolved_name.to_string()
    }

    /// Qualify a resolved import reference while retaining its original name.
    ///
    /// The provided implementation preserves the 0.6.8 two-argument hook for
    /// external adapters. Haskell overrides this current hook because its alias
    /// spelling depends on both the original and resolved names.
    #[expect(deprecated, reason = "0.6.8 compatibility bridge")]
    fn qualify_import_reference(
        &self,
        module: &str,
        _original_name: &str,
        resolved_name: &str,
    ) -> String {
        self.qualify_import_name(module, resolved_name)
    }

    /// Frozen 0.6.8 separator for qualified inline type references.
    ///
    /// Examples include `"::"` for Rust/C++ and `"."` for Go/Python/Java.
    #[deprecated(
        note = "legacy shared type grammar; implement RendererLang::lower_type_name instead"
    )]
    fn module_separator(&self) -> Option<&str> {
        None
    }

    /// Frozen 0.6.8 compound-type rendering configuration.
    ///
    /// This default preserves external-adapter source behavior. It is not the
    /// extension model for new target type grammar.
    #[deprecated(
        note = "legacy shared type grammar; implement RendererLang::lower_type_name instead"
    )]
    #[expect(deprecated, reason = "0.6.8 compatibility default")]
    fn type_presentation(&self) -> config::TypePresentationConfig<'_> {
        config::TypePresentationConfig::default()
    }

    /// Frozen 0.6.8 generic-delimiter and constraint configuration.
    ///
    /// This default preserves external-adapter source behavior. It is not the
    /// extension model for new target declaration grammar.
    #[deprecated(
        note = "legacy shared declaration grammar; implement complete language-owned lowering instead"
    )]
    #[expect(deprecated, reason = "0.6.8 compatibility default")]
    fn generic_syntax(&self) -> config::GenericSyntaxConfig<'_> {
        config::GenericSyntaxConfig::default()
    }
}

/// Full language trait for spec-level code generation.
///
/// Extends [`RendererLang`] with the additional methods needed by the spec
/// layer (`FunSpec`, `TypeSpec`, `FieldSpec`, etc.) and `FileSpec` (imports).
///
/// Implement this when you need full `FileSpec`-level generation including
/// functions, types, fields, and imports. For basic `CodeBlock` rendering,
/// only [`RendererLang`] is required.
///
/// # Implementing complete type lowering
///
/// ```
/// use sigil_stitch::code_block::CodeBlock;
/// use sigil_stitch::error::SigilStitchError;
/// use sigil_stitch::lang::capability::{
///     LanguageCapabilities, TypeCapabilityProfile,
/// };
/// use sigil_stitch::lang::{CodeLang, RendererLang, ValidatedType};
/// use sigil_stitch::spec::file_spec::FileSpec;
/// use sigil_stitch::spec::modifiers::TypeKind;
/// use sigil_stitch::spec::type_spec::TypeSpec;
/// use sigil_stitch::type_name::TypeName;
///
/// #[derive(Debug)]
/// struct ExampleLang;
///
/// impl RendererLang for ExampleLang {
///     fn file_extension(&self) -> &str { "example" }
///     fn line_comment_prefix(&self) -> &str { "//" }
/// }
///
/// const TYPES: &[TypeCapabilityProfile<'_>] =
///     &[TypeCapabilityProfile::new(TypeKind::TypeAlias, &[])];
///
/// impl CodeLang for ExampleLang {
///     fn capabilities(&self) -> LanguageCapabilities<'_> {
///         LanguageCapabilities::strict().with_types(TYPES)
///     }
///
///     fn lower_type(
///         &self,
///         type_: ValidatedType<'_>,
///     ) -> Result<Vec<CodeBlock>, SigilStitchError> {
///         let target = type_.target_type().expect("validated alias target").clone();
///         Ok(vec![CodeBlock::of(
///             &format!("type {} = %T", type_.name()),
///             target,
///         )?])
///     }
/// }
///
/// let wrapped = TypeSpec::builder("Wrapped", TypeKind::TypeAlias)
///     .extends(TypeName::primitive("String"))
///     .build()?;
/// let output = FileSpec::builder_with("wrapped.example", ExampleLang)
///     .add_type(wrapped)
///     .build()?
///     .render(80)?;
/// assert!(output.contains("type Wrapped = String"));
/// # Ok::<(), SigilStitchError>(())
/// ```
pub trait CodeLang: RendererLang {
    // ── Capability contract ───────────────────────────────────────────

    /// Declare which spec constructs this language supports.
    ///
    /// Built-in languages return strict local matrices. Adapters written for
    /// sigil-stitch 0.6.8 inherit a permissive compatibility profile.
    fn capabilities(&self) -> LanguageCapabilities<'_> {
        LanguageCapabilities::permissive()
    }

    /// Apply target-specific validation to one complete type declaration.
    ///
    /// Intrinsic shape and capability validation run before this hook. An
    /// override adds identifier, visibility, inheritance, constructor, kind,
    /// constraint, annotation, empty-body, and opaque-member rules owned by
    /// the target grammar.
    fn validate_type(&self, _type_: TypeIntent<'_>) -> Result<(), SigilStitchError> {
        Ok(())
    }

    /// Collect target-specific type-declaration failures.
    fn collect_type_validation_errors(
        &self,
        type_: TypeIntent<'_>,
        errors: &mut Vec<SigilStitchError>,
    ) {
        if let Err(error) = self.validate_type(type_) {
            errors.push(error);
        }
    }

    /// Lower one fully validated type declaration into structured output.
    ///
    /// Permissive pre-0.6.8 adapters retain frozen compatibility lowering.
    /// Strict adapters must override this complete seam. Every implementation
    /// must return one or more non-empty blocks;
    /// [`TypeSpec::emit()`](crate::spec::type_spec::TypeSpec::emit) rejects empty
    /// output.
    fn lower_type(&self, type_: ValidatedType<'_>) -> Result<Vec<CodeBlock>, SigilStitchError> {
        if self.capabilities().type_validation_is_permissive() {
            type_lowering::lower_compatibility(self, type_)
        } else {
            Err(SigilStitchError::MissingTypeLowerer {
                language: self.file_extension().to_string(),
                kind: type_.kind(),
                type_name: type_.name().to_string(),
            })
        }
    }

    /// Semantic capability represented by the legacy `is_abstract` modifier.
    ///
    /// Most languages use [`FunctionCapability::AbstractMethod`]. C++ uses
    /// [`FunctionCapability::VirtualMethod`] to preserve the established
    /// virtual-only behavior of `is_abstract`.
    fn abstract_modifier_capability(&self) -> FunctionCapability {
        FunctionCapability::AbstractMethod
    }

    /// Classify the declaration form used for capability validation.
    ///
    /// The default distinguishes ordinary functions from declarations marked
    /// as constructors. `FunSpec` and `TypeSpec` apply implicit naming rules in
    /// member context before calling this classifier. C++ additionally
    /// recognizes its established `~Type` destructor naming convention.
    fn function_form(&self, _name: &str, is_constructor: bool) -> FunctionForm {
        if is_constructor {
            FunctionForm::Constructor
        } else {
            FunctionForm::Function
        }
    }

    /// Whether a name identifies a constructor without an explicit marker.
    ///
    /// `declaring_type` is available for declarations owned by a
    /// [`TypeSpec`](crate::spec::type_spec::TypeSpec) and absent for direct
    /// [`FunSpec`](crate::spec::fun_spec::FunSpec) emission. Languages with
    /// fixed names such as `constructor` or `init` can recognize both contexts;
    /// languages whose constructors repeat the declaring type require the owner.
    fn constructor_name_matches(&self, _name: &str, _declaring_type: Option<&str>) -> bool {
        false
    }

    /// Whether a static member with a constructor-shaped name is still a
    /// constructor declaration.
    ///
    /// This defaults to the ordinary constructor naming rule because most
    /// languages reject static constructors through the capability matrix.
    /// Languages where the same spelling is a valid ordinary static method
    /// override this hook.
    fn static_constructor_name_matches(&self, name: &str, declaring_type: Option<&str>) -> bool {
        self.constructor_name_matches(name, declaring_type)
    }

    /// Whether an owner-named declaration with an explicit return type is an
    /// ordinary function instead of a malformed constructor.
    ///
    /// Most languages reserve the owner name for constructors. Java overrides
    /// this because a return type disambiguates a same-named method.
    fn constructor_name_with_return_type_is_function(&self) -> bool {
        false
    }

    /// Whether an explicitly marked constructor has a valid name in its type.
    ///
    /// `declaring_type` is present for declarations owned by a `TypeSpec` and
    /// absent for direct `FunSpec` validation. The compatibility default
    /// accepts any name. Built-in languages with fixed or owner-derived
    /// constructor names override this hook.
    fn constructor_name_is_valid(&self, _name: &str, _declaring_type: Option<&str>) -> bool {
        true
    }

    /// Declaration context used for members of one language-level type kind.
    ///
    /// Interfaces and traits are contracts by default. Languages that render a
    /// nominal kind with concrete member semantics can override this mapping;
    /// PHP traits, for example, allow bodyful and non-public methods.
    fn type_member_declaration_context(&self, kind: TypeKind) -> DeclarationContext {
        match kind {
            TypeKind::Interface | TypeKind::Trait => DeclarationContext::InterfaceMember,
            TypeKind::Class
            | TypeKind::Struct
            | TypeKind::Enum
            | TypeKind::TypeAlias
            | TypeKind::Newtype => DeclarationContext::Member,
        }
    }

    /// Whether this language permits an explicit `abstract` modifier on a
    /// declaration of `kind`.
    ///
    /// The compatibility default preserves pre-capability adapters. Strict
    /// built-in adapters override this for the type kinds their grammar can
    /// represent as explicitly abstract declarations.
    #[deprecated(note = "legacy 0.6.8 type grammar; implement CodeLang::validate_type instead")]
    fn abstract_type_modifier_is_valid(&self, _kind: TypeKind) -> bool {
        self.capabilities().function_validation_is_permissive()
    }

    /// Whether the complete parameter list satisfies this language's typing
    /// rules when a profile requires [`FunctionCapability::TypedParameters`].
    ///
    /// The default requires every parameter to carry a type. Rust overrides
    /// this for receiver spellings such as `&self`, while Go permits a run of
    /// names to share the following parameter's type.
    fn function_parameters_are_typed(
        &self,
        parameters: &[ParameterSpec],
        _context: FunctionContext,
        _form: FunctionForm,
    ) -> bool {
        parameters
            .iter()
            .all(|parameter| !parameter.param_type().is_empty())
    }

    /// Body policy after accounting for declaration modifiers.
    ///
    /// The default uses the static capability profile. Languages whose body
    /// requirements depend on a modifier can refine it here; Java uses this
    /// for static interface methods.
    fn function_body_policy(
        &self,
        context: FunctionContext,
        form: FunctionForm,
        _is_static: bool,
    ) -> FunctionBodyPolicy {
        self.capabilities().function_body_policy(context, form)
    }

    /// Modifier-aware parameter limit for one function profile.
    ///
    /// The default uses the static capability profile. Languages with limits
    /// that depend on a modifier can refine it here; C# uses this for
    /// parameterless static constructors.
    fn maximum_function_parameters(
        &self,
        context: FunctionContext,
        form: FunctionForm,
        _is_static: bool,
    ) -> Option<usize> {
        self.capabilities()
            .maximum_function_parameters(context, form)
    }

    /// Whether a function profile accepts the selected visibility.
    ///
    /// The default accepts only inherited visibility. Languages that can
    /// preserve explicit visibility opt in to the forms and contexts they own.
    fn function_visibility_is_valid(
        &self,
        _context: FunctionContext,
        _form: FunctionForm,
        _is_static: bool,
        visibility: Visibility,
    ) -> bool {
        visibility == Visibility::Inherited
    }

    /// Whether required parameters must precede every defaulted parameter.
    fn function_parameters_require_trailing_defaults(
        &self,
        _context: FunctionContext,
        _form: FunctionForm,
    ) -> bool {
        false
    }

    /// Validate that function type constraints are representable by this
    /// language.
    ///
    /// The default is deliberately syntax-independent. Languages that lower
    /// constraints onto declared type parameters opt into the policy-free
    /// structural validator; languages with another semantic constraint model
    /// validate the complete set locally.
    fn validate_function_type_constraints(
        &self,
        _function_name: &str,
        _type_params: &[TypeParamSpec],
        _constraints: &[WhereConstraint],
    ) -> Result<(), SigilStitchError> {
        Ok(())
    }

    /// Whether any explicit function type information must form a complete
    /// typed declaration.
    ///
    /// Haskell uses this to reject partial type signatures assembled from a
    /// return type, parameter types, type parameters, or constraints. The
    /// compatibility default preserves the permissive pre-0.6.8 behavior.
    fn requires_complete_function_type_information(
        &self,
        _context: FunctionContext,
        _form: FunctionForm,
    ) -> bool {
        false
    }

    /// Apply additional target-specific validation to one classified function.
    ///
    /// `FunSpec` always applies the semantic capability matrix and shared
    /// validation hooks against this same adapter before calling this method.
    /// An override can only add target-local checks; sigil-stitch constructs
    /// the `ValidatedFunction` wrapper after both phases succeed.
    fn validate_function(&self, _function: FunctionIntent<'_>) -> Result<(), SigilStitchError> {
        Ok(())
    }

    /// Lower one fully validated function declaration into structured output.
    ///
    /// The default preserves the pre-0.6.8 syntax-configuration contract for
    /// external adapters. New language-specific grammar belongs in an override
    /// of this complete lowering seam, not in another placement or keyword
    /// hook interpreted by `FunSpec`.
    fn lower_function(
        &self,
        function: ValidatedFunction<'_>,
    ) -> Result<CodeBlock, SigilStitchError> {
        function_lowering::lower_compatibility(self, function)
    }

    /// Apply additional target-specific validation to one complete field sequence.
    ///
    /// Crate-owned modifier, duplicate-name, context, supported-capability, and
    /// required-capability checks run first. An override can only add local
    /// rules such as visibility, annotation-form, tag, or identifier checks.
    fn validate_fields(&self, _fields: FieldSequenceIntent<'_>) -> Result<(), SigilStitchError> {
        Ok(())
    }

    /// Collect target-specific failures for one complete field sequence.
    ///
    /// The default preserves adapters that implement [`CodeLang::validate_fields`]
    /// by appending its single result. Built-ins may override this to retain
    /// independent sibling failures during file validation.
    fn collect_field_validation_errors(
        &self,
        fields: FieldSequenceIntent<'_>,
        errors: &mut Vec<SigilStitchError>,
    ) {
        if let Err(error) = self.validate_fields(fields) {
            errors.push(error);
        }
    }

    /// Lower one fully validated field sequence into structured output.
    ///
    /// The default is the frozen pre-0.6.8 compatibility implementation for
    /// permissive external adapters. Built-ins override this complete seam and
    /// do not interpret shared declaration syntax configuration.
    fn lower_fields(&self, fields: ValidatedFields<'_>) -> Result<CodeBlock, SigilStitchError> {
        field_lowering::lower_compatibility(self, fields)
    }

    /// Apply additional target-specific validation to one computed property.
    ///
    /// Intrinsic and capability validation run against this same adapter first.
    /// An override can add identifier, visibility, accessor-combination, and
    /// other target-local checks before sigil-stitch constructs the
    /// `ValidatedProperty` wrapper.
    fn validate_property(&self, _property: PropertyIntent<'_>) -> Result<(), SigilStitchError> {
        Ok(())
    }

    /// Collect target-specific failures for one computed property.
    ///
    /// The default preserves adapters that implement
    /// [`CodeLang::validate_property`] by appending its single result.
    fn collect_property_validation_errors(
        &self,
        property: PropertyIntent<'_>,
        errors: &mut Vec<SigilStitchError>,
    ) {
        if let Err(error) = self.validate_property(property) {
            errors.push(error);
        }
    }

    /// Lower one fully validated property into structured output.
    ///
    /// The default is the frozen pre-0.6.8 compatibility implementation for
    /// external adapters. Built-ins override this complete seam and keep
    /// target grammar local.
    fn lower_property(
        &self,
        property: ValidatedProperty<'_>,
    ) -> Result<Vec<CodeBlock>, SigilStitchError> {
        property_lowering::lower_compatibility(self, property)
    }

    /// Apply target-specific validation to relationships among one type's members.
    ///
    /// Per-family validation has already run. This validation-only seam is for
    /// owner-wide rules such as target-derived collisions among fields,
    /// properties, and explicit methods. It does not participate in lowering.
    fn validate_type_members(
        &self,
        _members: TypeMembersIntent<'_>,
    ) -> Result<(), SigilStitchError> {
        Ok(())
    }

    /// Collect target-specific owner-wide member failures.
    ///
    /// The default preserves adapters that implement
    /// [`CodeLang::validate_type_members`] by appending its single result.
    fn collect_type_members_validation_errors(
        &self,
        members: TypeMembersIntent<'_>,
        errors: &mut Vec<SigilStitchError>,
    ) {
        if let Err(error) = self.validate_type_members(members) {
            errors.push(error);
        }
    }

    /// Apply additional target-specific validation to one owner-aware variant sequence.
    ///
    /// Intrinsic and capability validation run against this same adapter before
    /// lowering. During file-level aggregation this hook may still receive a
    /// sequence whose independent sibling produced an earlier error. Overrides
    /// must therefore inspect only the intent they understand and add
    /// target-local checks, including a validity-preserving interpretation of
    /// the deprecated `.value()` input.
    fn validate_variants(&self, _variants: VariantIntent<'_>) -> Result<(), SigilStitchError> {
        Ok(())
    }

    /// Collect target-specific validation failures for one variant sequence.
    ///
    /// The default preserves adapters that implement [`CodeLang::validate_variants`]
    /// by appending its single result. Built-in adapters with independent
    /// per-variant checks override this hook so file validation can report safe
    /// sibling failures together.
    fn collect_variant_validation_errors(
        &self,
        variants: VariantIntent<'_>,
        errors: &mut Vec<SigilStitchError>,
    ) {
        if let Err(error) = self.validate_variants(variants) {
            errors.push(error);
        }
    }

    /// Lower one fully validated, owner-aware variant sequence.
    ///
    /// The default is the frozen pre-0.6.8 compatibility implementation for
    /// permissive external adapters. Built-ins override this complete seam and
    /// do not interpret shared declaration syntax configuration.
    fn lower_variants(
        &self,
        variants: ValidatedVariants<'_>,
    ) -> Result<CodeBlock, SigilStitchError> {
        variant_lowering::lower_compatibility(self, variants)
    }

    // ── Spec-layer methods — used by FunSpec, TypeSpec, FieldSpec, etc. ───

    /// Render an import group to a string.
    ///
    /// Default: `""` (no import system).
    fn render_imports(&self, _imports: &ImportGroup) -> String {
        String::new()
    }

    /// Render a doc comment block.
    ///
    /// Default: wraps each line with `line_comment_prefix()` and
    /// `line_comment_suffix()`.
    fn render_doc_comment(&self, lines: &[&str]) -> String {
        let prefix = self.line_comment_prefix();
        let suffix = self.line_comment_suffix();
        lines
            .iter()
            .map(|line| {
                if line.is_empty() {
                    format!("{prefix}{suffix}")
                } else {
                    format!("{prefix} {line}{suffix}")
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Render a visibility modifier for the given declaration context.
    ///
    /// Default: `""` (no visibility modifiers).
    fn render_visibility(
        &self,
        _vis: crate::spec::modifiers::Visibility,
        _ctx: crate::spec::modifiers::DeclarationContext,
    ) -> &str {
        ""
    }

    /// The keyword used to declare a function (e.g., "fn", "function").
    ///
    /// Default: `""`.
    #[deprecated(
        note = "legacy 0.6.8 function grammar; implement CodeLang::lower_function instead"
    )]
    fn function_keyword(&self, _ctx: crate::spec::modifiers::DeclarationContext) -> &str {
        ""
    }

    /// Whether a constructor may use this explicit return type.
    ///
    /// The capability profile decides whether constructor return annotations
    /// exist at all. This hook handles languages such as Python that permit
    /// only a particular annotated type. The permissive default preserves
    /// behavior for adapters written before capability validation.
    fn constructor_return_type_is_valid(&self, _return_type: &TypeName) -> bool {
        true
    }

    /// The keyword for a type declaration (e.g., "struct", "class").
    ///
    /// Default: `""`.
    #[deprecated(note = "legacy 0.6.8 type grammar; implement CodeLang::lower_type instead")]
    fn type_keyword(&self, _kind: crate::spec::modifiers::TypeKind) -> &str {
        ""
    }

    /// Whether methods are declared inside the type body (true for TS class, Rust trait)
    /// vs in a separate impl block (Rust struct/enum).
    ///
    /// Default: `true`.
    #[deprecated(note = "legacy 0.6.8 type grammar; implement CodeLang::lower_type instead")]
    fn methods_inside_type_body(&self, _kind: crate::spec::modifiers::TypeKind) -> bool {
        true
    }

    /// Escape a field/property name. Languages where property names never
    /// conflict with reserved words (e.g. TypeScript) can return the name as-is.
    fn escape_field_name(&self, name: &str) -> String {
        self.escape_reserved(name)
    }

    /// Prefix applied to variable names (parameters, fields, properties,
    /// receiver names). Returns `""` by default. PHP returns `"$"`.
    #[deprecated(
        note = "legacy 0.6.8 declaration grammar; implement complete language-owned lowering instead"
    )]
    fn variable_prefix(&self) -> &str {
        ""
    }

    /// Optional kind suffix after the type name (e.g., Go's `type Foo struct`).
    ///
    /// Default: empty (TS/Rust put the kind keyword before the name).
    #[deprecated(note = "legacy 0.6.8 type grammar; implement CodeLang::lower_type instead")]
    fn type_kind_suffix(&self, _kind: crate::spec::modifiers::TypeKind) -> &str {
        ""
    }

    /// Render a newtype declaration line from pre-rendered components.
    ///
    /// This exact 0.6.8 hook remains available only for external-adapter and
    /// direct compatibility use. Current declaration lowering uses
    /// [`CodeLang::emit_newtype_decl`].
    #[deprecated(note = "legacy 0.6.8 type grammar; implement CodeLang::lower_type instead")]
    fn render_newtype_line(&self, vis: &str, name: &str, inner: &str) -> String {
        format!("{vis}struct {name}({inner});")
    }

    /// Emit a newtype declaration while preserving semantic type references.
    ///
    /// Default: Rust tuple-struct `{visibility}struct {name}<T>({inner});`.
    #[deprecated(note = "legacy 0.6.8 type grammar; implement CodeLang::lower_type instead")]
    fn emit_newtype_decl(
        &self,
        visibility: &str,
        name: &str,
        type_params: &[TypeParamSpec],
        inner: &TypeName,
    ) -> Result<CodeBlock, SigilStitchError> {
        let mut markers = LegacyTypeMarkers::new("CodeLang::render_newtype_line");
        let type_params = markers.render_marked_type_params(type_params, self)?;
        let inner = markers.mark(inner);
        #[expect(deprecated, reason = "0.6.8 compatibility bridge")]
        let output = self.render_newtype_line(visibility, &format!("{name}{type_params}"), &inner);
        markers.recover(&output)
    }

    /// Opening block delimiter for function bodies specifically.
    ///
    /// Default: `" {"`.
    #[deprecated(
        note = "legacy 0.6.8 function grammar; implement CodeLang::lower_function instead"
    )]
    fn fun_block_open(&self) -> &str {
        " {"
    }

    /// Opening block delimiter for type headers, parameterized by type kind.
    ///
    /// Default: `" {"`.
    #[deprecated(note = "legacy 0.6.8 type grammar; implement CodeLang::lower_type instead")]
    fn type_header_block_open(&self, _kind: crate::spec::modifiers::TypeKind) -> &str {
        " {"
    }

    /// Whether doc comments should be rendered inside the body (after block open)
    /// rather than above the declaration.
    ///
    /// Default: `false`. Python overrides to `true` (docstrings go inside the body).
    #[deprecated(
        note = "legacy 0.6.8 preamble grammar; implement complete declaration lowerers instead"
    )]
    fn doc_comment_inside_body(&self) -> bool {
        false
    }

    /// Whether doc comments should be emitted before annotations/attributes.
    ///
    /// Default: `true`.
    #[deprecated(
        note = "legacy 0.6.8 preamble grammar; implement complete declaration lowerers instead"
    )]
    fn doc_before_annotations(&self) -> bool {
        true
    }

    /// How this language expresses that a field is optional (key may be absent).
    ///
    /// Default: `OptionalFieldStyle::Ignored`.
    #[deprecated(note = "legacy 0.6.8 field grammar; implement CodeLang::lower_fields instead")]
    #[expect(deprecated, reason = "0.6.8 compatibility default")]
    fn optional_field_style(&self) -> crate::lang::config::OptionalFieldStyle {
        crate::lang::config::OptionalFieldStyle::Ignored
    }

    /// How `PropertySpec` renders: accessor methods or inline field body.
    ///
    /// Default: `Accessor`.
    #[deprecated(
        note = "legacy 0.6.8 property grammar; implement CodeLang::lower_property instead"
    )]
    #[expect(deprecated, reason = "0.6.8 compatibility default")]
    fn property_style(&self) -> crate::spec::modifiers::PropertyStyle {
        crate::spec::modifiers::PropertyStyle::Accessor
    }

    /// The keyword for a property getter in field-style rendering.
    ///
    /// Default: `"get"`.
    #[deprecated(
        note = "legacy 0.6.8 property grammar; implement CodeLang::lower_property instead"
    )]
    fn property_getter_keyword(&self) -> &str {
        "get"
    }

    /// Render a type context from complete type parameters.
    ///
    /// This exact 0.6.8 string hook is retained for compatibility. Current
    /// lowering uses [`CodeLang::emit_type_context`].
    #[deprecated(note = "legacy 0.6.8 generic grammar; implement CodeLang::lower_function instead")]
    fn render_type_context(&self, _type_params: &[TypeParamSpec]) -> String {
        String::new()
    }

    /// Emit a type context / constraint prefix for split function signatures.
    ///
    /// Default: no context.
    fn emit_type_context(
        &self,
        type_params: &[TypeParamSpec],
    ) -> Result<Option<CodeBlock>, SigilStitchError> {
        let mut markers = LegacyTypeMarkers::new("CodeLang::render_type_context");
        let marked = markers.mark_type_params(type_params);
        #[expect(deprecated, reason = "0.6.8 compatibility bridge")]
        let output = self.render_type_context(&marked);
        if output.is_empty() && type_params.is_empty() {
            return Ok(None);
        }
        let block = markers.recover(&output)?;
        if block.is_empty() {
            Ok(None)
        } else {
            Ok(Some(block))
        }
    }

    /// Content emitted after `block_open` but before the first field in a type body.
    ///
    /// Default: `""`.
    #[deprecated(note = "legacy 0.6.8 type grammar; implement CodeLang::lower_type instead")]
    fn type_body_prefix(&self, _name: &str, _kind: crate::spec::modifiers::TypeKind) -> String {
        String::new()
    }

    /// Content emitted after the last field but before `block_close` in a type body.
    ///
    /// Default: `""`.
    #[deprecated(note = "legacy 0.6.8 type grammar; implement CodeLang::lower_type instead")]
    fn type_body_suffix(&self, _name: &str, _kind: crate::spec::modifiers::TypeKind) -> String {
        String::new()
    }

    /// Render a suffix after a type's closing delimiter.
    ///
    /// This exact 0.6.8 string hook is retained for compatibility. Current
    /// lowering uses [`CodeLang::emit_type_close_suffix`].
    #[deprecated(note = "legacy 0.6.8 type grammar; implement CodeLang::lower_type instead")]
    fn render_type_close_suffix(&self, _kind: TypeKind, _impl_types: &[String]) -> String {
        String::new()
    }

    /// Emit a suffix after the type's closing delimiter (e.g., Haskell `deriving`).
    ///
    /// Default: no suffix.
    #[deprecated(note = "legacy 0.6.8 type grammar; implement CodeLang::lower_type instead")]
    fn emit_type_close_suffix(
        &self,
        kind: TypeKind,
        impl_types: &[TypeName],
    ) -> Result<Option<CodeBlock>, SigilStitchError> {
        let mut markers = LegacyTypeMarkers::new("CodeLang::render_type_close_suffix");
        let marked: Vec<String> = impl_types
            .iter()
            .map(|type_name| markers.mark(type_name))
            .collect();
        #[expect(deprecated, reason = "0.6.8 compatibility bridge")]
        let output = self.render_type_close_suffix(kind, &marked);
        if output.is_empty() && impl_types.is_empty() {
            return Ok(None);
        }
        let block = markers.recover(&output)?;
        if block.is_empty() {
            Ok(None)
        } else {
            Ok(Some(block))
        }
    }

    /// Render a type parameter's kind annotation (for higher-kinded types).
    ///
    /// Default: empty string.
    #[deprecated(
        note = "legacy 0.6.8 generic grammar; implement complete language-owned lowering instead"
    )]
    fn render_type_param_kind(&self, _kind: &crate::spec::where_spec::TypeParamKind) -> String {
        String::new()
    }

    // ── Config struct accessors (spec-only) ───────────────────────────

    /// Legacy function-declaration syntax used by the 0.6.8 compatibility lowerer.
    #[deprecated(
        note = "legacy 0.6.8 declaration grammar; implement CodeLang::lower_function instead"
    )]
    #[expect(deprecated, reason = "0.6.8 compatibility default")]
    fn function_syntax(&self) -> config::FunctionSyntaxConfig<'_> {
        config::FunctionSyntaxConfig::default()
    }

    /// Legacy type-declaration syntax retained for 0.6.8 adapter compatibility.
    #[deprecated(
        note = "legacy 0.6.8 declaration grammar; migrate declarations to language-owned lowering"
    )]
    #[expect(deprecated, reason = "0.6.8 compatibility default")]
    fn type_decl_syntax(&self) -> config::TypeDeclSyntaxConfig<'_> {
        config::TypeDeclSyntaxConfig::default()
    }

    /// Legacy enum, annotation, and field declaration syntax retained for compatibility.
    #[deprecated(
        note = "legacy 0.6.8 declaration grammar; migrate declarations to language-owned lowering"
    )]
    #[expect(deprecated, reason = "0.6.8 compatibility default")]
    fn enum_and_annotation(&self) -> config::EnumAndAnnotationConfig<'_> {
        config::EnumAndAnnotationConfig::default()
    }
}

/// Derive a PascalCase namespace alias from a module path.
///
/// Used for wildcard imports that need a namespace name
/// (e.g., `import * as Models from "./models"`).
pub(crate) fn module_to_alias(module: &str) -> String {
    let last_segment = module
        .rsplit(['/', ':', '.', '\\'])
        .find(|s| !s.is_empty())
        .unwrap_or(module);

    let mut chars = last_segment.chars();
    match chars.next() {
        None => "Module".to_string(),
        Some(first) => {
            let upper: String = first.to_uppercase().collect();
            format!("{upper}{}", chars.as_str())
        }
    }
}

/// Create a default `CodeLang` implementation from a file extension.
///
/// Returns `None` if the extension is not recognized.
pub fn lang_from_extension(ext: &str) -> Option<Box<dyn CodeLang>> {
    match ext {
        "ts" | "tsx" => Some(Box::new(typescript::TypeScript::default())),
        "js" | "jsx" | "mjs" | "cjs" => Some(Box::new(javascript::JavaScript::default())),
        "rs" => Some(Box::new(rust::Rust::default())),
        "go" => Some(Box::new(go::Go::default())),
        "py" | "pyi" => Some(Box::new(python::Python::default())),
        "java" => Some(Box::new(java::Java::default())),
        "kt" | "kts" => Some(Box::new(kotlin::Kotlin::default())),
        "swift" => Some(Box::new(swift::Swift::default())),
        "dart" => Some(Box::new(dart::Dart::default())),
        "scala" | "sc" => Some(Box::new(scala::Scala::default())),
        "hs" => Some(Box::new(haskell::Haskell::default())),
        "ml" | "mli" => Some(Box::new(ocaml::OCaml::default())),
        "c" | "h" => Some(Box::new(c::C::default())),
        "cpp" | "cxx" | "cc" | "hpp" | "hxx" => Some(Box::new(cpp::Cpp::default())),
        "cs" => Some(Box::new(csharp::CSharp::default())),
        "lua" => Some(Box::new(lua::Lua::default())),
        "sh" | "bash" => Some(Box::new(bash::Bash::default())),
        "zsh" => Some(Box::new(zsh::Zsh::default())),
        "php" => Some(Box::new(php::Php::default())),
        "rb" => Some(Box::new(ruby::Ruby::default())),
        _ => None,
    }
}
