use sigil_stitch::lang::CodeLang;
use sigil_stitch::lang::capability::{FieldCapability, FieldContext};
use sigil_stitch::spec::modifiers::{DeclarationContext, TypeKind};

#[path = "shared/languages.rs"]
mod languages_registry;

use languages_registry::{BUILT_IN_LANGUAGES, adapter_for};

use FieldCapability::{
    Attributes, ExplicitType, Initializer, OptionalPresence, ReadOnly, StaticField,
};

const ALL_CAPABILITIES: &[FieldCapability] = &[
    ExplicitType,
    Initializer,
    Attributes,
    StaticField,
    ReadOnly,
    OptionalPresence,
];

const ALL_CONTEXTS: &[FieldContext] = &[
    FieldContext::Direct(DeclarationContext::TopLevel),
    FieldContext::Direct(DeclarationContext::Member),
    FieldContext::Direct(DeclarationContext::InterfaceMember),
    FieldContext::TypeMember(TypeKind::Class),
    FieldContext::TypeMember(TypeKind::Struct),
    FieldContext::TypeMember(TypeKind::Interface),
    FieldContext::TypeMember(TypeKind::Trait),
    FieldContext::TypeMember(TypeKind::Enum),
    FieldContext::TypeMember(TypeKind::TypeAlias),
    FieldContext::TypeMember(TypeKind::Newtype),
    FieldContext::VariantRecordPayload(TypeKind::Class),
    FieldContext::VariantRecordPayload(TypeKind::Struct),
    FieldContext::VariantRecordPayload(TypeKind::Interface),
    FieldContext::VariantRecordPayload(TypeKind::Trait),
    FieldContext::VariantRecordPayload(TypeKind::Enum),
    FieldContext::VariantRecordPayload(TypeKind::TypeAlias),
    FieldContext::VariantRecordPayload(TypeKind::Newtype),
];

#[derive(Clone, Copy)]
struct ExpectedProfile {
    context: FieldContext,
    capabilities: &'static [FieldCapability],
    required: &'static [FieldCapability],
}

const fn profile(
    context: FieldContext,
    capabilities: &'static [FieldCapability],
    required: &'static [FieldCapability],
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
        let profile = expected.iter().find(|profile| profile.context == *context);
        assert_eq!(
            capabilities.supports_field_context(*context),
            profile.is_some(),
            "{} {context:?}",
            lang.file_extension()
        );
        for capability in ALL_CAPABILITIES {
            assert_eq!(
                capabilities.supports_field_capability(*context, *capability),
                profile.is_some_and(|profile| profile.capabilities.contains(capability)),
                "{} {context:?} {capability:?}",
                lang.file_extension()
            );
        }
        assert_eq!(
            capabilities.required_field_capabilities(*context),
            profile.map_or(&[][..], |profile| profile.required),
            "{} {context:?} required",
            lang.file_extension()
        );
    }
}

const EXPLICIT: &[FieldCapability] = &[ExplicitType];
const C_FIELDS: &[FieldCapability] = &[ExplicitType, Attributes, ReadOnly];
const C_TOP_LEVEL_FIELDS: &[FieldCapability] =
    &[ExplicitType, Initializer, Attributes, StaticField, ReadOnly];
const FULL_TYPED: &[FieldCapability] =
    &[ExplicitType, Initializer, Attributes, StaticField, ReadOnly];
const DYNAMIC_FULL: &[FieldCapability] =
    &[ExplicitType, Initializer, Attributes, StaticField, ReadOnly];
const GO_FIELDS: &[FieldCapability] = &[ExplicitType, Attributes];
const IMMUTABLE_FIELDS: &[FieldCapability] = &[ExplicitType, ReadOnly];
const RUST_FIELDS: &[FieldCapability] = &[ExplicitType, Attributes];
const JS_FIELDS: &[FieldCapability] = &[Initializer, Attributes, StaticField];
const VALUE_FIELDS: &[FieldCapability] = &[ExplicitType, Initializer, Attributes, ReadOnly];
const PYTHON_FIELDS: &[FieldCapability] = &[ExplicitType, Initializer];
const TS_CONCRETE: &[FieldCapability] = &[
    ExplicitType,
    Initializer,
    Attributes,
    StaticField,
    ReadOnly,
    OptionalPresence,
];
const TS_CONTRACT: &[FieldCapability] = &[ExplicitType, ReadOnly, OptionalPresence];

#[test]
fn built_in_field_matrices_are_exhaustive() {
    use DeclarationContext::{InterfaceMember, Member, TopLevel};
    use FieldContext::{Direct, TypeMember, VariantRecordPayload};
    use TypeKind::{Class, Enum, Interface, Struct, Trait};

    assert_matrix(
        adapter_for("c").as_ref(),
        &[
            profile(Direct(TopLevel), C_TOP_LEVEL_FIELDS, EXPLICIT),
            profile(Direct(Member), C_FIELDS, EXPLICIT),
            profile(Direct(InterfaceMember), C_FIELDS, EXPLICIT),
            profile(TypeMember(Struct), C_FIELDS, EXPLICIT),
            profile(TypeMember(Class), C_FIELDS, EXPLICIT),
            profile(TypeMember(Interface), C_FIELDS, EXPLICIT),
            profile(TypeMember(Trait), C_FIELDS, EXPLICIT),
        ],
    );
    assert_matrix(
        adapter_for("cpp").as_ref(),
        &[
            profile(Direct(TopLevel), FULL_TYPED, EXPLICIT),
            profile(Direct(Member), FULL_TYPED, EXPLICIT),
            profile(Direct(InterfaceMember), FULL_TYPED, EXPLICIT),
            profile(TypeMember(Class), FULL_TYPED, EXPLICIT),
            profile(TypeMember(Struct), FULL_TYPED, EXPLICIT),
            profile(TypeMember(Interface), FULL_TYPED, EXPLICIT),
            profile(TypeMember(Trait), FULL_TYPED, EXPLICIT),
        ],
    );
    assert_matrix(
        adapter_for("csharp").as_ref(),
        &[
            profile(Direct(Member), FULL_TYPED, EXPLICIT),
            profile(TypeMember(Class), FULL_TYPED, EXPLICIT),
            profile(TypeMember(Struct), FULL_TYPED, EXPLICIT),
        ],
    );
    assert_matrix(
        adapter_for("dart").as_ref(),
        &[
            profile(Direct(Member), DYNAMIC_FULL, &[]),
            profile(TypeMember(Class), DYNAMIC_FULL, &[]),
            profile(TypeMember(Struct), DYNAMIC_FULL, &[]),
        ],
    );
    assert_matrix(
        adapter_for("go").as_ref(),
        &[
            profile(Direct(Member), GO_FIELDS, EXPLICIT),
            profile(TypeMember(Struct), GO_FIELDS, EXPLICIT),
            profile(TypeMember(Class), GO_FIELDS, EXPLICIT),
        ],
    );
    assert_matrix(
        adapter_for("haskell").as_ref(),
        &[
            profile(Direct(Member), IMMUTABLE_FIELDS, EXPLICIT),
            profile(TypeMember(Struct), IMMUTABLE_FIELDS, EXPLICIT),
            profile(TypeMember(Class), IMMUTABLE_FIELDS, EXPLICIT),
            profile(VariantRecordPayload(Enum), IMMUTABLE_FIELDS, EXPLICIT),
        ],
    );
    assert_matrix(
        adapter_for("java").as_ref(),
        &[
            profile(Direct(Member), FULL_TYPED, EXPLICIT),
            profile(TypeMember(Class), FULL_TYPED, EXPLICIT),
            profile(TypeMember(Struct), FULL_TYPED, EXPLICIT),
            profile(TypeMember(Enum), FULL_TYPED, EXPLICIT),
        ],
    );
    assert_matrix(
        adapter_for("javascript").as_ref(),
        &[
            profile(Direct(Member), JS_FIELDS, &[]),
            profile(Direct(InterfaceMember), JS_FIELDS, &[]),
            profile(TypeMember(Class), JS_FIELDS, &[]),
            profile(TypeMember(Struct), JS_FIELDS, &[]),
            profile(TypeMember(Interface), JS_FIELDS, &[]),
            profile(TypeMember(Trait), JS_FIELDS, &[]),
            profile(TypeMember(Enum), JS_FIELDS, &[]),
        ],
    );
    assert_matrix(
        adapter_for("kotlin").as_ref(),
        &[
            profile(Direct(Member), VALUE_FIELDS, &[]),
            profile(TypeMember(Class), VALUE_FIELDS, &[]),
            profile(TypeMember(Struct), VALUE_FIELDS, &[]),
            profile(TypeMember(Enum), VALUE_FIELDS, &[]),
        ],
    );
    assert_matrix(
        adapter_for("ocaml").as_ref(),
        &[
            profile(Direct(Member), IMMUTABLE_FIELDS, EXPLICIT),
            profile(TypeMember(Struct), IMMUTABLE_FIELDS, EXPLICIT),
            profile(TypeMember(Class), IMMUTABLE_FIELDS, EXPLICIT),
            profile(VariantRecordPayload(Enum), IMMUTABLE_FIELDS, EXPLICIT),
        ],
    );
    assert_matrix(
        adapter_for("php").as_ref(),
        &[
            profile(Direct(Member), FULL_TYPED, &[]),
            profile(TypeMember(Class), FULL_TYPED, &[]),
            profile(TypeMember(Struct), FULL_TYPED, &[]),
            profile(TypeMember(Trait), FULL_TYPED, &[]),
        ],
    );
    assert_matrix(
        adapter_for("python").as_ref(),
        &[
            profile(Direct(Member), PYTHON_FIELDS, &[]),
            profile(Direct(InterfaceMember), PYTHON_FIELDS, &[]),
            profile(TypeMember(Class), PYTHON_FIELDS, &[]),
            profile(TypeMember(Struct), PYTHON_FIELDS, &[]),
            profile(TypeMember(Interface), PYTHON_FIELDS, &[]),
            profile(TypeMember(Trait), PYTHON_FIELDS, &[]),
        ],
    );
    assert_matrix(
        adapter_for("rust").as_ref(),
        &[
            profile(Direct(Member), RUST_FIELDS, EXPLICIT),
            profile(TypeMember(Struct), RUST_FIELDS, EXPLICIT),
            profile(TypeMember(Class), RUST_FIELDS, EXPLICIT),
            profile(VariantRecordPayload(Enum), RUST_FIELDS, EXPLICIT),
        ],
    );
    assert_matrix(
        adapter_for("scala").as_ref(),
        &[
            profile(Direct(Member), VALUE_FIELDS, &[]),
            profile(TypeMember(Class), VALUE_FIELDS, &[]),
            profile(TypeMember(Struct), VALUE_FIELDS, &[]),
            profile(TypeMember(Enum), VALUE_FIELDS, &[]),
        ],
    );
    assert_matrix(
        adapter_for("swift").as_ref(),
        &[
            profile(Direct(Member), DYNAMIC_FULL, &[]),
            profile(TypeMember(Class), DYNAMIC_FULL, &[]),
            profile(TypeMember(Struct), DYNAMIC_FULL, &[]),
        ],
    );
    assert_matrix(
        adapter_for("typescript").as_ref(),
        &[
            profile(Direct(Member), TS_CONCRETE, &[]),
            profile(Direct(InterfaceMember), TS_CONTRACT, EXPLICIT),
            profile(TypeMember(Class), TS_CONCRETE, &[]),
            profile(TypeMember(Struct), TS_CONCRETE, &[]),
            profile(TypeMember(Interface), TS_CONTRACT, EXPLICIT),
            profile(TypeMember(Trait), TS_CONTRACT, EXPLICIT),
        ],
    );
}

#[test]
fn shell_lua_and_ruby_have_no_field_profiles() {
    const UNSUPPORTED: &[&str] = &["bash", "zsh", "lua", "ruby"];

    for language in BUILT_IN_LANGUAGES
        .into_iter()
        .filter(|language| UNSUPPORTED.contains(&language.id))
    {
        assert_matrix(language.adapter().as_ref(), &[]);
    }
}
