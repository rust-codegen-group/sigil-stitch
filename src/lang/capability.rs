//! Language capability declarations for spec emission.
//!
//! A language adapter declares **what** spec constructs it supports. This
//! module deliberately contains no rendering policy, keywords, delimiters, or
//! fallback syntax.

use crate::spec::modifiers::{DeclarationContext, TypeKind};

/// A semantic capability of a type declaration.
///
/// # Naming invariant
///
/// Each variant represents exactly one type-system concept. Do not add
/// language-specific aliases such as `Enum`, `Trait`, `Extends`, `Implements`,
/// or `GetSet`; those names belong in the language-local rendering module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TypeCapability {
    /// Labelled members of a record type.
    RecordFields,
    /// Accessor members over record data.
    AccessorMethods,
    /// Function-valued members.
    Methods,
    /// Structural embedding of one declaration's members into another.
    StructuralEmbedding,
    /// Nominal inheritance / subtyping between declarations.
    NominalSubtyping,
    /// Implementation of a contract / interface / type class.
    InterfaceImplementation,
    /// Universally quantified type parameters.
    ParametricPolymorphism,
    /// Bounded or constrained type parameters.
    BoundedPolymorphism,
    /// Parameters introduced directly by a declaration constructor.
    ConstructorParameters,
    /// Sum-type variants.
    Variants,
    /// Declaration metadata / attributes.
    Attributes,
}

/// The semantic context in which a complete field sequence is emitted.
///
/// A context identifies the owning grammar without exposing separators,
/// first/last flags, or other target syntax to callers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FieldContext {
    /// A directly emitted field sequence with only legacy declaration placement.
    Direct(DeclarationContext),
    /// Fields owned by a complete type declaration.
    TypeMember(TypeKind),
    /// Named fields carried by one algebraic variant record payload.
    VariantRecordPayload(TypeKind),
}

/// A semantic capability of a field declaration.
///
/// These variants describe caller intent, never target spelling, placement,
/// delimiters, separators, or ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FieldCapability {
    /// An explicit type annotation.
    ExplicitType,
    /// A value initializer.
    Initializer,
    /// Structured or opaque metadata, including the legacy Go tag escape hatch.
    Attributes,
    /// A type-side or static field.
    StaticField,
    /// A field whose declaration promises read-only binding semantics.
    ReadOnly,
    /// A field whose key may be absent from its containing value.
    ///
    /// This is distinct from [`TypeName::Optional`](crate::type_name::TypeName::Optional),
    /// which allows a present field to carry a nullable or option-like value.
    OptionalPresence,
}

/// The semantic context in which one computed property is emitted.
///
/// The context identifies the owning declaration without exposing target
/// placement, accessor spelling, or preamble order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PropertyContext {
    /// A directly emitted property with only legacy declaration placement.
    Direct(DeclarationContext),
    /// A property owned by a complete type declaration.
    TypeMember(TypeKind),
}

/// A semantic capability of a computed property declaration.
///
/// These variants describe caller intent. They do not prescribe whether a
/// language uses methods, a field-style computed property, or another target
/// construct to preserve that intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PropertyCapability {
    /// An explicit property value type.
    ExplicitType,
    /// Read access implemented by the supplied getter body.
    ReadAccessor,
    /// Write access implemented by the supplied setter body.
    WriteAccessor,
    /// Structured or opaque declaration metadata.
    Attributes,
    /// A type-side or static property.
    StaticProperty,
}

/// Capability profile for one computed-property context.
#[derive(Debug, Clone, Copy)]
pub struct PropertyCapabilityProfile<'a> {
    context: PropertyContext,
    capabilities: &'a [PropertyCapability],
    required_capabilities: &'a [PropertyCapability],
}

impl<'a> PropertyCapabilityProfile<'a> {
    /// Create a property profile for one semantic context.
    pub const fn new(context: PropertyContext, capabilities: &'a [PropertyCapability]) -> Self {
        Self {
            context,
            capabilities,
            required_capabilities: &[],
        }
    }

    /// Declare capabilities every property in this context must provide.
    pub const fn with_required_capabilities(
        mut self,
        capabilities: &'a [PropertyCapability],
    ) -> Self {
        self.required_capabilities = capabilities;
        self
    }

    /// The semantic property context this profile describes.
    pub const fn context(self) -> PropertyContext {
        self.context
    }

    /// Whether this profile supports the requested semantic capability.
    pub fn supports(self, capability: PropertyCapability) -> bool {
        self.capabilities.contains(&capability)
    }

    /// Capabilities every property in this context must provide.
    pub fn required_capabilities(self) -> &'a [PropertyCapability] {
        self.required_capabilities
    }
}

/// Capability profile for one field-sequence context.
#[derive(Debug, Clone, Copy)]
pub struct FieldCapabilityProfile<'a> {
    context: FieldContext,
    capabilities: &'a [FieldCapability],
    required_capabilities: &'a [FieldCapability],
}

impl<'a> FieldCapabilityProfile<'a> {
    /// Create a field profile for one semantic context.
    pub const fn new(context: FieldContext, capabilities: &'a [FieldCapability]) -> Self {
        Self {
            context,
            capabilities,
            required_capabilities: &[],
        }
    }

    /// Declare capabilities that every field in this profile must provide.
    pub const fn with_required_capabilities(mut self, capabilities: &'a [FieldCapability]) -> Self {
        self.required_capabilities = capabilities;
        self
    }

    /// The semantic field context this profile describes.
    pub const fn context(self) -> FieldContext {
        self.context
    }

    /// Whether this profile supports the requested semantic capability.
    pub fn supports(self, capability: FieldCapability) -> bool {
        self.capabilities.contains(&capability)
    }

    /// Capabilities every field in this profile must provide.
    pub fn required_capabilities(self) -> &'a [FieldCapability] {
        self.required_capabilities
    }
}

/// A semantic capability of an enum-variant sequence.
///
/// # Naming invariant
///
/// These variants describe caller intent, never target spelling, placement,
/// delimiters, separators, or ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum VariantCapability {
    /// An explicit value identifying an enum member.
    Discriminant,
    /// Expressions passed to an enum entry's constructor.
    ConstructorArguments,
    /// Types carried positionally by a sum-type constructor or enum case.
    PositionalPayload,
    /// Named typed fields carried by a sum-type constructor or enum case.
    RecordPayload,
    /// Declaration metadata / annotations attached to a variant.
    Attributes,
}

/// Capability profile for variants owned by one [`TypeKind`].
#[derive(Debug, Clone, Copy)]
pub struct VariantCapabilityProfile<'a> {
    owner_kind: TypeKind,
    capabilities: &'a [VariantCapability],
}

impl<'a> VariantCapabilityProfile<'a> {
    /// Create a variant profile for one owning type kind.
    pub const fn new(owner_kind: TypeKind, capabilities: &'a [VariantCapability]) -> Self {
        Self {
            owner_kind,
            capabilities,
        }
    }

    /// The type kind that owns this variant sequence.
    pub const fn owner_kind(self) -> TypeKind {
        self.owner_kind
    }

    /// Whether this owner supports the requested semantic capability.
    pub fn supports(self, capability: VariantCapability) -> bool {
        self.capabilities.contains(&capability)
    }
}

/// Capability profile for one [`TypeKind`].
#[derive(Debug, Clone, Copy)]
pub struct TypeCapabilityProfile<'a> {
    kind: TypeKind,
    capabilities: &'a [TypeCapability],
}

impl<'a> TypeCapabilityProfile<'a> {
    /// Create a capability profile for one type kind.
    pub const fn new(kind: TypeKind, capabilities: &'a [TypeCapability]) -> Self {
        Self { kind, capabilities }
    }

    /// The type kind this profile describes.
    pub const fn kind(self) -> TypeKind {
        self.kind
    }

    /// Whether this kind supports the given semantic capability.
    pub fn supports(self, capability: TypeCapability) -> bool {
        self.capabilities.contains(&capability)
    }
}

/// The semantic context in which a function declaration is emitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FunctionContext {
    /// A free function at file scope.
    TopLevel,
    /// A receiver method emitted at file scope, such as a Go method.
    ReceiverMethod,
    /// A concrete member declared in or for a type.
    Member,
    /// A member declared in an interface or trait contract.
    InterfaceMember,
}

/// The declaration form emitted in a [`FunctionContext`].
///
/// Keeping form separate from context lets profiles distinguish ordinary
/// functions, constructors, and destructors without multiplying capability
/// variants for form-specific spellings of the same semantic feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FunctionForm {
    /// An ordinary function or method declaration.
    Function,
    /// An object initializer or constructor-like associated function.
    Constructor,
    /// An object finalizer or destructor.
    ///
    /// C++ adapters classify the established `~Type` naming convention as
    /// this form so destructor declarations are not forced to carry a return
    /// type.
    Destructor,
}

/// Whether declarations in a function profile may carry an implementation
/// body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FunctionBodyPolicy {
    /// A body may be present or absent.
    Optional,
    /// A concrete declaration must have a body. Abstract declarations remain
    /// bodyless.
    Required,
    /// A body is not valid in this declaration context.
    Forbidden,
}

/// A semantic capability of a function or method declaration.
///
/// # Naming invariant
///
/// Each variant represents exactly one type-system or declaration concept.
/// Language-specific spellings such as `suspend`, `virtual`, or `vararg`
/// remain in the language-local rendering module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FunctionCapability {
    /// Universally quantified declaration type parameters.
    ParametricPolymorphism,
    /// Bounded or constrained declaration type parameters.
    BoundedPolymorphism,
    /// Declaration metadata / annotations.
    Attributes,
    /// An explicitly declared return type.
    ExplicitReturnType,
    /// Type annotations on function parameters.
    TypedParameters,
    /// Asynchronous effect.
    AsyncEffect,
    /// File-local / internal-linkage free functions.
    StaticFunction,
    /// Class-side / static methods.
    StaticMethod,
    /// Type initialization through a static constructor.
    StaticConstructor,
    /// Methods that participate in virtual dispatch without requiring deferral.
    VirtualMethod,
    /// Deferred methods without an implementation body.
    AbstractMethod,
    /// A declaration that refines an inherited declaration.
    Override,
    /// Constructor delegation to another constructor or superclass.
    ConstructorDelegation,
    /// Parameters with default values.
    DefaultParameters,
    /// Variadic parameters.
    VariadicParameters,
    /// Constructor parameters promoted to type properties.
    ConstructorProperties,
}

/// Capability profile for one context and declaration form.
#[derive(Debug, Clone, Copy)]
pub struct FunctionCapabilityProfile<'a> {
    context: FunctionContext,
    form: FunctionForm,
    capabilities: &'a [FunctionCapability],
    required_capabilities: &'a [FunctionCapability],
    incompatible_capabilities: &'a [(FunctionCapability, FunctionCapability)],
    body_policy: FunctionBodyPolicy,
    maximum_parameters: Option<usize>,
}

impl<'a> FunctionCapabilityProfile<'a> {
    /// Create a capability profile for one context and declaration form.
    pub const fn new(
        context: FunctionContext,
        form: FunctionForm,
        capabilities: &'a [FunctionCapability],
    ) -> Self {
        Self {
            context,
            form,
            capabilities,
            required_capabilities: &[],
            incompatible_capabilities: &[],
            body_policy: FunctionBodyPolicy::Optional,
            maximum_parameters: None,
        }
    }

    /// Declare capabilities that every declaration in this profile must
    /// provide.
    pub const fn with_required_capabilities(
        mut self,
        capabilities: &'a [FunctionCapability],
    ) -> Self {
        self.required_capabilities = capabilities;
        self
    }

    /// Declare capability pairs that cannot be combined in this profile.
    pub const fn with_incompatible_capabilities(
        mut self,
        combinations: &'a [(FunctionCapability, FunctionCapability)],
    ) -> Self {
        self.incompatible_capabilities = combinations;
        self
    }

    /// Declare whether implementation bodies are optional, required, or
    /// forbidden in this profile.
    pub const fn with_body_policy(mut self, policy: FunctionBodyPolicy) -> Self {
        self.body_policy = policy;
        self
    }

    /// Limit the number of parameters accepted by declarations in this
    /// profile.
    pub const fn with_maximum_parameters(mut self, maximum: usize) -> Self {
        self.maximum_parameters = Some(maximum);
        self
    }

    /// The function context this profile describes.
    pub const fn context(self) -> FunctionContext {
        self.context
    }

    /// The declaration form this profile describes.
    pub const fn form(self) -> FunctionForm {
        self.form
    }

    /// Whether this profile supports the given semantic capability.
    pub fn supports(self, capability: FunctionCapability) -> bool {
        self.capabilities.contains(&capability)
    }

    /// Capabilities every declaration in this profile must provide.
    pub fn required_capabilities(self) -> &'a [FunctionCapability] {
        self.required_capabilities
    }

    /// Whether declarations in this profile may carry implementation bodies.
    pub const fn body_policy(self) -> FunctionBodyPolicy {
        self.body_policy
    }

    /// Capability pairs that cannot be combined in this profile.
    pub fn incompatible_capabilities(self) -> &'a [(FunctionCapability, FunctionCapability)] {
        self.incompatible_capabilities
    }

    /// Maximum number of parameters accepted by this profile, when bounded.
    pub const fn maximum_parameters(self) -> Option<usize> {
        self.maximum_parameters
    }

    fn first_incompatible_pair(
        self,
        requested: &[FunctionCapability],
    ) -> Option<(FunctionCapability, FunctionCapability)> {
        self.incompatible_capabilities
            .iter()
            .copied()
            .find(|(first, second)| requested.contains(first) && requested.contains(second))
    }
}

#[derive(Debug, Clone, Copy)]
enum CapabilityProfiles<'a, T> {
    Permissive,
    Strict(&'a [T]),
}

/// Complete spec capability matrix for a language.
///
/// Built-in languages construct a fail-closed matrix with [`Self::strict`].
/// The [`CodeLang`](crate::lang::CodeLang) default uses [`Self::permissive`]
/// so adapters written for sigil-stitch 0.6.8 continue to compile and retain
/// their previous behavior.
#[derive(Debug, Clone, Copy)]
pub struct LanguageCapabilities<'a> {
    types: CapabilityProfiles<'a, TypeCapabilityProfile<'a>>,
    functions: CapabilityProfiles<'a, FunctionCapabilityProfile<'a>>,
    variants: CapabilityProfiles<'a, VariantCapabilityProfile<'a>>,
    fields: CapabilityProfiles<'a, FieldCapabilityProfile<'a>>,
    properties: CapabilityProfiles<'a, PropertyCapabilityProfile<'a>>,
}

impl<'a> LanguageCapabilities<'a> {
    /// Start a strict matrix. Omitted profiles are unsupported.
    pub const fn strict() -> Self {
        Self {
            types: CapabilityProfiles::Strict(&[]),
            functions: CapabilityProfiles::Strict(&[]),
            variants: CapabilityProfiles::Strict(&[]),
            fields: CapabilityProfiles::Strict(&[]),
            properties: CapabilityProfiles::Strict(&[]),
        }
    }

    /// Add strict type-declaration profiles.
    pub const fn with_types(mut self, profiles: &'a [TypeCapabilityProfile<'a>]) -> Self {
        self.types = CapabilityProfiles::Strict(profiles);
        self
    }

    /// Add strict function-declaration profiles.
    pub const fn with_functions(mut self, profiles: &'a [FunctionCapabilityProfile<'a>]) -> Self {
        self.functions = CapabilityProfiles::Strict(profiles);
        self
    }

    /// Add strict owner-aware enum-variant profiles.
    pub const fn with_variants(mut self, profiles: &'a [VariantCapabilityProfile<'a>]) -> Self {
        self.variants = CapabilityProfiles::Strict(profiles);
        self
    }

    /// Add strict field-sequence profiles.
    pub const fn with_fields(mut self, profiles: &'a [FieldCapabilityProfile<'a>]) -> Self {
        self.fields = CapabilityProfiles::Strict(profiles);
        self
    }

    /// Add strict computed-property profiles.
    pub const fn with_properties(mut self, profiles: &'a [PropertyCapabilityProfile<'a>]) -> Self {
        self.properties = CapabilityProfiles::Strict(profiles);
        self
    }

    /// Compatibility profile for adapters that predate capability validation.
    pub const fn permissive() -> Self {
        Self {
            types: CapabilityProfiles::Permissive,
            functions: CapabilityProfiles::Permissive,
            variants: CapabilityProfiles::Permissive,
            fields: CapabilityProfiles::Permissive,
            properties: CapabilityProfiles::Permissive,
        }
    }

    /// Whether any profile declares this type kind.
    pub fn supports_type_kind(&self, kind: TypeKind) -> bool {
        match self.types {
            CapabilityProfiles::Permissive => true,
            CapabilityProfiles::Strict(profiles) => {
                profiles.iter().any(|profile| profile.kind() == kind)
            }
        }
    }

    /// Whether the profile for `kind` declares `capability`.
    pub fn supports_type_capability(&self, kind: TypeKind, capability: TypeCapability) -> bool {
        match self.types {
            CapabilityProfiles::Permissive => true,
            CapabilityProfiles::Strict(profiles) => profiles
                .iter()
                .find(|profile| profile.kind() == kind)
                .is_some_and(|profile| profile.supports(capability)),
        }
    }

    /// Whether any profile declares this function context.
    pub fn supports_function_context(&self, context: FunctionContext) -> bool {
        match self.functions {
            CapabilityProfiles::Permissive => true,
            CapabilityProfiles::Strict(profiles) => {
                profiles.iter().any(|profile| profile.context() == context)
            }
        }
    }

    /// Whether a profile declares this context and declaration form.
    pub fn supports_function_form(&self, context: FunctionContext, form: FunctionForm) -> bool {
        match self.functions {
            CapabilityProfiles::Permissive => true,
            CapabilityProfiles::Strict(profiles) => profiles
                .iter()
                .any(|profile| profile.context() == context && profile.form() == form),
        }
    }

    /// Whether the profile for `context` and `form` declares `capability`.
    pub fn supports_function_capability(
        &self,
        context: FunctionContext,
        form: FunctionForm,
        capability: FunctionCapability,
    ) -> bool {
        match self.functions {
            CapabilityProfiles::Permissive => true,
            CapabilityProfiles::Strict(profiles) => profiles
                .iter()
                .find(|profile| profile.context() == context && profile.form() == form)
                .is_some_and(|profile| profile.supports(capability)),
        }
    }

    pub(crate) fn first_incompatible_function_capabilities(
        &self,
        context: FunctionContext,
        form: FunctionForm,
        requested: &[FunctionCapability],
    ) -> Option<(FunctionCapability, FunctionCapability)> {
        match self.functions {
            CapabilityProfiles::Permissive => None,
            CapabilityProfiles::Strict(profiles) => profiles
                .iter()
                .find(|profile| profile.context() == context && profile.form() == form)
                .and_then(|profile| profile.first_incompatible_pair(requested)),
        }
    }

    /// Required capabilities for this context and declaration form.
    pub fn required_function_capabilities(
        &self,
        context: FunctionContext,
        form: FunctionForm,
    ) -> &[FunctionCapability] {
        match self.functions {
            CapabilityProfiles::Permissive => &[],
            CapabilityProfiles::Strict(profiles) => profiles
                .iter()
                .find(|profile| profile.context() == context && profile.form() == form)
                .map_or(&[], |profile| profile.required_capabilities()),
        }
    }

    /// Body policy for this context and declaration form.
    pub fn function_body_policy(
        &self,
        context: FunctionContext,
        form: FunctionForm,
    ) -> FunctionBodyPolicy {
        match self.functions {
            CapabilityProfiles::Permissive => FunctionBodyPolicy::Optional,
            CapabilityProfiles::Strict(profiles) => profiles
                .iter()
                .find(|profile| profile.context() == context && profile.form() == form)
                .map_or(FunctionBodyPolicy::Optional, |profile| {
                    profile.body_policy()
                }),
        }
    }

    /// Incompatible capability pairs for this context and declaration form.
    pub fn incompatible_function_capabilities(
        &self,
        context: FunctionContext,
        form: FunctionForm,
    ) -> &[(FunctionCapability, FunctionCapability)] {
        match self.functions {
            CapabilityProfiles::Permissive => &[],
            CapabilityProfiles::Strict(profiles) => profiles
                .iter()
                .find(|profile| profile.context() == context && profile.form() == form)
                .map_or(&[], |profile| profile.incompatible_capabilities()),
        }
    }

    /// Maximum parameter count for this context and declaration form.
    pub fn maximum_function_parameters(
        &self,
        context: FunctionContext,
        form: FunctionForm,
    ) -> Option<usize> {
        match self.functions {
            CapabilityProfiles::Permissive => None,
            CapabilityProfiles::Strict(profiles) => profiles
                .iter()
                .find(|profile| profile.context() == context && profile.form() == form)
                .and_then(|profile| profile.maximum_parameters()),
        }
    }

    pub(crate) fn function_validation_is_permissive(&self) -> bool {
        matches!(self.functions, CapabilityProfiles::Permissive)
    }

    /// Whether this language declares a variant profile for `owner_kind`.
    pub fn supports_variant_owner(&self, owner_kind: TypeKind) -> bool {
        match self.variants {
            CapabilityProfiles::Permissive => true,
            CapabilityProfiles::Strict(profiles) => profiles
                .iter()
                .any(|profile| profile.owner_kind() == owner_kind),
        }
    }

    /// Whether variants owned by `owner_kind` support `capability`.
    pub fn supports_variant_capability(
        &self,
        owner_kind: TypeKind,
        capability: VariantCapability,
    ) -> bool {
        match self.variants {
            CapabilityProfiles::Permissive => true,
            CapabilityProfiles::Strict(profiles) => profiles
                .iter()
                .find(|profile| profile.owner_kind() == owner_kind)
                .is_some_and(|profile| profile.supports(capability)),
        }
    }

    pub(crate) fn variant_validation_is_permissive(&self) -> bool {
        matches!(self.variants, CapabilityProfiles::Permissive)
    }

    /// Whether this language declares a profile for `context`.
    pub fn supports_field_context(&self, context: FieldContext) -> bool {
        match self.fields {
            CapabilityProfiles::Permissive => true,
            CapabilityProfiles::Strict(profiles) => {
                profiles.iter().any(|profile| profile.context() == context)
            }
        }
    }

    /// Whether fields in `context` support `capability`.
    pub fn supports_field_capability(
        &self,
        context: FieldContext,
        capability: FieldCapability,
    ) -> bool {
        match self.fields {
            CapabilityProfiles::Permissive => true,
            CapabilityProfiles::Strict(profiles) => profiles
                .iter()
                .find(|profile| profile.context() == context)
                .is_some_and(|profile| profile.supports(capability)),
        }
    }

    /// Required capabilities for every field in `context`.
    pub fn required_field_capabilities(&self, context: FieldContext) -> &[FieldCapability] {
        match self.fields {
            CapabilityProfiles::Permissive => &[],
            CapabilityProfiles::Strict(profiles) => profiles
                .iter()
                .find(|profile| profile.context() == context)
                .map_or(&[], |profile| profile.required_capabilities()),
        }
    }

    /// Whether this language declares a property profile for `context`.
    pub fn supports_property_context(&self, context: PropertyContext) -> bool {
        match self.properties {
            CapabilityProfiles::Permissive => true,
            CapabilityProfiles::Strict(profiles) => {
                profiles.iter().any(|profile| profile.context() == context)
            }
        }
    }

    /// Whether properties in `context` support `capability`.
    pub fn supports_property_capability(
        &self,
        context: PropertyContext,
        capability: PropertyCapability,
    ) -> bool {
        match self.properties {
            CapabilityProfiles::Permissive => true,
            CapabilityProfiles::Strict(profiles) => profiles
                .iter()
                .find(|profile| profile.context() == context)
                .is_some_and(|profile| profile.supports(capability)),
        }
    }

    /// Required capabilities for every property in `context`.
    pub fn required_property_capabilities(
        &self,
        context: PropertyContext,
    ) -> &[PropertyCapability] {
        match self.properties {
            CapabilityProfiles::Permissive => &[],
            CapabilityProfiles::Strict(profiles) => profiles
                .iter()
                .find(|profile| profile.context() == context)
                .map_or(&[], |profile| profile.required_capabilities()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permissive_matrix_accepts_all_capability_families() {
        let capabilities = LanguageCapabilities::permissive();
        assert!(capabilities.supports_type_kind(TypeKind::Enum));
        assert!(capabilities.supports_type_capability(TypeKind::Enum, TypeCapability::Variants));
        assert!(capabilities.supports_function_context(FunctionContext::ReceiverMethod));
        assert!(capabilities.supports_function_capability(
            FunctionContext::ReceiverMethod,
            FunctionForm::Function,
            FunctionCapability::AsyncEffect
        ));
        assert!(capabilities.function_validation_is_permissive());
        assert!(capabilities.supports_variant_owner(TypeKind::Enum));
        assert!(
            capabilities
                .supports_variant_capability(TypeKind::Enum, VariantCapability::RecordPayload)
        );
        assert!(capabilities.variant_validation_is_permissive());
        assert!(
            capabilities.supports_property_context(PropertyContext::TypeMember(TypeKind::Class))
        );
        assert!(capabilities.supports_property_capability(
            PropertyContext::TypeMember(TypeKind::Class),
            PropertyCapability::ReadAccessor
        ));
    }

    #[test]
    fn strict_matrix_checks_profiles_and_capabilities() {
        const TYPES: &[TypeCapabilityProfile] = &[TypeCapabilityProfile::new(
            TypeKind::Struct,
            &[TypeCapability::RecordFields],
        )];
        const INCOMPATIBLE: &[(FunctionCapability, FunctionCapability)] = &[(
            FunctionCapability::AsyncEffect,
            FunctionCapability::StaticFunction,
        )];
        const FUNCTIONS: &[FunctionCapabilityProfile] = &[
            FunctionCapabilityProfile::new(
                FunctionContext::TopLevel,
                FunctionForm::Function,
                &[
                    FunctionCapability::AsyncEffect,
                    FunctionCapability::StaticFunction,
                ],
            )
            .with_incompatible_capabilities(INCOMPATIBLE),
            FunctionCapabilityProfile::new(
                FunctionContext::TopLevel,
                FunctionForm::Constructor,
                &[FunctionCapability::AsyncEffect],
            ),
        ];
        const VARIANTS: &[VariantCapabilityProfile] = &[VariantCapabilityProfile::new(
            TypeKind::Enum,
            &[
                VariantCapability::Discriminant,
                VariantCapability::PositionalPayload,
            ],
        )];
        const PROPERTIES: &[PropertyCapabilityProfile] = &[PropertyCapabilityProfile::new(
            PropertyContext::TypeMember(TypeKind::Struct),
            &[
                PropertyCapability::ExplicitType,
                PropertyCapability::ReadAccessor,
            ],
        )
        .with_required_capabilities(&[PropertyCapability::ReadAccessor])];
        let capabilities = LanguageCapabilities::strict()
            .with_types(TYPES)
            .with_functions(FUNCTIONS)
            .with_variants(VARIANTS)
            .with_properties(PROPERTIES);

        assert!(capabilities.supports_type_kind(TypeKind::Struct));
        assert!(!capabilities.supports_type_kind(TypeKind::Enum));
        assert!(
            capabilities.supports_type_capability(TypeKind::Struct, TypeCapability::RecordFields)
        );
        assert!(!capabilities.supports_type_capability(TypeKind::Struct, TypeCapability::Methods));
        assert!(capabilities.supports_function_context(FunctionContext::TopLevel));
        assert!(!capabilities.supports_function_context(FunctionContext::Member));
        assert!(
            capabilities.supports_function_form(FunctionContext::TopLevel, FunctionForm::Function)
        );
        assert!(
            capabilities
                .supports_function_form(FunctionContext::TopLevel, FunctionForm::Constructor)
        );
        assert!(capabilities.supports_function_capability(
            FunctionContext::TopLevel,
            FunctionForm::Function,
            FunctionCapability::AsyncEffect
        ));
        assert!(capabilities.supports_function_capability(
            FunctionContext::TopLevel,
            FunctionForm::Function,
            FunctionCapability::StaticFunction
        ));
        assert_eq!(
            capabilities.first_incompatible_function_capabilities(
                FunctionContext::TopLevel,
                FunctionForm::Function,
                &[
                    FunctionCapability::StaticFunction,
                    FunctionCapability::AsyncEffect,
                ],
            ),
            Some((
                FunctionCapability::AsyncEffect,
                FunctionCapability::StaticFunction,
            ))
        );
        assert!(!capabilities.function_validation_is_permissive());
        assert!(capabilities.supports_variant_owner(TypeKind::Enum));
        assert!(!capabilities.supports_variant_owner(TypeKind::Struct));
        assert!(
            capabilities
                .supports_variant_capability(TypeKind::Enum, VariantCapability::Discriminant)
        );
        assert!(
            !capabilities
                .supports_variant_capability(TypeKind::Enum, VariantCapability::RecordPayload)
        );
        assert!(!capabilities.variant_validation_is_permissive());
        let property_context = PropertyContext::TypeMember(TypeKind::Struct);
        assert!(capabilities.supports_property_context(property_context));
        assert!(
            capabilities
                .supports_property_capability(property_context, PropertyCapability::ExplicitType)
        );
        assert!(
            !capabilities
                .supports_property_capability(property_context, PropertyCapability::StaticProperty)
        );
        assert_eq!(
            capabilities.required_property_capabilities(property_context),
            &[PropertyCapability::ReadAccessor]
        );
    }
}
