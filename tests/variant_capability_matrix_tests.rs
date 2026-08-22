use sigil_stitch::lang::CodeLang;
use sigil_stitch::lang::capability::{
    LanguageCapabilities, VariantCapability, VariantCapabilityProfile,
};
use sigil_stitch::spec::modifiers::TypeKind;

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
    assert_matrix(&sigil_stitch::lang::bash::Bash::new(), None);
    assert_matrix(&sigil_stitch::lang::zsh::Zsh::new(), None);
    assert_matrix(&sigil_stitch::lang::lua::Lua::new(), None);
    assert_matrix(&sigil_stitch::lang::go::Go::new(), None);
}

#[test]
fn discriminant_enum_profiles() {
    let discriminant = &[VariantCapability::Discriminant][..];
    assert_matrix(
        &sigil_stitch::lang::javascript::JavaScript::new(),
        Some(discriminant),
    );
    assert_matrix(
        &sigil_stitch::lang::python::Python::new(),
        Some(discriminant),
    );
    assert_matrix(
        &sigil_stitch::lang::typescript::TypeScript::new(),
        Some(discriminant),
    );

    let attributed = &[
        VariantCapability::Discriminant,
        VariantCapability::Attributes,
    ][..];
    assert_matrix(&sigil_stitch::lang::c::C::new(), Some(attributed));
    assert_matrix(&sigil_stitch::lang::cpp::Cpp::new(), Some(attributed));
    assert_matrix(&sigil_stitch::lang::csharp::CSharp::new(), Some(attributed));
    assert_matrix(&sigil_stitch::lang::ruby::Ruby::new(), Some(attributed));
}

#[test]
fn constructor_argument_profiles() {
    let expected = &[
        VariantCapability::ConstructorArguments,
        VariantCapability::Attributes,
    ][..];
    assert_matrix(&sigil_stitch::lang::java::Java::new(), Some(expected));
    assert_matrix(&sigil_stitch::lang::kotlin::Kotlin::new(), Some(expected));
}

#[test]
fn algebraic_payload_profiles() {
    let records = &[
        VariantCapability::PositionalPayload,
        VariantCapability::RecordPayload,
    ][..];
    assert_matrix(&sigil_stitch::lang::haskell::Haskell::new(), Some(records));
    assert_matrix(&sigil_stitch::lang::ocaml::OCaml::new(), Some(records));

    let rust = &[
        VariantCapability::Discriminant,
        VariantCapability::PositionalPayload,
        VariantCapability::RecordPayload,
        VariantCapability::Attributes,
    ][..];
    assert_matrix(&sigil_stitch::lang::rust::Rust::new(), Some(rust));

    let swift = &[
        VariantCapability::PositionalPayload,
        VariantCapability::Attributes,
    ][..];
    assert_matrix(&sigil_stitch::lang::swift::Swift::new(), Some(swift));
}

#[test]
fn simple_and_attributed_case_profiles() {
    assert_matrix(
        &sigil_stitch::lang::dart::Dart::new(),
        Some(&[VariantCapability::Attributes]),
    );
    assert_matrix(
        &sigil_stitch::lang::php::Php::new(),
        Some(&[VariantCapability::Attributes]),
    );
    assert_matrix(
        &sigil_stitch::lang::scala::Scala::new(),
        Some(&[VariantCapability::Attributes]),
    );
}
