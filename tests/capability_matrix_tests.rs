use sigil_stitch::lang::CodeLang;
use sigil_stitch::lang::capability::SpecCapability;
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

const ALL_CAPABILITIES: [SpecCapability; 12] = [
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

fn assert_matrix(lang: &dyn CodeLang, expected: &[(TypeKind, &[SpecCapability])]) {
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
                actual.supports_capability(kind, capability),
                expected_caps.contains(&capability),
                "{}.{} {:?}",
                lang.file_extension(),
                kind_name(kind),
                capability.as_str()
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
fn empty_matrices_for_shell_and_lua() {
    assert_matrix(&sigil_stitch::lang::bash::Bash::new(), &[]);
    assert_matrix(&sigil_stitch::lang::zsh::Zsh::new(), &[]);
    assert_matrix(&sigil_stitch::lang::lua::Lua::new(), &[]);
}

#[test]
fn c_matrix() {
    let record = &[
        SpecCapability::RecordFields,
        SpecCapability::Attributes,
        SpecCapability::OptionalRecordFields,
    ];
    let enum_caps = &[SpecCapability::Variants, SpecCapability::Attributes];
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
        SpecCapability::RecordFields,
        SpecCapability::Methods,
        SpecCapability::NominalSubtyping,
        SpecCapability::ParametricPolymorphism,
        SpecCapability::Attributes,
        SpecCapability::OptionalRecordFields,
    ];
    let record_caps = &[
        SpecCapability::RecordFields,
        SpecCapability::Attributes,
        SpecCapability::OptionalRecordFields,
    ];
    let enum_caps = &[SpecCapability::Variants, SpecCapability::Attributes];
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
                &[SpecCapability::ParametricPolymorphism],
            ),
        ],
    );
}

#[test]
fn csharp_matrix() {
    let class_caps = &[
        SpecCapability::RecordFields,
        SpecCapability::Methods,
        SpecCapability::NominalSubtyping,
        SpecCapability::ParametricPolymorphism,
        SpecCapability::BoundedPolymorphism,
        SpecCapability::Attributes,
        SpecCapability::OptionalRecordFields,
    ];
    let struct_caps = &[
        SpecCapability::RecordFields,
        SpecCapability::Methods,
        SpecCapability::ParametricPolymorphism,
        SpecCapability::Attributes,
        SpecCapability::OptionalRecordFields,
    ];
    let contract_caps = &[
        SpecCapability::Methods,
        SpecCapability::NominalSubtyping,
        SpecCapability::ParametricPolymorphism,
        SpecCapability::BoundedPolymorphism,
        SpecCapability::Attributes,
    ];
    let enum_caps = &[SpecCapability::Variants, SpecCapability::Attributes];
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
        SpecCapability::RecordFields,
        SpecCapability::Methods,
        SpecCapability::NominalSubtyping,
        SpecCapability::InterfaceImplementation,
        SpecCapability::ParametricPolymorphism,
        SpecCapability::Attributes,
        SpecCapability::OptionalRecordFields,
    ];
    let contract_caps = &[
        SpecCapability::Methods,
        SpecCapability::NominalSubtyping,
        SpecCapability::InterfaceImplementation,
        SpecCapability::ParametricPolymorphism,
        SpecCapability::Attributes,
    ];
    assert_matrix(
        &sigil_stitch::lang::dart::Dart::new(),
        &[
            (TypeKind::Class, class_caps),
            (TypeKind::Struct, class_caps),
            (TypeKind::Interface, contract_caps),
            (TypeKind::Trait, contract_caps),
            (TypeKind::Enum, &[SpecCapability::Variants]),
            (
                TypeKind::TypeAlias,
                &[SpecCapability::ParametricPolymorphism],
            ),
        ],
    );
}

#[test]
fn go_matrix() {
    let struct_caps = &[
        SpecCapability::RecordFields,
        SpecCapability::StructuralEmbedding,
        SpecCapability::ParametricPolymorphism,
        SpecCapability::OptionalRecordFields,
    ];
    let contract_caps = &[SpecCapability::Methods, SpecCapability::StructuralEmbedding];
    assert_matrix(
        &sigil_stitch::lang::go::Go::new(),
        &[
            (TypeKind::Struct, struct_caps),
            (TypeKind::Class, struct_caps),
            (TypeKind::Interface, contract_caps),
            (TypeKind::Trait, contract_caps),
            (TypeKind::TypeAlias, &[]),
            (TypeKind::Newtype, &[SpecCapability::ParametricPolymorphism]),
        ],
    );
}

#[test]
fn haskell_matrix() {
    let data_caps = &[
        SpecCapability::RecordFields,
        SpecCapability::ParametricPolymorphism,
        SpecCapability::BoundedPolymorphism,
        SpecCapability::ConstructorParameters,
        SpecCapability::Variants,
        SpecCapability::InterfaceImplementation,
    ];
    let contract_caps = &[
        SpecCapability::Methods,
        SpecCapability::ParametricPolymorphism,
        SpecCapability::BoundedPolymorphism,
    ];
    let enum_caps = &[
        SpecCapability::ParametricPolymorphism,
        SpecCapability::BoundedPolymorphism,
        SpecCapability::ConstructorParameters,
        SpecCapability::Variants,
        SpecCapability::InterfaceImplementation,
    ];
    let newtype_caps = &[
        SpecCapability::ParametricPolymorphism,
        SpecCapability::BoundedPolymorphism,
        SpecCapability::InterfaceImplementation,
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
                &[SpecCapability::ParametricPolymorphism],
            ),
            (TypeKind::Newtype, newtype_caps),
        ],
    );
}

#[test]
fn java_matrix() {
    let class_caps = &[
        SpecCapability::RecordFields,
        SpecCapability::Methods,
        SpecCapability::NominalSubtyping,
        SpecCapability::InterfaceImplementation,
        SpecCapability::ParametricPolymorphism,
        SpecCapability::BoundedPolymorphism,
        SpecCapability::Attributes,
        SpecCapability::OptionalRecordFields,
    ];
    let contract_caps = &[
        SpecCapability::Methods,
        SpecCapability::NominalSubtyping,
        SpecCapability::ParametricPolymorphism,
        SpecCapability::BoundedPolymorphism,
        SpecCapability::Attributes,
    ];
    let enum_caps = &[
        SpecCapability::RecordFields,
        SpecCapability::Methods,
        SpecCapability::InterfaceImplementation,
        SpecCapability::Attributes,
        SpecCapability::Variants,
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
        SpecCapability::RecordFields,
        SpecCapability::Methods,
        SpecCapability::NominalSubtyping,
        SpecCapability::Attributes,
        SpecCapability::OptionalRecordFields,
    ];
    let enum_caps = &[
        SpecCapability::RecordFields,
        SpecCapability::Methods,
        SpecCapability::Variants,
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
        SpecCapability::RecordFields,
        SpecCapability::AccessorMethods,
        SpecCapability::Methods,
        SpecCapability::NominalSubtyping,
        SpecCapability::ParametricPolymorphism,
        SpecCapability::BoundedPolymorphism,
        SpecCapability::ConstructorParameters,
        SpecCapability::Attributes,
        SpecCapability::OptionalRecordFields,
    ];
    let contract_caps = &[
        SpecCapability::AccessorMethods,
        SpecCapability::Methods,
        SpecCapability::NominalSubtyping,
        SpecCapability::ParametricPolymorphism,
        SpecCapability::BoundedPolymorphism,
        SpecCapability::Attributes,
    ];
    let enum_caps = &[
        SpecCapability::RecordFields,
        SpecCapability::AccessorMethods,
        SpecCapability::Methods,
        SpecCapability::ConstructorParameters,
        SpecCapability::Attributes,
        SpecCapability::Variants,
    ];
    let newtype_caps = &[
        SpecCapability::ParametricPolymorphism,
        SpecCapability::Attributes,
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
                &[SpecCapability::ParametricPolymorphism],
            ),
            (TypeKind::Newtype, newtype_caps),
        ],
    );
}

#[test]
fn ocaml_matrix() {
    let record_caps = &[
        SpecCapability::RecordFields,
        SpecCapability::ParametricPolymorphism,
    ];
    let variant_caps = &[
        SpecCapability::ParametricPolymorphism,
        SpecCapability::ConstructorParameters,
        SpecCapability::Variants,
    ];
    assert_matrix(
        &sigil_stitch::lang::ocaml::OCaml::new(),
        &[
            (TypeKind::Struct, record_caps),
            (TypeKind::Class, record_caps),
            (TypeKind::Enum, variant_caps),
            (
                TypeKind::TypeAlias,
                &[SpecCapability::ParametricPolymorphism],
            ),
        ],
    );
}

#[test]
fn php_matrix() {
    let class_caps = &[
        SpecCapability::RecordFields,
        SpecCapability::AccessorMethods,
        SpecCapability::Methods,
        SpecCapability::NominalSubtyping,
        SpecCapability::InterfaceImplementation,
        SpecCapability::Attributes,
        SpecCapability::OptionalRecordFields,
    ];
    let interface_caps = &[
        SpecCapability::Methods,
        SpecCapability::NominalSubtyping,
        SpecCapability::Attributes,
    ];
    let trait_caps = &[
        SpecCapability::RecordFields,
        SpecCapability::AccessorMethods,
        SpecCapability::Methods,
        SpecCapability::Attributes,
    ];
    let enum_caps = &[
        SpecCapability::Methods,
        SpecCapability::InterfaceImplementation,
        SpecCapability::Attributes,
        SpecCapability::Variants,
    ];
    assert_matrix(
        &sigil_stitch::lang::php::Php::new(),
        &[
            (TypeKind::Class, class_caps),
            (TypeKind::Struct, class_caps),
            (TypeKind::Interface, interface_caps),
            (TypeKind::Trait, trait_caps),
            (TypeKind::Enum, enum_caps),
            (TypeKind::Newtype, &[SpecCapability::Attributes]),
        ],
    );
}

#[test]
fn python_matrix() {
    let class_caps = &[
        SpecCapability::RecordFields,
        SpecCapability::Methods,
        SpecCapability::StructuralEmbedding,
        SpecCapability::NominalSubtyping,
        SpecCapability::InterfaceImplementation,
        SpecCapability::Attributes,
        SpecCapability::OptionalRecordFields,
    ];
    let enum_caps = &[
        SpecCapability::Methods,
        SpecCapability::NominalSubtyping,
        SpecCapability::Variants,
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
                &[SpecCapability::ParametricPolymorphism],
            ),
            (TypeKind::Newtype, &[]),
        ],
    );
}

#[test]
fn ruby_matrix() {
    let class_caps = &[
        SpecCapability::RecordFields,
        SpecCapability::Methods,
        SpecCapability::NominalSubtyping,
        SpecCapability::Attributes,
    ];
    let contract_caps = &[SpecCapability::Methods, SpecCapability::Attributes];
    let enum_caps = &[SpecCapability::Variants, SpecCapability::Attributes];
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
        SpecCapability::RecordFields,
        SpecCapability::Methods,
        SpecCapability::StructuralEmbedding,
        SpecCapability::ParametricPolymorphism,
        SpecCapability::BoundedPolymorphism,
        SpecCapability::Attributes,
        SpecCapability::OptionalRecordFields,
    ];
    let contract_caps = &[
        SpecCapability::Methods,
        SpecCapability::ParametricPolymorphism,
        SpecCapability::BoundedPolymorphism,
        SpecCapability::Attributes,
    ];
    let enum_caps = &[
        SpecCapability::Methods,
        SpecCapability::ParametricPolymorphism,
        SpecCapability::BoundedPolymorphism,
        SpecCapability::Attributes,
        SpecCapability::Variants,
    ];
    let alias_caps = &[
        SpecCapability::ParametricPolymorphism,
        SpecCapability::BoundedPolymorphism,
    ];
    let newtype_caps = &[
        SpecCapability::ParametricPolymorphism,
        SpecCapability::BoundedPolymorphism,
        SpecCapability::Attributes,
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
        SpecCapability::RecordFields,
        SpecCapability::AccessorMethods,
        SpecCapability::Methods,
        SpecCapability::NominalSubtyping,
        SpecCapability::InterfaceImplementation,
        SpecCapability::ParametricPolymorphism,
        SpecCapability::BoundedPolymorphism,
        SpecCapability::ConstructorParameters,
        SpecCapability::Attributes,
    ];
    let contract_caps = &[
        SpecCapability::Methods,
        SpecCapability::NominalSubtyping,
        SpecCapability::ParametricPolymorphism,
        SpecCapability::BoundedPolymorphism,
        SpecCapability::Attributes,
    ];
    let enum_caps = &[
        SpecCapability::RecordFields,
        SpecCapability::AccessorMethods,
        SpecCapability::Methods,
        SpecCapability::ParametricPolymorphism,
        SpecCapability::Attributes,
        SpecCapability::Variants,
    ];
    let newtype_caps = &[
        SpecCapability::ParametricPolymorphism,
        SpecCapability::Attributes,
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
                &[SpecCapability::ParametricPolymorphism],
            ),
            (TypeKind::Newtype, newtype_caps),
        ],
    );
}

#[test]
fn swift_matrix() {
    let class_caps = &[
        SpecCapability::RecordFields,
        SpecCapability::AccessorMethods,
        SpecCapability::Methods,
        SpecCapability::NominalSubtyping,
        SpecCapability::ParametricPolymorphism,
        SpecCapability::BoundedPolymorphism,
        SpecCapability::Attributes,
        SpecCapability::OptionalRecordFields,
    ];
    let enum_caps = &[
        SpecCapability::RecordFields,
        SpecCapability::Methods,
        SpecCapability::Attributes,
        SpecCapability::Variants,
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
        SpecCapability::RecordFields,
        SpecCapability::AccessorMethods,
        SpecCapability::Methods,
        SpecCapability::StructuralEmbedding,
        SpecCapability::NominalSubtyping,
        SpecCapability::InterfaceImplementation,
        SpecCapability::ParametricPolymorphism,
        SpecCapability::BoundedPolymorphism,
        SpecCapability::Attributes,
        SpecCapability::OptionalRecordFields,
    ];
    let contract_caps = &[
        SpecCapability::RecordFields,
        SpecCapability::AccessorMethods,
        SpecCapability::Methods,
        SpecCapability::StructuralEmbedding,
        SpecCapability::NominalSubtyping,
        SpecCapability::ParametricPolymorphism,
        SpecCapability::BoundedPolymorphism,
        SpecCapability::Attributes,
        SpecCapability::OptionalRecordFields,
    ];
    assert_matrix(
        &sigil_stitch::lang::typescript::TypeScript::new(),
        &[
            (TypeKind::Class, class_caps),
            (TypeKind::Struct, class_caps),
            (TypeKind::Interface, contract_caps),
            (TypeKind::Trait, contract_caps),
            (TypeKind::Enum, &[SpecCapability::Variants]),
            (
                TypeKind::TypeAlias,
                &[SpecCapability::ParametricPolymorphism],
            ),
        ],
    );
}
