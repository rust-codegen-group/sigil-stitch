use sigil_stitch::lang::CodeLang;
use sigil_stitch::lang::capability::{
    LanguageCapabilities, VariantCapability, VariantCapabilityProfile,
};
use sigil_stitch::spec::modifiers::TypeKind;

#[path = "shared/languages.rs"]
mod languages_registry;

use languages_registry::adapter_for;

const ALL_KINDS: [TypeKind; 7] = [
    TypeKind::Class,
    TypeKind::Struct,
    TypeKind::Interface,
    TypeKind::Trait,
    TypeKind::Enum,
    TypeKind::TypeAlias,
    TypeKind::Newtype,
];

const ALL_CAPABILITIES: [VariantCapability; 5] = [
    VariantCapability::Discriminant,
    VariantCapability::ConstructorArguments,
    VariantCapability::PositionalPayload,
    VariantCapability::RecordPayload,
    VariantCapability::Attributes,
];

fn assert_matrix(lang: &dyn CodeLang, expected: Option<&[VariantCapability]>) {
    let actual = lang.capabilities();
    for kind in ALL_KINDS {
        assert_eq!(
            actual.supports_variant_owner(kind),
            kind == TypeKind::Enum && expected.is_some(),
            "{}.owner.{kind:?}",
            lang.file_extension()
        );
        for capability in ALL_CAPABILITIES {
            assert_eq!(
                actual.supports_variant_capability(kind, capability),
                kind == TypeKind::Enum
                    && expected.is_some_and(|capabilities| capabilities.contains(&capability)),
                "{}.owner.{kind:?}.{capability:?}",
                lang.file_extension()
            );
        }
    }
}

#[test]
fn profile_and_registry_builders_preserve_variant_semantics() {
    let profile = VariantCapabilityProfile::new(
        TypeKind::Enum,
        &[
            VariantCapability::Discriminant,
            VariantCapability::Attributes,
        ],
    );
    assert_eq!(profile.owner_kind(), TypeKind::Enum);
    assert!(profile.supports(VariantCapability::Discriminant));
    assert!(!profile.supports(VariantCapability::RecordPayload));

    let profiles = [profile];
    let registry = LanguageCapabilities::strict().with_variants(&profiles);
    assert!(registry.supports_variant_owner(TypeKind::Enum));
    assert!(!registry.supports_variant_owner(TypeKind::Class));
    assert!(registry.supports_variant_capability(TypeKind::Enum, VariantCapability::Attributes));
}

#[test]
fn languages_without_variant_declarations_have_no_owner_profile() {
    assert_matrix(adapter_for("bash").as_ref(), None);
    assert_matrix(adapter_for("zsh").as_ref(), None);
    assert_matrix(adapter_for("lua").as_ref(), None);
    assert_matrix(adapter_for("go").as_ref(), None);
}

#[test]
fn discriminant_enum_profiles() {
    let discriminant = &[VariantCapability::Discriminant][..];
    assert_matrix(adapter_for("javascript").as_ref(), Some(discriminant));
    assert_matrix(adapter_for("python").as_ref(), Some(discriminant));
    assert_matrix(adapter_for("typescript").as_ref(), Some(discriminant));

    let attributed = &[
        VariantCapability::Discriminant,
        VariantCapability::Attributes,
    ][..];
    assert_matrix(adapter_for("c").as_ref(), Some(attributed));
    assert_matrix(adapter_for("cpp").as_ref(), Some(attributed));
    assert_matrix(adapter_for("csharp").as_ref(), Some(attributed));
    assert_matrix(adapter_for("ruby").as_ref(), Some(attributed));
}

#[test]
fn constructor_argument_profiles() {
    let expected = &[
        VariantCapability::ConstructorArguments,
        VariantCapability::Attributes,
    ][..];
    assert_matrix(adapter_for("java").as_ref(), Some(expected));
    assert_matrix(adapter_for("kotlin").as_ref(), Some(expected));
}

#[test]
fn algebraic_payload_profiles() {
    let records = &[
        VariantCapability::PositionalPayload,
        VariantCapability::RecordPayload,
    ][..];
    assert_matrix(adapter_for("haskell").as_ref(), Some(records));
    assert_matrix(adapter_for("ocaml").as_ref(), Some(records));

    let rust = &[
        VariantCapability::Discriminant,
        VariantCapability::PositionalPayload,
        VariantCapability::RecordPayload,
        VariantCapability::Attributes,
    ][..];
    assert_matrix(adapter_for("rust").as_ref(), Some(rust));

    let swift = &[
        VariantCapability::PositionalPayload,
        VariantCapability::Attributes,
    ][..];
    assert_matrix(adapter_for("swift").as_ref(), Some(swift));
}

#[test]
fn simple_and_attributed_case_profiles() {
    assert_matrix(
        adapter_for("dart").as_ref(),
        Some(&[VariantCapability::Attributes]),
    );
    assert_matrix(
        adapter_for("php").as_ref(),
        Some(&[VariantCapability::Attributes]),
    );
    assert_matrix(
        adapter_for("scala").as_ref(),
        Some(&[VariantCapability::Attributes]),
    );
}
