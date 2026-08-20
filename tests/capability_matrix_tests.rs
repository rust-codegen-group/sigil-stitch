use sigil_stitch::lang::CodeLang;
use sigil_stitch::lang::capability::{
    FunctionBodyPolicy, FunctionCapability, FunctionCapabilityProfile, FunctionContext,
    FunctionForm, LanguageCapabilities, TypeCapability, TypeCapabilityProfile,
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

const ALL_CAPABILITIES: [TypeCapability; 12] = [
    TypeCapability::RecordFields,
    TypeCapability::AccessorMethods,
    TypeCapability::Methods,
    TypeCapability::StructuralEmbedding,
    TypeCapability::NominalSubtyping,
    TypeCapability::InterfaceImplementation,
    TypeCapability::ParametricPolymorphism,
    TypeCapability::BoundedPolymorphism,
    TypeCapability::ConstructorParameters,
    TypeCapability::Variants,
    TypeCapability::Attributes,
    TypeCapability::OptionalRecordFields,
];

fn assert_matrix(lang: &dyn CodeLang, expected: &[(TypeKind, &[TypeCapability])]) {
    let actual = lang.capabilities();
    for kind in ALL_KINDS {
        let expected_supported = expected.iter().any(|(candidate, _)| *candidate == kind);
        assert_eq!(
            actual.supports_type_kind(kind),
            expected_supported,
            "{}.{} expected supported={expected_supported}",
            lang.file_extension(),
            kind_name(kind)
        );
        let expected_caps = expected
            .iter()
            .find(|(candidate, _)| *candidate == kind)
            .map(|(_, caps)| *caps)
            .unwrap_or(&[]);
        for capability in ALL_CAPABILITIES {
            assert_eq!(
                actual.supports_type_capability(kind, capability),
                expected_caps.contains(&capability),
                "{}.{} {capability:?}",
                lang.file_extension(),
                kind_name(kind)
            );
        }
    }
}

fn kind_name(kind: TypeKind) -> &'static str {
    match kind {
        TypeKind::Class => "Class",
        TypeKind::Struct => "Struct",
        TypeKind::Interface => "Interface",
        TypeKind::Trait => "Trait",
        TypeKind::Enum => "Enum",
        TypeKind::TypeAlias => "TypeAlias",
        TypeKind::Newtype => "Newtype",
    }
}

#[test]
fn capability_profile_builders_preserve_semantic_policy() {
    // Production matrices construct these profiles in constants. Keep the
    // inputs opaque so this test also exercises their runtime API contract.
    let type_kind = std::hint::black_box(TypeKind::Class);
    let type_capabilities = std::hint::black_box(&[TypeCapability::Methods][..]);
    let type_profile = TypeCapabilityProfile::new(type_kind, type_capabilities);
    assert_eq!(type_profile.kind(), TypeKind::Class);
    assert!(type_profile.supports(TypeCapability::Methods));

    let context = std::hint::black_box(FunctionContext::Member);
    let capabilities = std::hint::black_box(
        &[
            FunctionCapability::ExplicitReturnType,
            FunctionCapability::TypedParameters,
        ][..],
    );
    let required = std::hint::black_box(&[FunctionCapability::ExplicitReturnType][..]);
    let incompatible = [(
        FunctionCapability::AsyncEffect,
        FunctionCapability::StaticMethod,
    )];
    let incompatible = std::hint::black_box(&incompatible[..]);
    let profile = FunctionCapabilityProfile::new(context, FunctionForm::Function, capabilities)
        .with_required_capabilities(required)
        .with_incompatible_capabilities(incompatible)
        .with_body_policy(FunctionBodyPolicy::Required)
        .with_maximum_parameters(2);

    assert_eq!(profile.context(), FunctionContext::Member);
    assert_eq!(profile.form(), FunctionForm::Function);
    assert!(profile.supports(FunctionCapability::TypedParameters));
    assert_eq!(profile.required_capabilities(), required);
    assert_eq!(profile.incompatible_capabilities(), incompatible);
    assert_eq!(profile.body_policy(), FunctionBodyPolicy::Required);
    assert_eq!(profile.maximum_parameters(), Some(2));

    let permissive = LanguageCapabilities::permissive();
    assert_eq!(
        permissive.function_body_policy(FunctionContext::TopLevel, FunctionForm::Function),
        FunctionBodyPolicy::Optional
    );
    assert!(
        permissive
            .incompatible_function_capabilities(FunctionContext::TopLevel, FunctionForm::Function,)
            .is_empty()
    );
    assert_eq!(
        permissive.maximum_function_parameters(FunctionContext::TopLevel, FunctionForm::Function,),
        None
    );
}

#[test]
fn empty_matrices_for_shell_and_lua() {
    assert_matrix(&sigil_stitch::lang::bash::Bash::new(), &[]);
    assert_matrix(&sigil_stitch::lang::zsh::Zsh::new(), &[]);
    assert_matrix(&sigil_stitch::lang::lua::Lua::new(), &[]);
}

#[test]
fn c_matrix() {
    let record = &[
        TypeCapability::RecordFields,
        TypeCapability::Attributes,
        TypeCapability::OptionalRecordFields,
    ];
    let enum_caps = &[TypeCapability::Variants, TypeCapability::Attributes];
    assert_matrix(
        &sigil_stitch::lang::c::C::new(),
        &[
            (TypeKind::Struct, record),
            (TypeKind::Class, record),
            (TypeKind::Interface, record),
            (TypeKind::Trait, record),
            (TypeKind::Enum, enum_caps),
            (TypeKind::TypeAlias, &[]),
            (TypeKind::Newtype, &[]),
        ],
    );
}

#[test]
fn cpp_matrix() {
    let class_caps = &[
        TypeCapability::RecordFields,
        TypeCapability::Methods,
        TypeCapability::NominalSubtyping,
        TypeCapability::ParametricPolymorphism,
        TypeCapability::Attributes,
        TypeCapability::OptionalRecordFields,
    ];
    let record_caps = &[
        TypeCapability::RecordFields,
        TypeCapability::Attributes,
        TypeCapability::OptionalRecordFields,
    ];
    let enum_caps = &[TypeCapability::Variants, TypeCapability::Attributes];
    assert_matrix(
        &sigil_stitch::lang::cpp::Cpp::new(),
        &[
            (TypeKind::Class, class_caps),
            (TypeKind::Struct, record_caps),
            (TypeKind::Interface, class_caps),
            (TypeKind::Trait, class_caps),
            (TypeKind::Enum, enum_caps),
            (
                TypeKind::TypeAlias,
                &[TypeCapability::ParametricPolymorphism],
            ),
        ],
    );
}

#[test]
fn csharp_matrix() {
    let class_caps = &[
        TypeCapability::RecordFields,
        TypeCapability::Methods,
        TypeCapability::NominalSubtyping,
        TypeCapability::ParametricPolymorphism,
        TypeCapability::BoundedPolymorphism,
        TypeCapability::Attributes,
        TypeCapability::OptionalRecordFields,
    ];
    let struct_caps = &[
        TypeCapability::RecordFields,
        TypeCapability::Methods,
        TypeCapability::ParametricPolymorphism,
        TypeCapability::Attributes,
        TypeCapability::OptionalRecordFields,
    ];
    let contract_caps = &[
        TypeCapability::Methods,
        TypeCapability::NominalSubtyping,
        TypeCapability::ParametricPolymorphism,
        TypeCapability::BoundedPolymorphism,
        TypeCapability::Attributes,
    ];
    let enum_caps = &[TypeCapability::Variants, TypeCapability::Attributes];
    assert_matrix(
        &sigil_stitch::lang::csharp::CSharp::new(),
        &[
            (TypeKind::Class, class_caps),
            (TypeKind::Struct, struct_caps),
            (TypeKind::Interface, contract_caps),
            (TypeKind::Trait, contract_caps),
            (TypeKind::Enum, enum_caps),
        ],
    );
}

#[test]
fn dart_matrix() {
    let class_caps = &[
        TypeCapability::RecordFields,
        TypeCapability::Methods,
        TypeCapability::NominalSubtyping,
        TypeCapability::InterfaceImplementation,
        TypeCapability::ParametricPolymorphism,
        TypeCapability::Attributes,
        TypeCapability::OptionalRecordFields,
    ];
    let contract_caps = &[
        TypeCapability::Methods,
        TypeCapability::NominalSubtyping,
        TypeCapability::InterfaceImplementation,
        TypeCapability::ParametricPolymorphism,
        TypeCapability::Attributes,
    ];
    assert_matrix(
        &sigil_stitch::lang::dart::Dart::new(),
        &[
            (TypeKind::Class, class_caps),
            (TypeKind::Struct, class_caps),
            (TypeKind::Interface, contract_caps),
            (TypeKind::Trait, contract_caps),
            (TypeKind::Enum, &[TypeCapability::Variants]),
            (
                TypeKind::TypeAlias,
                &[TypeCapability::ParametricPolymorphism],
            ),
        ],
    );
}

#[test]
fn go_matrix() {
    let struct_caps = &[
        TypeCapability::RecordFields,
        TypeCapability::StructuralEmbedding,
        TypeCapability::ParametricPolymorphism,
        TypeCapability::OptionalRecordFields,
    ];
    let contract_caps = &[TypeCapability::Methods, TypeCapability::StructuralEmbedding];
    assert_matrix(
        &sigil_stitch::lang::go::Go::new(),
        &[
            (TypeKind::Struct, struct_caps),
            (TypeKind::Class, struct_caps),
            (TypeKind::Interface, contract_caps),
            (TypeKind::Trait, contract_caps),
            (TypeKind::TypeAlias, &[]),
            (TypeKind::Newtype, &[TypeCapability::ParametricPolymorphism]),
        ],
    );
}

#[test]
fn haskell_matrix() {
    let data_caps = &[
        TypeCapability::RecordFields,
        TypeCapability::ParametricPolymorphism,
        TypeCapability::BoundedPolymorphism,
        TypeCapability::ConstructorParameters,
        TypeCapability::Variants,
        TypeCapability::InterfaceImplementation,
    ];
    let contract_caps = &[
        TypeCapability::Methods,
        TypeCapability::ParametricPolymorphism,
        TypeCapability::BoundedPolymorphism,
    ];
    let enum_caps = &[
        TypeCapability::ParametricPolymorphism,
        TypeCapability::BoundedPolymorphism,
        TypeCapability::ConstructorParameters,
        TypeCapability::Variants,
        TypeCapability::InterfaceImplementation,
    ];
    let newtype_caps = &[
        TypeCapability::ParametricPolymorphism,
        TypeCapability::BoundedPolymorphism,
        TypeCapability::InterfaceImplementation,
    ];
    assert_matrix(
        &sigil_stitch::lang::haskell::Haskell::new(),
        &[
            (TypeKind::Struct, data_caps),
            (TypeKind::Class, data_caps),
            (TypeKind::Trait, contract_caps),
            (TypeKind::Interface, contract_caps),
            (TypeKind::Enum, enum_caps),
            (
                TypeKind::TypeAlias,
                &[TypeCapability::ParametricPolymorphism],
            ),
            (TypeKind::Newtype, newtype_caps),
        ],
    );
}

#[test]
fn java_matrix() {
    let class_caps = &[
        TypeCapability::RecordFields,
        TypeCapability::Methods,
        TypeCapability::NominalSubtyping,
        TypeCapability::InterfaceImplementation,
        TypeCapability::ParametricPolymorphism,
        TypeCapability::BoundedPolymorphism,
        TypeCapability::Attributes,
        TypeCapability::OptionalRecordFields,
    ];
    let contract_caps = &[
        TypeCapability::Methods,
        TypeCapability::NominalSubtyping,
        TypeCapability::ParametricPolymorphism,
        TypeCapability::BoundedPolymorphism,
        TypeCapability::Attributes,
    ];
    let enum_caps = &[
        TypeCapability::RecordFields,
        TypeCapability::Methods,
        TypeCapability::InterfaceImplementation,
        TypeCapability::Attributes,
        TypeCapability::Variants,
    ];
    assert_matrix(
        &sigil_stitch::lang::java::Java::new(),
        &[
            (TypeKind::Class, class_caps),
            (TypeKind::Struct, class_caps),
            (TypeKind::Interface, contract_caps),
            (TypeKind::Trait, contract_caps),
            (TypeKind::Enum, enum_caps),
        ],
    );
}

#[test]
fn javascript_matrix() {
    let class_caps = &[
        TypeCapability::RecordFields,
        TypeCapability::Methods,
        TypeCapability::NominalSubtyping,
        TypeCapability::Attributes,
        TypeCapability::OptionalRecordFields,
    ];
    let enum_caps = &[
        TypeCapability::RecordFields,
        TypeCapability::Methods,
        TypeCapability::Variants,
    ];
    assert_matrix(
        &sigil_stitch::lang::javascript::JavaScript::new(),
        &[
            (TypeKind::Class, class_caps),
            (TypeKind::Struct, class_caps),
            (TypeKind::Interface, class_caps),
            (TypeKind::Trait, class_caps),
            (TypeKind::Enum, enum_caps),
        ],
    );
}

#[test]
fn kotlin_matrix() {
    let class_caps = &[
        TypeCapability::RecordFields,
        TypeCapability::AccessorMethods,
        TypeCapability::Methods,
        TypeCapability::NominalSubtyping,
        TypeCapability::ParametricPolymorphism,
        TypeCapability::BoundedPolymorphism,
        TypeCapability::ConstructorParameters,
        TypeCapability::Attributes,
        TypeCapability::OptionalRecordFields,
    ];
    let contract_caps = &[
        TypeCapability::AccessorMethods,
        TypeCapability::Methods,
        TypeCapability::NominalSubtyping,
        TypeCapability::ParametricPolymorphism,
        TypeCapability::BoundedPolymorphism,
        TypeCapability::Attributes,
    ];
    let enum_caps = &[
        TypeCapability::RecordFields,
        TypeCapability::AccessorMethods,
        TypeCapability::Methods,
        TypeCapability::ConstructorParameters,
        TypeCapability::Attributes,
        TypeCapability::Variants,
    ];
    let newtype_caps = &[
        TypeCapability::ParametricPolymorphism,
        TypeCapability::Attributes,
    ];
    assert_matrix(
        &sigil_stitch::lang::kotlin::Kotlin::new(),
        &[
            (TypeKind::Class, class_caps),
            (TypeKind::Struct, class_caps),
            (TypeKind::Interface, contract_caps),
            (TypeKind::Trait, contract_caps),
            (TypeKind::Enum, enum_caps),
            (
                TypeKind::TypeAlias,
                &[TypeCapability::ParametricPolymorphism],
            ),
            (TypeKind::Newtype, newtype_caps),
        ],
    );
}

#[test]
fn ocaml_matrix() {
    let record_caps = &[
        TypeCapability::RecordFields,
        TypeCapability::ParametricPolymorphism,
    ];
    let variant_caps = &[
        TypeCapability::ParametricPolymorphism,
        TypeCapability::ConstructorParameters,
        TypeCapability::Variants,
    ];
    assert_matrix(
        &sigil_stitch::lang::ocaml::OCaml::new(),
        &[
            (TypeKind::Struct, record_caps),
            (TypeKind::Class, record_caps),
            (TypeKind::Enum, variant_caps),
            (
                TypeKind::TypeAlias,
                &[TypeCapability::ParametricPolymorphism],
            ),
        ],
    );
}

#[test]
fn php_matrix() {
    let class_caps = &[
        TypeCapability::RecordFields,
        TypeCapability::AccessorMethods,
        TypeCapability::Methods,
        TypeCapability::NominalSubtyping,
        TypeCapability::InterfaceImplementation,
        TypeCapability::Attributes,
        TypeCapability::OptionalRecordFields,
    ];
    let interface_caps = &[
        TypeCapability::Methods,
        TypeCapability::NominalSubtyping,
        TypeCapability::Attributes,
    ];
    let trait_caps = &[
        TypeCapability::RecordFields,
        TypeCapability::AccessorMethods,
        TypeCapability::Methods,
        TypeCapability::Attributes,
    ];
    let enum_caps = &[
        TypeCapability::Methods,
        TypeCapability::InterfaceImplementation,
        TypeCapability::Attributes,
        TypeCapability::Variants,
    ];
    assert_matrix(
        &sigil_stitch::lang::php::Php::new(),
        &[
            (TypeKind::Class, class_caps),
            (TypeKind::Struct, class_caps),
            (TypeKind::Interface, interface_caps),
            (TypeKind::Trait, trait_caps),
            (TypeKind::Enum, enum_caps),
            (TypeKind::Newtype, &[TypeCapability::Attributes]),
        ],
    );
}

#[test]
fn python_matrix() {
    let class_caps = &[
        TypeCapability::RecordFields,
        TypeCapability::Methods,
        TypeCapability::StructuralEmbedding,
        TypeCapability::NominalSubtyping,
        TypeCapability::InterfaceImplementation,
        TypeCapability::Attributes,
        TypeCapability::OptionalRecordFields,
    ];
    let enum_caps = &[
        TypeCapability::Methods,
        TypeCapability::NominalSubtyping,
        TypeCapability::Variants,
    ];
    assert_matrix(
        &sigil_stitch::lang::python::Python::new(),
        &[
            (TypeKind::Class, class_caps),
            (TypeKind::Struct, class_caps),
            (TypeKind::Interface, class_caps),
            (TypeKind::Trait, class_caps),
            (TypeKind::Enum, enum_caps),
            (
                TypeKind::TypeAlias,
                &[TypeCapability::ParametricPolymorphism],
            ),
            (TypeKind::Newtype, &[]),
        ],
    );
}

#[test]
fn ruby_matrix() {
    let class_caps = &[
        TypeCapability::RecordFields,
        TypeCapability::Methods,
        TypeCapability::NominalSubtyping,
        TypeCapability::Attributes,
    ];
    let contract_caps = &[TypeCapability::Methods, TypeCapability::Attributes];
    let enum_caps = &[TypeCapability::Variants, TypeCapability::Attributes];
    assert_matrix(
        &sigil_stitch::lang::ruby::Ruby::new(),
        &[
            (TypeKind::Class, class_caps),
            (TypeKind::Struct, class_caps),
            (TypeKind::Interface, contract_caps),
            (TypeKind::Trait, contract_caps),
            (TypeKind::Enum, enum_caps),
        ],
    );
}

#[test]
fn rust_matrix() {
    let record_caps = &[
        TypeCapability::RecordFields,
        TypeCapability::Methods,
        TypeCapability::StructuralEmbedding,
        TypeCapability::ParametricPolymorphism,
        TypeCapability::BoundedPolymorphism,
        TypeCapability::Attributes,
        TypeCapability::OptionalRecordFields,
    ];
    let contract_caps = &[
        TypeCapability::Methods,
        TypeCapability::ParametricPolymorphism,
        TypeCapability::BoundedPolymorphism,
        TypeCapability::Attributes,
    ];
    let enum_caps = &[
        TypeCapability::Methods,
        TypeCapability::ParametricPolymorphism,
        TypeCapability::BoundedPolymorphism,
        TypeCapability::Attributes,
        TypeCapability::Variants,
    ];
    let alias_caps = &[
        TypeCapability::ParametricPolymorphism,
        TypeCapability::BoundedPolymorphism,
    ];
    let newtype_caps = &[
        TypeCapability::ParametricPolymorphism,
        TypeCapability::BoundedPolymorphism,
        TypeCapability::Attributes,
    ];
    assert_matrix(
        &sigil_stitch::lang::rust::Rust::new(),
        &[
            (TypeKind::Struct, record_caps),
            (TypeKind::Class, record_caps),
            (TypeKind::Trait, contract_caps),
            (TypeKind::Interface, contract_caps),
            (TypeKind::Enum, enum_caps),
            (TypeKind::TypeAlias, alias_caps),
            (TypeKind::Newtype, newtype_caps),
        ],
    );
}

#[test]
fn scala_matrix() {
    let class_caps = &[
        TypeCapability::RecordFields,
        TypeCapability::AccessorMethods,
        TypeCapability::Methods,
        TypeCapability::NominalSubtyping,
        TypeCapability::InterfaceImplementation,
        TypeCapability::ParametricPolymorphism,
        TypeCapability::BoundedPolymorphism,
        TypeCapability::ConstructorParameters,
        TypeCapability::Attributes,
    ];
    let contract_caps = &[
        TypeCapability::Methods,
        TypeCapability::NominalSubtyping,
        TypeCapability::ParametricPolymorphism,
        TypeCapability::BoundedPolymorphism,
        TypeCapability::Attributes,
    ];
    let enum_caps = &[
        TypeCapability::RecordFields,
        TypeCapability::AccessorMethods,
        TypeCapability::Methods,
        TypeCapability::ParametricPolymorphism,
        TypeCapability::Attributes,
        TypeCapability::Variants,
    ];
    let newtype_caps = &[
        TypeCapability::ParametricPolymorphism,
        TypeCapability::Attributes,
    ];
    assert_matrix(
        &sigil_stitch::lang::scala::Scala::new(),
        &[
            (TypeKind::Class, class_caps),
            (TypeKind::Struct, class_caps),
            (TypeKind::Trait, contract_caps),
            (TypeKind::Interface, contract_caps),
            (TypeKind::Enum, enum_caps),
            (
                TypeKind::TypeAlias,
                &[TypeCapability::ParametricPolymorphism],
            ),
            (TypeKind::Newtype, newtype_caps),
        ],
    );
}

#[test]
fn swift_matrix() {
    let class_caps = &[
        TypeCapability::RecordFields,
        TypeCapability::AccessorMethods,
        TypeCapability::Methods,
        TypeCapability::NominalSubtyping,
        TypeCapability::ParametricPolymorphism,
        TypeCapability::BoundedPolymorphism,
        TypeCapability::Attributes,
        TypeCapability::OptionalRecordFields,
    ];
    let enum_caps = &[
        TypeCapability::RecordFields,
        TypeCapability::Methods,
        TypeCapability::Attributes,
        TypeCapability::Variants,
    ];
    assert_matrix(
        &sigil_stitch::lang::swift::Swift::new(),
        &[
            (TypeKind::Class, class_caps),
            (TypeKind::Struct, class_caps),
            (TypeKind::Interface, class_caps),
            (TypeKind::Trait, class_caps),
            (TypeKind::Enum, enum_caps),
        ],
    );
}

#[test]
fn typescript_matrix() {
    let class_caps = &[
        TypeCapability::RecordFields,
        TypeCapability::AccessorMethods,
        TypeCapability::Methods,
        TypeCapability::StructuralEmbedding,
        TypeCapability::NominalSubtyping,
        TypeCapability::InterfaceImplementation,
        TypeCapability::ParametricPolymorphism,
        TypeCapability::BoundedPolymorphism,
        TypeCapability::Attributes,
        TypeCapability::OptionalRecordFields,
    ];
    let contract_caps = &[
        TypeCapability::RecordFields,
        TypeCapability::AccessorMethods,
        TypeCapability::Methods,
        TypeCapability::StructuralEmbedding,
        TypeCapability::NominalSubtyping,
        TypeCapability::ParametricPolymorphism,
        TypeCapability::BoundedPolymorphism,
        TypeCapability::Attributes,
        TypeCapability::OptionalRecordFields,
    ];
    assert_matrix(
        &sigil_stitch::lang::typescript::TypeScript::new(),
        &[
            (TypeKind::Class, class_caps),
            (TypeKind::Struct, class_caps),
            (TypeKind::Interface, contract_caps),
            (TypeKind::Trait, contract_caps),
            (TypeKind::Enum, &[TypeCapability::Variants]),
            (
                TypeKind::TypeAlias,
                &[TypeCapability::ParametricPolymorphism],
            ),
        ],
    );
}
