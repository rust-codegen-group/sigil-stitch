use sigil_stitch::lang::CodeLang;
use sigil_stitch::lang::capability::{PropertyCapability, PropertyContext};
use sigil_stitch::spec::modifiers::{DeclarationContext, TypeKind};

#[path = "shared/languages.rs"]
mod languages_registry;

use languages_registry::{BUILT_IN_LANGUAGES, adapter_for};

use PropertyCapability::{Attributes, ExplicitType, ReadAccessor, StaticProperty, WriteAccessor};

const ALL_CAPABILITIES: &[PropertyCapability] = &[
    ExplicitType,
    ReadAccessor,
    WriteAccessor,
    Attributes,
    StaticProperty,
];

const ALL_CONTEXTS: &[PropertyContext] = &[
    PropertyContext::Direct(DeclarationContext::TopLevel),
    PropertyContext::Direct(DeclarationContext::Member),
    PropertyContext::Direct(DeclarationContext::InterfaceMember),
    PropertyContext::TypeMember(TypeKind::Class),
    PropertyContext::TypeMember(TypeKind::Struct),
    PropertyContext::TypeMember(TypeKind::Interface),
    PropertyContext::TypeMember(TypeKind::Trait),
    PropertyContext::TypeMember(TypeKind::Enum),
    PropertyContext::TypeMember(TypeKind::TypeAlias),
    PropertyContext::TypeMember(TypeKind::Newtype),
];

#[derive(Clone, Copy)]
struct ExpectedProfile {
    context: PropertyContext,
    capabilities: &'static [PropertyCapability],
    required: &'static [PropertyCapability],
}

const fn profile(
    context: PropertyContext,
    capabilities: &'static [PropertyCapability],
    required: &'static [PropertyCapability],
) -> ExpectedProfile {
    ExpectedProfile {
        context,
        capabilities,
        required,
    }
}

fn assert_matrix(lang: &dyn CodeLang, expected: &[ExpectedProfile]) {
    let capabilities = lang.capabilities();
    for context in ALL_CONTEXTS {
        let expected = expected.iter().find(|profile| profile.context == *context);
        assert_eq!(
            capabilities.supports_property_context(*context),
            expected.is_some(),
            "{} {context:?}",
            lang.file_extension()
        );
        for capability in ALL_CAPABILITIES {
            assert_eq!(
                capabilities.supports_property_capability(*context, *capability),
                expected.is_some_and(|profile| profile.capabilities.contains(capability)),
                "{} {context:?} {capability:?}",
                lang.file_extension()
            );
        }
        assert_eq!(
            capabilities.required_property_capabilities(*context),
            expected.map_or(&[][..], |profile| profile.required),
            "{} {context:?} required",
            lang.file_extension()
        );
        if let PropertyContext::TypeMember(kind) = context {
            assert_eq!(
                capabilities.supports_property_context(*context),
                capabilities.supports_type_capability(
                    *kind,
                    sigil_stitch::lang::capability::TypeCapability::AccessorMethods,
                ),
                "{} {kind:?} property and type profiles disagree",
                lang.file_extension()
            );
        }
    }
}

const DYNAMIC: &[PropertyCapability] = &[ReadAccessor, WriteAccessor, Attributes, StaticProperty];
const TYPED: &[PropertyCapability] = &[
    ExplicitType,
    ReadAccessor,
    WriteAccessor,
    Attributes,
    StaticProperty,
];
const VALUE_PROPERTY: &[PropertyCapability] =
    &[ExplicitType, ReadAccessor, WriteAccessor, Attributes];
const REQUIRED_TYPED_READ: &[PropertyCapability] = &[ExplicitType, ReadAccessor];

#[test]
fn built_in_property_matrices_are_exhaustive() {
    use DeclarationContext::{InterfaceMember, Member};
    use PropertyContext::{Direct, TypeMember};
    use TypeKind::{Class, Enum, Interface, Struct, Trait};

    assert_matrix(
        adapter_for("javascript").as_ref(),
        &[
            profile(Direct(Member), DYNAMIC, &[]),
            profile(Direct(InterfaceMember), DYNAMIC, &[]),
            profile(TypeMember(Class), DYNAMIC, &[]),
            profile(TypeMember(Struct), DYNAMIC, &[]),
            profile(TypeMember(Interface), DYNAMIC, &[]),
            profile(TypeMember(Trait), DYNAMIC, &[]),
            profile(TypeMember(Enum), DYNAMIC, &[]),
        ],
    );
    assert_matrix(
        adapter_for("kotlin").as_ref(),
        &[
            profile(Direct(Member), VALUE_PROPERTY, REQUIRED_TYPED_READ),
            profile(Direct(InterfaceMember), VALUE_PROPERTY, REQUIRED_TYPED_READ),
            profile(TypeMember(Class), VALUE_PROPERTY, REQUIRED_TYPED_READ),
            profile(TypeMember(Struct), VALUE_PROPERTY, REQUIRED_TYPED_READ),
            profile(TypeMember(Interface), VALUE_PROPERTY, REQUIRED_TYPED_READ),
            profile(TypeMember(Trait), VALUE_PROPERTY, REQUIRED_TYPED_READ),
            profile(TypeMember(Enum), VALUE_PROPERTY, REQUIRED_TYPED_READ),
        ],
    );
    assert_matrix(
        adapter_for("php").as_ref(),
        &[
            profile(Direct(Member), TYPED, &[]),
            profile(TypeMember(Class), TYPED, &[]),
            profile(TypeMember(Struct), TYPED, &[]),
            profile(TypeMember(Trait), TYPED, &[]),
        ],
    );
    assert_matrix(
        adapter_for("scala").as_ref(),
        &[
            profile(Direct(Member), VALUE_PROPERTY, &[]),
            profile(TypeMember(Class), VALUE_PROPERTY, &[]),
            profile(TypeMember(Struct), VALUE_PROPERTY, &[]),
            profile(TypeMember(Enum), VALUE_PROPERTY, &[]),
        ],
    );
    assert_matrix(
        adapter_for("swift").as_ref(),
        &[
            profile(Direct(Member), TYPED, REQUIRED_TYPED_READ),
            profile(TypeMember(Class), TYPED, REQUIRED_TYPED_READ),
            profile(TypeMember(Struct), TYPED, REQUIRED_TYPED_READ),
        ],
    );
    assert_matrix(
        adapter_for("typescript").as_ref(),
        &[
            profile(Direct(Member), TYPED, &[]),
            profile(TypeMember(Class), TYPED, &[]),
            profile(TypeMember(Struct), TYPED, &[]),
        ],
    );
}

#[test]
fn unsupported_built_ins_have_no_property_profiles() {
    const SUPPORTED: &[&str] = &[
        "javascript",
        "kotlin",
        "php",
        "scala",
        "swift",
        "typescript",
    ];

    for language in BUILT_IN_LANGUAGES
        .into_iter()
        .filter(|language| !SUPPORTED.contains(&language.id))
    {
        assert_matrix(language.adapter().as_ref(), &[]);
    }
}
