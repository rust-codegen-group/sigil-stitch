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
pub enum SpecCapability {
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

impl SpecCapability {
    /// Stable diagnostic name for this capability.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RecordFields => "RecordFields",
            Self::AccessorMethods => "AccessorMethods",
            Self::Methods => "Methods",
            Self::StructuralEmbedding => "StructuralEmbedding",
            Self::NominalSubtyping => "NominalSubtyping",
            Self::InterfaceImplementation => "InterfaceImplementation",
            Self::ParametricPolymorphism => "ParametricPolymorphism",
            Self::BoundedPolymorphism => "BoundedPolymorphism",
            Self::ConstructorParameters => "ConstructorParameters",
            Self::Variants => "Variants",
            Self::Attributes => "Attributes",
            Self::OptionalRecordFields => "OptionalRecordFields",
        }
    }
}

/// Capability profile for one [`TypeKind`].
#[derive(Debug, Clone, Copy)]
pub struct TypeCapabilities<'a> {
    kind: TypeKind,
    features: &'a [SpecCapability],
}

impl<'a> TypeCapabilities<'a> {
    /// Create a capability profile for one type kind.
    pub const fn new(kind: TypeKind, features: &'a [SpecCapability]) -> Self {
        Self { kind, features }
    }

    /// The type kind this profile describes.
    pub const fn kind(self) -> TypeKind {
        self.kind
    }

    /// Whether this kind supports the given semantic capability.
    pub fn supports(self, capability: SpecCapability) -> bool {
        self.features.contains(&capability)
    }
}

/// Complete spec capability matrix for a language.
#[derive(Debug, Clone, Copy)]
pub struct LanguageCapabilities<'a> {
    types: &'a [TypeCapabilities<'a>],
}

/// The permissive legacy profile used by unknown external adapters.
#[deprecated(note = "declare a strict local capability matrix instead")]
const ALL_FEATURES: &[SpecCapability] = &[
    SpecCapability::RecordFields,
    SpecCapability::AccessorMethods,
    SpecCapability::Methods,
    SpecCapability::StructuralEmbedding,
    SpecCapability::NominalSubtyping,
    SpecCapability::InterfaceImplementation,
    SpecCapability::ParametricPolymorphism,
    SpecCapability::BoundedPolymorphism,
    SpecCapability::ConstructorParameters,
    SpecCapability::Variants,
    SpecCapability::Attributes,
    SpecCapability::OptionalRecordFields,
];

#[deprecated(note = "declare a strict local capability matrix instead")]
#[allow(deprecated)]
const ALL_TYPES: &[TypeCapabilities<'static>] = &[
    TypeCapabilities::new(TypeKind::Class, ALL_FEATURES),
    TypeCapabilities::new(TypeKind::Struct, ALL_FEATURES),
    TypeCapabilities::new(TypeKind::Interface, ALL_FEATURES),
    TypeCapabilities::new(TypeKind::Trait, ALL_FEATURES),
    TypeCapabilities::new(TypeKind::Enum, ALL_FEATURES),
    TypeCapabilities::new(TypeKind::TypeAlias, ALL_FEATURES),
    TypeCapabilities::new(TypeKind::Newtype, ALL_FEATURES),
];

impl<'a> LanguageCapabilities<'a> {
    /// Create a strict matrix from per-kind profiles.
    pub const fn new(types: &'a [TypeCapabilities<'a>]) -> Self {
        Self { types }
    }

    /// Legacy permissive profile for unknown external adapters.
    #[deprecated(note = "declare a strict local capability matrix instead")]
    #[allow(deprecated)]
    pub const fn all() -> Self {
        Self { types: ALL_TYPES }
    }

    /// Whether any profile declares this type kind.
    pub fn supports_type_kind(&self, kind: TypeKind) -> bool {
        self.types.iter().any(|profile| profile.kind() == kind)
    }

    /// Whether the profile for `kind` declares `capability`.
    pub fn supports_capability(&self, kind: TypeKind, capability: SpecCapability) -> bool {
        self.types
            .iter()
            .find(|profile| profile.kind() == kind)
            .is_some_and(|profile| profile.supports(capability))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(deprecated)]
    fn all_is_permissive_for_every_kind_and_capability() {
        let all = LanguageCapabilities::all();
        let kinds = [
            TypeKind::Class,
            TypeKind::Struct,
            TypeKind::Interface,
            TypeKind::Trait,
            TypeKind::Enum,
            TypeKind::TypeAlias,
            TypeKind::Newtype,
        ];
        for kind in kinds {
            assert!(all.supports_type_kind(kind), "{kind:?}");
            for capability in ALL_FEATURES {
                assert!(
                    all.supports_capability(kind, *capability),
                    "{kind:?} {capability:?}"
                );
            }
        }
    }

    #[test]
    fn strict_matrix_checks_both_kind_and_capability() {
        const TYPES: &[TypeCapabilities] = &[TypeCapabilities::new(
            TypeKind::Struct,
            &[SpecCapability::RecordFields],
        )];
        let caps = LanguageCapabilities::new(TYPES);
        assert!(caps.supports_type_kind(TypeKind::Struct));
        assert!(!caps.supports_type_kind(TypeKind::Enum));
        assert!(caps.supports_capability(TypeKind::Struct, SpecCapability::RecordFields));
        assert!(!caps.supports_capability(TypeKind::Struct, SpecCapability::Methods));
    }
}
