use sigil_stitch::lang::CodeLang;
use sigil_stitch::lang::capability::{
    FunctionBodyPolicy, FunctionCapability, FunctionContext, FunctionForm,
};

#[path = "shared/languages.rs"]
mod languages_registry;

use languages_registry::adapter_for;

const ALL_CONTEXTS: [FunctionContext; 4] = [
    FunctionContext::TopLevel,
    FunctionContext::ReceiverMethod,
    FunctionContext::Member,
    FunctionContext::InterfaceMember,
];

const ALL_FORMS: [FunctionForm; 3] = [
    FunctionForm::Function,
    FunctionForm::Constructor,
    FunctionForm::Destructor,
];

const ALL_CAPABILITIES: [FunctionCapability; 17] = [
    FunctionCapability::ParametricPolymorphism,
    FunctionCapability::BoundedPolymorphism,
    FunctionCapability::HigherKindedPolymorphism,
    FunctionCapability::Attributes,
    FunctionCapability::ExplicitReturnType,
    FunctionCapability::TypedParameters,
    FunctionCapability::AsyncEffect,
    FunctionCapability::StaticFunction,
    FunctionCapability::StaticMethod,
    FunctionCapability::StaticConstructor,
    FunctionCapability::VirtualMethod,
    FunctionCapability::AbstractMethod,
    FunctionCapability::Override,
    FunctionCapability::ConstructorDelegation,
    FunctionCapability::DefaultParameters,
    FunctionCapability::VariadicParameters,
    FunctionCapability::ConstructorProperties,
];

type ExpectedProfile = (FunctionContext, FunctionForm, &'static [FunctionCapability]);

const REQUIRED_TYPES: &[FunctionCapability] = &[
    FunctionCapability::ExplicitReturnType,
    FunctionCapability::TypedParameters,
];
const REQUIRED_PARAMETERS: &[FunctionCapability] = &[FunctionCapability::TypedParameters];

fn assert_function_matrix(lang: &dyn CodeLang, expected: &[ExpectedProfile]) {
    let actual = lang.capabilities();
    for context in ALL_CONTEXTS {
        assert_eq!(
            actual.supports_function_context(context),
            expected
                .iter()
                .any(|(candidate_context, _, _)| *candidate_context == context),
            "{}.context {context:?}",
            lang.file_extension()
        );
        for form in ALL_FORMS {
            let expected_capabilities = expected
                .iter()
                .find(|(candidate_context, candidate_form, _)| {
                    *candidate_context == context && *candidate_form == form
                })
                .map(|(_, _, capabilities)| *capabilities);
            assert_eq!(
                actual.supports_function_form(context, form),
                expected_capabilities.is_some(),
                "{}.context {context:?} form {form:?}",
                lang.file_extension()
            );
            for capability in ALL_CAPABILITIES {
                assert_eq!(
                    actual.supports_function_capability(context, form, capability),
                    expected_capabilities.is_some_and(|items| items.contains(&capability)),
                    "{}.context {context:?} form {form:?} {capability:?}",
                    lang.file_extension()
                );
            }
        }
    }
}

fn assert_profile_metadata(
    lang: &dyn CodeLang,
    context: FunctionContext,
    form: FunctionForm,
    required: &[FunctionCapability],
    body_policy: FunctionBodyPolicy,
    incompatible: &[(FunctionCapability, FunctionCapability)],
    maximum_parameters: Option<usize>,
) {
    let actual = lang.capabilities();
    assert_eq!(
        actual.required_function_capabilities(context, form),
        required,
        "{}.context {context:?} form {form:?} required capabilities",
        lang.file_extension()
    );
    assert_eq!(
        actual.function_body_policy(context, form),
        body_policy,
        "{}.context {context:?} form {form:?} body policy",
        lang.file_extension()
    );
    assert_eq!(
        actual.incompatible_function_capabilities(context, form),
        incompatible,
        "{}.context {context:?} form {form:?} incompatible capabilities",
        lang.file_extension()
    );
    assert_eq!(
        actual.maximum_function_parameters(context, form),
        maximum_parameters,
        "{}.context {context:?} form {form:?} maximum parameters",
        lang.file_extension()
    );
}

#[test]
fn profile_metadata_matrix() {
    use FunctionBodyPolicy::{Forbidden, Optional, Required};
    use FunctionCapability::{
        AbstractMethod, AsyncEffect, DefaultParameters, Override, StaticMethod, VirtualMethod,
    };
    use FunctionContext::{InterfaceMember, Member, ReceiverMethod, TopLevel};
    use FunctionForm::{Constructor, Destructor, Function};

    let no_required: &[FunctionCapability] = &[];
    let none: &[(FunctionCapability, FunctionCapability)] = &[];
    let cpp_incompatible = &[(StaticMethod, VirtualMethod), (StaticMethod, Override)];
    let csharp_incompatible = &[
        (AbstractMethod, AsyncEffect),
        (AbstractMethod, StaticMethod),
        (StaticMethod, Override),
    ];
    let dart_incompatible = &[
        (AbstractMethod, AsyncEffect),
        (AbstractMethod, StaticMethod),
    ];
    let java_incompatible = &[(AbstractMethod, StaticMethod), (StaticMethod, Override)];
    let swift_incompatible = &[(StaticMethod, Override)];
    let typescript_incompatible = &[
        (AbstractMethod, AsyncEffect),
        (AbstractMethod, StaticMethod),
        (AbstractMethod, DefaultParameters),
    ];

    let c = adapter_for("c");
    assert_profile_metadata(
        c.as_ref(),
        TopLevel,
        Function,
        REQUIRED_TYPES,
        Optional,
        none,
        None,
    );

    let cpp = adapter_for("cpp");
    assert_profile_metadata(
        cpp.as_ref(),
        TopLevel,
        Function,
        REQUIRED_TYPES,
        Optional,
        none,
        None,
    );
    assert_profile_metadata(
        cpp.as_ref(),
        Member,
        Function,
        REQUIRED_TYPES,
        Optional,
        cpp_incompatible,
        None,
    );
    assert_profile_metadata(
        cpp.as_ref(),
        Member,
        Constructor,
        REQUIRED_PARAMETERS,
        Optional,
        none,
        None,
    );
    assert_profile_metadata(
        cpp.as_ref(),
        InterfaceMember,
        Function,
        REQUIRED_TYPES,
        Optional,
        cpp_incompatible,
        None,
    );
    assert_profile_metadata(
        cpp.as_ref(),
        InterfaceMember,
        Constructor,
        REQUIRED_PARAMETERS,
        Optional,
        none,
        None,
    );
    assert_profile_metadata(
        cpp.as_ref(),
        Member,
        Destructor,
        no_required,
        Optional,
        none,
        Some(0),
    );
    assert_profile_metadata(
        cpp.as_ref(),
        InterfaceMember,
        Destructor,
        no_required,
        Optional,
        none,
        Some(0),
    );

    let csharp = adapter_for("csharp");
    assert_profile_metadata(
        csharp.as_ref(),
        Member,
        Function,
        REQUIRED_TYPES,
        Required,
        csharp_incompatible,
        None,
    );
    assert_profile_metadata(
        csharp.as_ref(),
        Member,
        Constructor,
        REQUIRED_PARAMETERS,
        Required,
        none,
        None,
    );
    assert_profile_metadata(
        csharp.as_ref(),
        InterfaceMember,
        Function,
        REQUIRED_TYPES,
        Optional,
        none,
        None,
    );

    let dart = adapter_for("dart");
    assert_profile_metadata(
        dart.as_ref(),
        TopLevel,
        Function,
        no_required,
        Required,
        none,
        None,
    );
    assert_profile_metadata(
        dart.as_ref(),
        Member,
        Function,
        no_required,
        Required,
        dart_incompatible,
        None,
    );
    assert_profile_metadata(
        dart.as_ref(),
        InterfaceMember,
        Function,
        no_required,
        Optional,
        dart_incompatible,
        None,
    );
    assert_profile_metadata(
        dart.as_ref(),
        InterfaceMember,
        Constructor,
        no_required,
        Optional,
        none,
        None,
    );

    let go = adapter_for("go");
    assert_profile_metadata(
        go.as_ref(),
        TopLevel,
        Function,
        REQUIRED_PARAMETERS,
        Required,
        none,
        None,
    );
    assert_profile_metadata(
        go.as_ref(),
        ReceiverMethod,
        Function,
        REQUIRED_PARAMETERS,
        Required,
        none,
        None,
    );
    assert_profile_metadata(
        go.as_ref(),
        InterfaceMember,
        Function,
        REQUIRED_PARAMETERS,
        Forbidden,
        none,
        None,
    );

    let haskell = adapter_for("haskell");
    assert_profile_metadata(
        haskell.as_ref(),
        TopLevel,
        Function,
        no_required,
        Required,
        none,
        None,
    );
    assert_profile_metadata(
        haskell.as_ref(),
        Member,
        Function,
        no_required,
        Required,
        none,
        None,
    );
    assert_profile_metadata(
        haskell.as_ref(),
        InterfaceMember,
        Function,
        REQUIRED_TYPES,
        Optional,
        none,
        None,
    );

    let java = adapter_for("java");
    assert_profile_metadata(
        java.as_ref(),
        Member,
        Function,
        REQUIRED_TYPES,
        Required,
        java_incompatible,
        None,
    );
    assert_profile_metadata(
        java.as_ref(),
        Member,
        Constructor,
        REQUIRED_PARAMETERS,
        Required,
        none,
        None,
    );
    assert_profile_metadata(
        java.as_ref(),
        InterfaceMember,
        Function,
        REQUIRED_TYPES,
        Optional,
        java_incompatible,
        None,
    );

    let javascript = adapter_for("javascript");
    for (context, form) in [
        (TopLevel, Function),
        (Member, Function),
        (Member, Constructor),
        (InterfaceMember, Function),
        (InterfaceMember, Constructor),
    ] {
        assert_profile_metadata(
            javascript.as_ref(),
            context,
            form,
            no_required,
            Required,
            none,
            None,
        );
    }

    let kotlin = adapter_for("kotlin");
    for (context, form, body) in [
        (TopLevel, Function, Required),
        (Member, Function, Required),
        (Member, Constructor, Required),
        (InterfaceMember, Function, Optional),
    ] {
        assert_profile_metadata(
            kotlin.as_ref(),
            context,
            form,
            REQUIRED_PARAMETERS,
            body,
            none,
            None,
        );
    }

    let ocaml = adapter_for("ocaml");
    assert_profile_metadata(
        ocaml.as_ref(),
        TopLevel,
        Function,
        no_required,
        Required,
        none,
        None,
    );

    let php = adapter_for("php");
    for (context, form, body) in [
        (TopLevel, Function, Required),
        (Member, Function, Required),
        (Member, Constructor, Required),
        (InterfaceMember, Function, Forbidden),
        (InterfaceMember, Constructor, Forbidden),
    ] {
        assert_profile_metadata(php.as_ref(), context, form, no_required, body, none, None);
    }

    let ruby = adapter_for("ruby");
    for context in [TopLevel, Member, InterfaceMember] {
        assert_profile_metadata(
            ruby.as_ref(),
            context,
            Function,
            no_required,
            Required,
            none,
            None,
        );
    }

    let rust = adapter_for("rust");
    for (context, form, body) in [
        (TopLevel, Function, Required),
        (TopLevel, Constructor, Required),
        (Member, Function, Required),
        (Member, Constructor, Required),
        (InterfaceMember, Function, Optional),
        (InterfaceMember, Constructor, Optional),
    ] {
        assert_profile_metadata(
            rust.as_ref(),
            context,
            form,
            REQUIRED_PARAMETERS,
            body,
            none,
            None,
        );
    }

    let scala = adapter_for("scala");
    for (context, body) in [
        (TopLevel, Required),
        (Member, Required),
        (InterfaceMember, Optional),
    ] {
        assert_profile_metadata(
            scala.as_ref(),
            context,
            Function,
            REQUIRED_PARAMETERS,
            body,
            none,
            None,
        );
    }

    let swift = adapter_for("swift");
    assert_profile_metadata(
        swift.as_ref(),
        TopLevel,
        Function,
        REQUIRED_PARAMETERS,
        Required,
        none,
        None,
    );
    assert_profile_metadata(
        swift.as_ref(),
        Member,
        Function,
        REQUIRED_PARAMETERS,
        Required,
        swift_incompatible,
        None,
    );
    assert_profile_metadata(
        swift.as_ref(),
        Member,
        Constructor,
        REQUIRED_PARAMETERS,
        Required,
        none,
        None,
    );
    assert_profile_metadata(
        swift.as_ref(),
        InterfaceMember,
        Function,
        REQUIRED_PARAMETERS,
        Forbidden,
        none,
        None,
    );
    assert_profile_metadata(
        swift.as_ref(),
        InterfaceMember,
        Constructor,
        REQUIRED_PARAMETERS,
        Forbidden,
        none,
        None,
    );

    let typescript = adapter_for("typescript");
    assert_profile_metadata(
        typescript.as_ref(),
        TopLevel,
        Function,
        no_required,
        Required,
        none,
        None,
    );
    assert_profile_metadata(
        typescript.as_ref(),
        Member,
        Function,
        no_required,
        Required,
        typescript_incompatible,
        None,
    );
    assert_profile_metadata(
        typescript.as_ref(),
        Member,
        Constructor,
        no_required,
        Required,
        none,
        None,
    );
    assert_profile_metadata(
        typescript.as_ref(),
        InterfaceMember,
        Function,
        no_required,
        Forbidden,
        none,
        None,
    );

    for id in ["bash", "zsh"] {
        let lang = adapter_for(id);
        assert_profile_metadata(
            lang.as_ref(),
            TopLevel,
            Function,
            no_required,
            Required,
            none,
            Some(0),
        );
    }
    let lua = adapter_for("lua");
    assert_profile_metadata(
        lua.as_ref(),
        TopLevel,
        Function,
        no_required,
        Required,
        none,
        None,
    );
}

#[test]
fn shell_and_lua_matrices() {
    let expected = &[(FunctionContext::TopLevel, FunctionForm::Function, &[][..])];
    assert_function_matrix(adapter_for("bash").as_ref(), expected);
    assert_function_matrix(adapter_for("zsh").as_ref(), expected);
    assert_function_matrix(adapter_for("lua").as_ref(), expected);
}

#[test]
fn c_matrix() {
    assert_function_matrix(
        adapter_for("c").as_ref(),
        &[(
            FunctionContext::TopLevel,
            FunctionForm::Function,
            &[
                FunctionCapability::Attributes,
                FunctionCapability::ExplicitReturnType,
                FunctionCapability::TypedParameters,
                FunctionCapability::StaticFunction,
            ],
        )],
    );
}

#[test]
fn cpp_matrix() {
    let member = &[
        FunctionCapability::Attributes,
        FunctionCapability::DefaultParameters,
        FunctionCapability::ExplicitReturnType,
        FunctionCapability::TypedParameters,
        FunctionCapability::Override,
        FunctionCapability::StaticMethod,
        FunctionCapability::VirtualMethod,
    ];
    assert_function_matrix(
        adapter_for("cpp").as_ref(),
        &[
            (
                FunctionContext::TopLevel,
                FunctionForm::Function,
                &[
                    FunctionCapability::Attributes,
                    FunctionCapability::DefaultParameters,
                    FunctionCapability::ExplicitReturnType,
                    FunctionCapability::TypedParameters,
                    FunctionCapability::StaticFunction,
                ],
            ),
            (FunctionContext::Member, FunctionForm::Function, member),
            (
                FunctionContext::Member,
                FunctionForm::Constructor,
                &[
                    FunctionCapability::Attributes,
                    FunctionCapability::ConstructorDelegation,
                    FunctionCapability::DefaultParameters,
                    FunctionCapability::TypedParameters,
                ],
            ),
            (
                FunctionContext::InterfaceMember,
                FunctionForm::Function,
                member,
            ),
            (
                FunctionContext::InterfaceMember,
                FunctionForm::Constructor,
                &[
                    FunctionCapability::Attributes,
                    FunctionCapability::ConstructorDelegation,
                    FunctionCapability::DefaultParameters,
                    FunctionCapability::TypedParameters,
                ],
            ),
            (
                FunctionContext::Member,
                FunctionForm::Destructor,
                &[
                    FunctionCapability::Attributes,
                    FunctionCapability::Override,
                    FunctionCapability::VirtualMethod,
                ],
            ),
            (
                FunctionContext::InterfaceMember,
                FunctionForm::Destructor,
                &[
                    FunctionCapability::Attributes,
                    FunctionCapability::Override,
                    FunctionCapability::VirtualMethod,
                ],
            ),
        ],
    );
}

#[test]
fn csharp_matrix() {
    assert_function_matrix(
        adapter_for("csharp").as_ref(),
        &[
            (
                FunctionContext::Member,
                FunctionForm::Function,
                &[
                    FunctionCapability::AbstractMethod,
                    FunctionCapability::AsyncEffect,
                    FunctionCapability::Attributes,
                    FunctionCapability::BoundedPolymorphism,
                    FunctionCapability::DefaultParameters,
                    FunctionCapability::ExplicitReturnType,
                    FunctionCapability::TypedParameters,
                    FunctionCapability::Override,
                    FunctionCapability::ParametricPolymorphism,
                    FunctionCapability::StaticMethod,
                ],
            ),
            (
                FunctionContext::Member,
                FunctionForm::Constructor,
                &[
                    FunctionCapability::Attributes,
                    FunctionCapability::DefaultParameters,
                    FunctionCapability::StaticConstructor,
                    FunctionCapability::TypedParameters,
                ],
            ),
            (
                FunctionContext::InterfaceMember,
                FunctionForm::Function,
                &[
                    FunctionCapability::AbstractMethod,
                    FunctionCapability::Attributes,
                    FunctionCapability::BoundedPolymorphism,
                    FunctionCapability::DefaultParameters,
                    FunctionCapability::ExplicitReturnType,
                    FunctionCapability::TypedParameters,
                    FunctionCapability::ParametricPolymorphism,
                    FunctionCapability::StaticMethod,
                ],
            ),
        ],
    );
}

#[test]
fn dart_matrix() {
    let member = &[
        FunctionCapability::AbstractMethod,
        FunctionCapability::AsyncEffect,
        FunctionCapability::Attributes,
        FunctionCapability::BoundedPolymorphism,
        FunctionCapability::ExplicitReturnType,
        FunctionCapability::TypedParameters,
        FunctionCapability::ParametricPolymorphism,
        FunctionCapability::StaticMethod,
    ];
    assert_function_matrix(
        adapter_for("dart").as_ref(),
        &[
            (
                FunctionContext::TopLevel,
                FunctionForm::Function,
                &[
                    FunctionCapability::AsyncEffect,
                    FunctionCapability::Attributes,
                    FunctionCapability::BoundedPolymorphism,
                    FunctionCapability::ExplicitReturnType,
                    FunctionCapability::TypedParameters,
                    FunctionCapability::ParametricPolymorphism,
                ],
            ),
            (FunctionContext::Member, FunctionForm::Function, member),
            (
                FunctionContext::Member,
                FunctionForm::Constructor,
                &[
                    FunctionCapability::Attributes,
                    FunctionCapability::ConstructorDelegation,
                    FunctionCapability::TypedParameters,
                ],
            ),
            (
                FunctionContext::InterfaceMember,
                FunctionForm::Function,
                member,
            ),
            (
                FunctionContext::InterfaceMember,
                FunctionForm::Constructor,
                &[
                    FunctionCapability::Attributes,
                    FunctionCapability::ConstructorDelegation,
                    FunctionCapability::TypedParameters,
                ],
            ),
        ],
    );
}

#[test]
fn go_matrix() {
    assert_function_matrix(
        adapter_for("go").as_ref(),
        &[
            (
                FunctionContext::TopLevel,
                FunctionForm::Function,
                &[
                    FunctionCapability::BoundedPolymorphism,
                    FunctionCapability::ExplicitReturnType,
                    FunctionCapability::TypedParameters,
                    FunctionCapability::ParametricPolymorphism,
                ],
            ),
            (
                FunctionContext::ReceiverMethod,
                FunctionForm::Function,
                &[
                    FunctionCapability::ExplicitReturnType,
                    FunctionCapability::TypedParameters,
                ],
            ),
            (
                FunctionContext::InterfaceMember,
                FunctionForm::Function,
                &[
                    FunctionCapability::ExplicitReturnType,
                    FunctionCapability::TypedParameters,
                ],
            ),
        ],
    );
}

#[test]
fn haskell_matrix() {
    let capabilities = &[
        FunctionCapability::BoundedPolymorphism,
        FunctionCapability::ExplicitReturnType,
        FunctionCapability::TypedParameters,
        FunctionCapability::ParametricPolymorphism,
    ];
    assert_function_matrix(
        adapter_for("haskell").as_ref(),
        &[
            (
                FunctionContext::TopLevel,
                FunctionForm::Function,
                capabilities,
            ),
            (
                FunctionContext::Member,
                FunctionForm::Function,
                capabilities,
            ),
            (
                FunctionContext::InterfaceMember,
                FunctionForm::Function,
                capabilities,
            ),
        ],
    );
}

#[test]
fn java_matrix() {
    let member = &[
        FunctionCapability::AbstractMethod,
        FunctionCapability::Attributes,
        FunctionCapability::BoundedPolymorphism,
        FunctionCapability::ExplicitReturnType,
        FunctionCapability::TypedParameters,
        FunctionCapability::Override,
        FunctionCapability::ParametricPolymorphism,
        FunctionCapability::StaticMethod,
    ];
    assert_function_matrix(
        adapter_for("java").as_ref(),
        &[
            (FunctionContext::Member, FunctionForm::Function, member),
            (
                FunctionContext::Member,
                FunctionForm::Constructor,
                &[
                    FunctionCapability::Attributes,
                    FunctionCapability::BoundedPolymorphism,
                    FunctionCapability::ConstructorDelegation,
                    FunctionCapability::ParametricPolymorphism,
                    FunctionCapability::TypedParameters,
                ],
            ),
            (
                FunctionContext::InterfaceMember,
                FunctionForm::Function,
                member,
            ),
        ],
    );
}

#[test]
fn javascript_matrix() {
    let member = &[
        FunctionCapability::AsyncEffect,
        FunctionCapability::DefaultParameters,
        FunctionCapability::StaticMethod,
        FunctionCapability::VariadicParameters,
    ];
    assert_function_matrix(
        adapter_for("javascript").as_ref(),
        &[
            (
                FunctionContext::TopLevel,
                FunctionForm::Function,
                &[
                    FunctionCapability::AsyncEffect,
                    FunctionCapability::DefaultParameters,
                    FunctionCapability::VariadicParameters,
                ],
            ),
            (FunctionContext::Member, FunctionForm::Function, member),
            (
                FunctionContext::Member,
                FunctionForm::Constructor,
                &[
                    FunctionCapability::ConstructorDelegation,
                    FunctionCapability::DefaultParameters,
                    FunctionCapability::VariadicParameters,
                ],
            ),
            (
                FunctionContext::InterfaceMember,
                FunctionForm::Function,
                member,
            ),
            (
                FunctionContext::InterfaceMember,
                FunctionForm::Constructor,
                &[
                    FunctionCapability::ConstructorDelegation,
                    FunctionCapability::DefaultParameters,
                    FunctionCapability::VariadicParameters,
                ],
            ),
        ],
    );
}

#[test]
fn kotlin_matrix() {
    let top = &[
        FunctionCapability::Attributes,
        FunctionCapability::AsyncEffect,
        FunctionCapability::Attributes,
        FunctionCapability::BoundedPolymorphism,
        FunctionCapability::DefaultParameters,
        FunctionCapability::ExplicitReturnType,
        FunctionCapability::TypedParameters,
        FunctionCapability::ParametricPolymorphism,
    ];
    let member = &[
        FunctionCapability::AbstractMethod,
        FunctionCapability::Attributes,
        FunctionCapability::AsyncEffect,
        FunctionCapability::Attributes,
        FunctionCapability::BoundedPolymorphism,
        FunctionCapability::DefaultParameters,
        FunctionCapability::ExplicitReturnType,
        FunctionCapability::TypedParameters,
        FunctionCapability::Override,
        FunctionCapability::ParametricPolymorphism,
    ];
    assert_function_matrix(
        adapter_for("kotlin").as_ref(),
        &[
            (FunctionContext::TopLevel, FunctionForm::Function, top),
            (FunctionContext::Member, FunctionForm::Function, member),
            (
                FunctionContext::Member,
                FunctionForm::Constructor,
                &[
                    FunctionCapability::Attributes,
                    FunctionCapability::ConstructorDelegation,
                    FunctionCapability::DefaultParameters,
                    FunctionCapability::TypedParameters,
                ],
            ),
            (
                FunctionContext::InterfaceMember,
                FunctionForm::Function,
                member,
            ),
        ],
    );
}

#[test]
fn ocaml_matrix() {
    assert_function_matrix(
        adapter_for("ocaml").as_ref(),
        &[(
            FunctionContext::TopLevel,
            FunctionForm::Function,
            &[
                FunctionCapability::ExplicitReturnType,
                FunctionCapability::TypedParameters,
            ],
        )],
    );
}

#[test]
fn php_matrix() {
    assert_function_matrix(
        adapter_for("php").as_ref(),
        &[
            (
                FunctionContext::TopLevel,
                FunctionForm::Function,
                &[
                    FunctionCapability::Attributes,
                    FunctionCapability::DefaultParameters,
                    FunctionCapability::ExplicitReturnType,
                    FunctionCapability::TypedParameters,
                ],
            ),
            (
                FunctionContext::Member,
                FunctionForm::Function,
                &[
                    FunctionCapability::AbstractMethod,
                    FunctionCapability::Attributes,
                    FunctionCapability::DefaultParameters,
                    FunctionCapability::ExplicitReturnType,
                    FunctionCapability::TypedParameters,
                    FunctionCapability::Override,
                    FunctionCapability::StaticMethod,
                ],
            ),
            (
                FunctionContext::Member,
                FunctionForm::Constructor,
                &[
                    FunctionCapability::AbstractMethod,
                    FunctionCapability::Attributes,
                    FunctionCapability::DefaultParameters,
                    FunctionCapability::TypedParameters,
                ],
            ),
            (
                FunctionContext::InterfaceMember,
                FunctionForm::Function,
                &[
                    FunctionCapability::Attributes,
                    FunctionCapability::DefaultParameters,
                    FunctionCapability::ExplicitReturnType,
                    FunctionCapability::TypedParameters,
                    FunctionCapability::Override,
                    FunctionCapability::StaticMethod,
                ],
            ),
            (
                FunctionContext::InterfaceMember,
                FunctionForm::Constructor,
                &[
                    FunctionCapability::Attributes,
                    FunctionCapability::DefaultParameters,
                    FunctionCapability::TypedParameters,
                ],
            ),
        ],
    );
}

#[test]
fn python_matrix() {
    let function = &[
        FunctionCapability::AsyncEffect,
        FunctionCapability::Attributes,
        FunctionCapability::DefaultParameters,
        FunctionCapability::ExplicitReturnType,
        FunctionCapability::TypedParameters,
    ];
    let member_function = &[
        FunctionCapability::AsyncEffect,
        FunctionCapability::Attributes,
        FunctionCapability::DefaultParameters,
        FunctionCapability::ExplicitReturnType,
        FunctionCapability::TypedParameters,
        FunctionCapability::StaticMethod,
    ];
    let constructor = &[
        FunctionCapability::Attributes,
        FunctionCapability::ConstructorDelegation,
        FunctionCapability::DefaultParameters,
        FunctionCapability::ExplicitReturnType,
        FunctionCapability::TypedParameters,
    ];
    assert_function_matrix(
        adapter_for("python").as_ref(),
        &[
            (FunctionContext::TopLevel, FunctionForm::Function, function),
            (
                FunctionContext::Member,
                FunctionForm::Function,
                member_function,
            ),
            (
                FunctionContext::Member,
                FunctionForm::Constructor,
                constructor,
            ),
            (
                FunctionContext::InterfaceMember,
                FunctionForm::Function,
                member_function,
            ),
            (
                FunctionContext::InterfaceMember,
                FunctionForm::Constructor,
                constructor,
            ),
        ],
    );
}

#[test]
fn ruby_matrix() {
    let capabilities = &[FunctionCapability::DefaultParameters];
    assert_function_matrix(
        adapter_for("ruby").as_ref(),
        &[
            (
                FunctionContext::TopLevel,
                FunctionForm::Function,
                capabilities,
            ),
            (
                FunctionContext::Member,
                FunctionForm::Function,
                capabilities,
            ),
            (
                FunctionContext::InterfaceMember,
                FunctionForm::Function,
                capabilities,
            ),
        ],
    );
}

#[test]
fn rust_matrix() {
    let capabilities = &[
        FunctionCapability::AsyncEffect,
        FunctionCapability::Attributes,
        FunctionCapability::BoundedPolymorphism,
        FunctionCapability::ExplicitReturnType,
        FunctionCapability::TypedParameters,
        FunctionCapability::ParametricPolymorphism,
    ];
    assert_function_matrix(
        adapter_for("rust").as_ref(),
        &[
            (
                FunctionContext::TopLevel,
                FunctionForm::Function,
                capabilities,
            ),
            (
                FunctionContext::TopLevel,
                FunctionForm::Constructor,
                capabilities,
            ),
            (
                FunctionContext::Member,
                FunctionForm::Function,
                capabilities,
            ),
            (
                FunctionContext::Member,
                FunctionForm::Constructor,
                capabilities,
            ),
            (
                FunctionContext::InterfaceMember,
                FunctionForm::Function,
                capabilities,
            ),
            (
                FunctionContext::InterfaceMember,
                FunctionForm::Constructor,
                capabilities,
            ),
        ],
    );
}

#[test]
fn scala_matrix() {
    let top = &[
        FunctionCapability::Attributes,
        FunctionCapability::BoundedPolymorphism,
        FunctionCapability::DefaultParameters,
        FunctionCapability::ExplicitReturnType,
        FunctionCapability::TypedParameters,
        FunctionCapability::ParametricPolymorphism,
        FunctionCapability::HigherKindedPolymorphism,
    ];
    let member = &[
        FunctionCapability::AbstractMethod,
        FunctionCapability::Attributes,
        FunctionCapability::BoundedPolymorphism,
        FunctionCapability::DefaultParameters,
        FunctionCapability::ExplicitReturnType,
        FunctionCapability::TypedParameters,
        FunctionCapability::Override,
        FunctionCapability::ParametricPolymorphism,
        FunctionCapability::HigherKindedPolymorphism,
    ];
    assert_function_matrix(
        adapter_for("scala").as_ref(),
        &[
            (FunctionContext::TopLevel, FunctionForm::Function, top),
            (FunctionContext::Member, FunctionForm::Function, member),
            (
                FunctionContext::InterfaceMember,
                FunctionForm::Function,
                member,
            ),
        ],
    );
}

#[test]
fn swift_matrix() {
    assert_function_matrix(
        adapter_for("swift").as_ref(),
        &[
            (
                FunctionContext::TopLevel,
                FunctionForm::Function,
                &[
                    FunctionCapability::AsyncEffect,
                    FunctionCapability::Attributes,
                    FunctionCapability::BoundedPolymorphism,
                    FunctionCapability::DefaultParameters,
                    FunctionCapability::ExplicitReturnType,
                    FunctionCapability::TypedParameters,
                    FunctionCapability::ParametricPolymorphism,
                ],
            ),
            (
                FunctionContext::Member,
                FunctionForm::Function,
                &[
                    FunctionCapability::AsyncEffect,
                    FunctionCapability::Attributes,
                    FunctionCapability::BoundedPolymorphism,
                    FunctionCapability::DefaultParameters,
                    FunctionCapability::ExplicitReturnType,
                    FunctionCapability::TypedParameters,
                    FunctionCapability::Override,
                    FunctionCapability::ParametricPolymorphism,
                    FunctionCapability::StaticMethod,
                ],
            ),
            (
                FunctionContext::Member,
                FunctionForm::Constructor,
                &[
                    FunctionCapability::AsyncEffect,
                    FunctionCapability::Attributes,
                    FunctionCapability::BoundedPolymorphism,
                    FunctionCapability::ConstructorDelegation,
                    FunctionCapability::DefaultParameters,
                    FunctionCapability::Override,
                    FunctionCapability::ParametricPolymorphism,
                    FunctionCapability::TypedParameters,
                ],
            ),
            (
                FunctionContext::InterfaceMember,
                FunctionForm::Function,
                &[
                    FunctionCapability::AsyncEffect,
                    FunctionCapability::Attributes,
                    FunctionCapability::BoundedPolymorphism,
                    FunctionCapability::ExplicitReturnType,
                    FunctionCapability::ParametricPolymorphism,
                    FunctionCapability::TypedParameters,
                    FunctionCapability::StaticMethod,
                ],
            ),
            (
                FunctionContext::InterfaceMember,
                FunctionForm::Constructor,
                &[
                    FunctionCapability::AsyncEffect,
                    FunctionCapability::Attributes,
                    FunctionCapability::BoundedPolymorphism,
                    FunctionCapability::ParametricPolymorphism,
                    FunctionCapability::TypedParameters,
                ],
            ),
        ],
    );
}

#[test]
fn typescript_matrix() {
    assert_function_matrix(
        adapter_for("typescript").as_ref(),
        &[
            (
                FunctionContext::TopLevel,
                FunctionForm::Function,
                &[
                    FunctionCapability::AsyncEffect,
                    FunctionCapability::BoundedPolymorphism,
                    FunctionCapability::DefaultParameters,
                    FunctionCapability::ExplicitReturnType,
                    FunctionCapability::TypedParameters,
                    FunctionCapability::ParametricPolymorphism,
                    FunctionCapability::VariadicParameters,
                ],
            ),
            (
                FunctionContext::Member,
                FunctionForm::Function,
                &[
                    FunctionCapability::AbstractMethod,
                    FunctionCapability::AsyncEffect,
                    FunctionCapability::Attributes,
                    FunctionCapability::BoundedPolymorphism,
                    FunctionCapability::DefaultParameters,
                    FunctionCapability::ExplicitReturnType,
                    FunctionCapability::TypedParameters,
                    FunctionCapability::Override,
                    FunctionCapability::ParametricPolymorphism,
                    FunctionCapability::StaticMethod,
                    FunctionCapability::VariadicParameters,
                ],
            ),
            (
                FunctionContext::Member,
                FunctionForm::Constructor,
                &[
                    FunctionCapability::ConstructorDelegation,
                    FunctionCapability::ConstructorProperties,
                    FunctionCapability::DefaultParameters,
                    FunctionCapability::TypedParameters,
                    FunctionCapability::VariadicParameters,
                ],
            ),
            (
                FunctionContext::InterfaceMember,
                FunctionForm::Function,
                &[
                    FunctionCapability::BoundedPolymorphism,
                    FunctionCapability::ExplicitReturnType,
                    FunctionCapability::TypedParameters,
                    FunctionCapability::ParametricPolymorphism,
                    FunctionCapability::VariadicParameters,
                ],
            ),
        ],
    );
}
