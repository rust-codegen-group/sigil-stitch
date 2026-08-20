//! Language capability declarations for spec emission.
//!
//! A language adapter declares **what** spec constructs it supports. This
//! module deliberately contains no rendering policy, keywords, delimiters, or
//! fallback syntax.

use crate::spec::modifiers::TypeKind;

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
    /// Optional record fields.
    OptionalRecordFields,
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
}

impl<'a> LanguageCapabilities<'a> {
    /// Start a strict matrix. Omitted profiles are unsupported.
    pub const fn strict() -> Self {
        Self {
            types: CapabilityProfiles::Strict(&[]),
            functions: CapabilityProfiles::Strict(&[]),
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

    /// Compatibility profile for adapters that predate capability validation.
    pub const fn permissive() -> Self {
        Self {
            types: CapabilityProfiles::Permissive,
            functions: CapabilityProfiles::Permissive,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permissive_matrix_accepts_type_and_function_capabilities() {
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
        let capabilities = LanguageCapabilities::strict()
            .with_types(TYPES)
            .with_functions(FUNCTIONS);

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
    }
}
