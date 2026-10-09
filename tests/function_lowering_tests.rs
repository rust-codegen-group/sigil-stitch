use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::error::SigilStitchError;
use sigil_stitch::lang::capability::{
    FunctionCapability, FunctionCapabilityProfile, FunctionContext, FunctionForm,
    LanguageCapabilities,
};
use sigil_stitch::lang::{CodeLang, FunctionIntent, RendererLang, ValidatedFunction};
use sigil_stitch::spec::annotation_spec::{AnnotationNameRef, AnnotationSpec};
use sigil_stitch::spec::emittable::Emittable;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::{DeclarationContext, TypeKind, Visibility};
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
#[allow(
    deprecated,
    reason = "exercise released generic and callable compatibility inputs"
)]
use sigil_stitch::spec::where_spec::{TypeParamKind, TypeParamSpec};
use sigil_stitch::type_name::TypeName;

#[path = "shared/languages.rs"]
mod languages_registry;

#[test]
fn missing_function_lowerers_fail_for_every_return_shape_and_composition_path() {
    #[derive(Debug)]
    struct MissingLang(bool);
    impl RendererLang for MissingLang {
        fn file_extension(&self) -> &str {
            "missing"
        }
        fn line_comment_prefix(&self) -> &str {
            "//"
        }
    }
    impl CodeLang for MissingLang {
        fn capabilities(&self) -> LanguageCapabilities<'_> {
            const PROFILES: &[FunctionCapabilityProfile<'_>] = &[
                FunctionCapabilityProfile::new(
                    FunctionContext::TopLevel,
                    FunctionForm::Function,
                    &[FunctionCapability::ExplicitReturns],
                ),
                FunctionCapabilityProfile::new(
                    FunctionContext::Member,
                    FunctionForm::Function,
                    &[FunctionCapability::ExplicitReturns],
                ),
            ];
            let capabilities = LanguageCapabilities::permissive();
            if self.0 {
                capabilities.with_functions(PROFILES)
            } else {
                capabilities
            }
        }
    }
    for strict in [false, true] {
        for returns in [
            None,
            Some(vec![]),
            Some(vec![TypeName::primitive("int")]),
            Some(vec![
                TypeName::primitive("int"),
                TypeName::primitive("bool"),
            ]),
        ] {
            let mut builder = FunSpec::builder("work");
            if let Some(returns) = returns {
                builder = builder.returns(returns);
            }
            let function = builder.build().unwrap();
            let lang = MissingLang(strict);
            function
                .validate(&lang, DeclarationContext::TopLevel)
                .unwrap();
            assert!(matches!(
                function.emit(&lang, DeclarationContext::TopLevel),
                Err(SigilStitchError::MissingFunctionLowerer { .. })
            ));
            assert!(matches!(
                Emittable::emit_members(&function, &lang),
                Err(SigilStitchError::MissingFunctionLowerer { .. })
            ));
            let owner = TypeSpec::builder("Owner", TypeKind::Class)
                .add_method(function.clone())
                .build()
                .unwrap();
            owner.validate(&lang).unwrap();
            assert!(matches!(
                Emittable::emit_members(&owner, &lang),
                Err(SigilStitchError::MissingFunctionLowerer { .. })
            ));
            for nested in [false, true] {
                let mut builder = FileSpec::builder_with("work.missing", MissingLang(strict));
                builder = if nested {
                    builder.add_type(owner.clone())
                } else {
                    builder.add_function(function.clone())
                };
                let file = builder.build().unwrap();
                file.validate().unwrap();
                assert!(matches!(
                    file.render(120),
                    Err(SigilStitchError::MissingFunctionLowerer { .. })
                ));
            }
        }
    }
}

#[test]
fn explicit_empty_satisfies_required_returns_without_changing_constructor_identity() {
    let method = FunSpec::builder("work").returns(vec![]).build().unwrap();
    method
        .validate(
            &sigil_stitch::lang::java::Java::new(),
            DeclarationContext::InterfaceMember,
        )
        .unwrap();
    let omitted = FunSpec::builder("work").build().unwrap();
    assert!(matches!(
        omitted.validate(
            &sigil_stitch::lang::java::Java::new(),
            DeclarationContext::InterfaceMember
        ),
        Err(SigilStitchError::MissingRequiredFunctionCapabilities { .. })
    ));
    let ordinary = FunSpec::builder("Owner")
        .returns(vec![])
        .body(CodeBlock::of("body", ()).unwrap())
        .build()
        .unwrap();
    TypeSpec::builder("Owner", TypeKind::Class)
        .add_method(ordinary)
        .build()
        .unwrap()
        .validate(&sigil_stitch::lang::java::Java::new())
        .unwrap();
    let constructor = FunSpec::builder("Owner")
        .is_constructor()
        .returns(vec![])
        .body(CodeBlock::of("body", ()).unwrap())
        .build()
        .unwrap();
    assert!(
        constructor
            .validate(
                &sigil_stitch::lang::java::Java::new(),
                DeclarationContext::Member
            )
            .is_err()
    );
}

#[test]
fn returns_preserve_presence_order_and_replacing_setter_after_serde() {
    #[derive(Debug)]
    struct ViewLang(Option<Vec<TypeName>>);
    impl RendererLang for ViewLang {
        fn file_extension(&self) -> &str {
            "view"
        }
        fn line_comment_prefix(&self) -> &str {
            "//"
        }
    }
    impl CodeLang for ViewLang {
        fn validate_function(&self, function: FunctionIntent<'_>) -> Result<(), SigilStitchError> {
            assert_eq!(function.returns(), self.0.as_deref());
            Ok(())
        }
        fn lower_function(
            &self,
            function: ValidatedFunction<'_>,
        ) -> Result<CodeBlock, SigilStitchError> {
            assert_eq!(function.returns(), self.0.as_deref());
            CodeBlock::of("view", ())
        }
    }
    for returns in [
        None,
        Some(vec![]),
        Some(vec![TypeName::primitive("int")]),
        Some(vec![
            TypeName::primitive("int"),
            TypeName::primitive("bool"),
        ]),
    ] {
        let mut builder = FunSpec::builder("work");
        if let Some(returns) = &returns {
            builder = builder.returns(returns.clone());
        }
        let function = builder.build().unwrap();
        let decoded: FunSpec =
            serde_json::from_value(serde_json::to_value(&function).unwrap()).unwrap();
        decoded
            .emit(&ViewLang(returns), DeclarationContext::TopLevel)
            .unwrap();
    }
    let function = FunSpec::builder("work")
        .returns(vec![TypeName::primitive("discarded")])
        .returns(vec![])
        .build()
        .unwrap();
    function
        .emit(&ViewLang(Some(vec![])), DeclarationContext::TopLevel)
        .unwrap();
}

#[test]
fn builtins_validate_return_cardinality_and_native_empty_conventions() {
    for descriptor in languages_registry::BUILT_IN_LANGUAGES {
        let lang = descriptor.adapter();
        let context = if matches!(descriptor.id, "java" | "csharp") {
            DeclarationContext::Member
        } else {
            DeclarationContext::TopLevel
        };
        let function = |returns| {
            FunSpec::builder("work")
                .returns(returns)
                .body(CodeBlock::of("body", ()).unwrap())
                .build()
                .unwrap()
        };
        let native = match descriptor.id {
            "bash" | "zsh" | "javascript" | "lua" | "ruby" => None,
            "go" => Some(""),
            "kotlin" | "scala" => Some("Unit"),
            "swift" => Some("Void"),
            "python" => Some("None"),
            "haskell" | "rust" => Some("()"),
            "ocaml" => Some("unit"),
            _ => Some("void"),
        };
        for returns in [
            vec![],
            vec![TypeName::primitive("Result")],
            vec![TypeName::primitive("First"), TypeName::primitive("Second")],
        ] {
            let spec = function(returns.clone());
            let validated = spec.validate(lang.as_ref(), context);
            if native.is_none() {
                assert!(
                    matches!(validated, Err(SigilStitchError::UnsupportedFunctionCapabilities { capabilities, .. })
                    if capabilities.contains(&FunctionCapability::ExplicitReturns)),
                    "{}",
                    descriptor.id
                );
            } else if returns.len() > 1 && descriptor.id != "go" {
                assert!(
                    matches!(validated, Err(SigilStitchError::UnsupportedFunctionReturns { actual: 2, function_name, .. })
                    if function_name == "work"),
                    "{}",
                    descriptor.id
                );
                assert!(matches!(
                    spec.emit(lang.as_ref(), context),
                    Err(SigilStitchError::UnsupportedFunctionReturns { .. })
                ));
            } else {
                validated.unwrap();
                for width in [8, 120] {
                    render_in_context(&spec, lang.as_ref(), context, width);
                }
            }
        }
        if let Some(native) = native {
            for width in [8, 120] {
                let empty = render_in_context(&function(vec![]), lang.as_ref(), context, width);
                let equivalent = if descriptor.id == "go" {
                    FunSpec::builder("work")
                        .body(CodeBlock::of("body", ()).unwrap())
                        .build()
                        .unwrap()
                } else {
                    function(vec![TypeName::primitive(native)])
                };
                assert_eq!(
                    empty,
                    render_in_context(&equivalent, lang.as_ref(), context, width),
                    "{}",
                    descriptor.id
                );
            }
        }
    }
}

#[test]
fn go_declarations_keep_all_slots_structured_in_each_existing_context() {
    let lang = sigil_stitch::lang::go::Go::new();
    for context in [
        DeclarationContext::TopLevel,
        DeclarationContext::InterfaceMember,
    ] {
        for receiver in [false, true] {
            if receiver && context == DeclarationContext::InterfaceMember {
                continue;
            }
            for returns in [
                vec![],
                vec![TypeName::primitive("int")],
                vec![TypeName::primitive("int"), TypeName::primitive("bool")],
            ] {
                let mut builder = FunSpec::builder("work").returns(returns.clone());
                if receiver {
                    builder =
                        builder.receiver(ParameterSpec::of("self", TypeName::primitive("Owner")));
                }
                if context == DeclarationContext::TopLevel {
                    builder = builder.body(CodeBlock::of("body", ()).unwrap());
                }
                let spec = builder.build().unwrap();
                let block = spec.emit(&lang, context).unwrap();
                // Every result survives declaration emission as a distinct TypeRef.
                fn refs(nodes: &[sigil_stitch::code_node::CodeNode]) -> Vec<TypeName> {
                    nodes
                        .iter()
                        .flat_map(|node| match node {
                            sigil_stitch::code_node::CodeNode::TypeRef(ty) => vec![ty.clone()],
                            sigil_stitch::code_node::CodeNode::Nested(block) => {
                                refs(block.clone().nodes_mut())
                            }
                            sigil_stitch::code_node::CodeNode::Sequence(nodes) => refs(nodes),
                            _ => vec![],
                        })
                        .collect()
                }
                assert!(refs(block.clone().nodes_mut()).ends_with(&returns));
                let output = render_in_context(&spec, &lang, context, 120);
                let clause = match returns.len() {
                    0 => "work()",
                    1 => "work() int",
                    _ => "work() (int, bool)",
                };
                assert!(output.contains(clause), "{output}");
            }
        }
    }
    let tuple = FunSpec::builder("work")
        .returns(vec![TypeName::tuple(vec![
            TypeName::primitive("int"),
            TypeName::primitive("bool"),
        ])])
        .body(CodeBlock::of("body", ()).unwrap())
        .build()
        .unwrap();
    let error = FileSpec::builder("tuple.go")
        .add_function(tuple)
        .build()
        .unwrap()
        .render(120)
        .unwrap_err();
    assert!(matches!(
        error,
        SigilStitchError::UnsupportedTypeName { .. }
    ));
}

#[test]
fn typescript_async_empty_returns_are_structured_promise_void_only_when_empty() {
    let lang = sigil_stitch::lang::typescript::TypeScript::new();
    for (returns, expected) in [
        (vec![], "Promise<void>"),
        (vec![TypeName::primitive("Payload")], "Payload"),
    ] {
        let function = FunSpec::builder("work")
            .is_async()
            .returns(returns)
            .body(CodeBlock::of("body", ()).unwrap())
            .build()
            .unwrap();
        let output = render_in_context(&function, &lang, DeclarationContext::TopLevel, 120);
        assert!(
            output.contains(&format!("async function work(): {expected}")),
            "{output}"
        );
        assert!(!output.contains("Promise<Payload>"));
    }
}

#[test]
fn same_version_function_return_lists_are_revalidated_on_target_selection() {
    let function = FunSpec::builder("work")
        .returns(vec![
            TypeName::primitive("int"),
            TypeName::primitive("bool"),
        ])
        .body(CodeBlock::of("body", ()).unwrap())
        .build()
        .unwrap();
    let decoded: FunSpec =
        serde_json::from_value(serde_json::to_value(&function).unwrap()).unwrap();
    decoded
        .validate(
            &sigil_stitch::lang::go::Go::new(),
            DeclarationContext::TopLevel,
        )
        .unwrap();
    assert!(matches!(
        decoded.validate(
            &sigil_stitch::lang::typescript::TypeScript::new(),
            DeclarationContext::TopLevel
        ),
        Err(SigilStitchError::UnsupportedFunctionReturns { actual: 2, .. })
    ));
    let file = FileSpec::builder("invalid.ts")
        .add_function(decoded)
        .build()
        .unwrap();
    assert!(
        matches!(file.validate(), Err(SigilStitchError::FileSpecValidation { errors, .. }) if matches!(errors.as_slice(), [SigilStitchError::UnsupportedFunctionReturns { actual: 2, .. }]))
    );
}

#[test]
fn python_constructor_return_sequence_keeps_existing_diagnostics() {
    let lang = sigil_stitch::lang::python::Python::new();
    for (returns, valid) in [
        (vec![], true),
        (vec![TypeName::primitive("None")], true),
        (vec![TypeName::primitive("str")], false),
    ] {
        let function = FunSpec::builder("__init__")
            .is_constructor()
            .returns(returns)
            .body(CodeBlock::of("pass", ()).unwrap())
            .build()
            .unwrap();
        let result = function.validate(&lang, DeclarationContext::Member);
        if valid {
            result.unwrap();
        } else {
            assert!(matches!(
                result,
                Err(SigilStitchError::InvalidConstructorReturnType { .. })
            ));
        }
    }
    let multiple = FunSpec::builder("__init__")
        .is_constructor()
        .returns(vec![
            TypeName::primitive("None"),
            TypeName::primitive("str"),
        ])
        .body(CodeBlock::of("pass", ()).unwrap())
        .build()
        .unwrap();
    assert!(matches!(
        multiple.validate(&lang, DeclarationContext::Member),
        Err(SigilStitchError::UnsupportedFunctionReturns {
            form: FunctionForm::Constructor,
            ..
        })
    ));
}

#[derive(Debug, Clone)]
struct NovelLang {
    calls: Arc<AtomicUsize>,
    validation_calls: Arc<AtomicUsize>,
    strict: bool,
}

impl NovelLang {
    fn permissive() -> Self {
        Self {
            calls: Arc::new(AtomicUsize::new(0)),
            validation_calls: Arc::new(AtomicUsize::new(0)),
            strict: false,
        }
    }

    fn strict() -> Self {
        Self {
            calls: Arc::new(AtomicUsize::new(0)),
            validation_calls: Arc::new(AtomicUsize::new(0)),
            strict: true,
        }
    }
}

impl RendererLang for NovelLang {
    fn file_extension(&self) -> &str {
        "novel"
    }

    fn line_comment_prefix(&self) -> &str {
        "//"
    }
}

const NOVEL_FUNCTIONS: &[FunctionCapabilityProfile<'_>] = &[FunctionCapabilityProfile::new(
    FunctionContext::TopLevel,
    FunctionForm::Function,
    &[
        FunctionCapability::ParametricPolymorphism,
        FunctionCapability::BoundedPolymorphism,
    ],
)];

impl CodeLang for NovelLang {
    fn capabilities(&self) -> LanguageCapabilities<'_> {
        if self.strict {
            LanguageCapabilities::strict().with_functions(NOVEL_FUNCTIONS)
        } else {
            LanguageCapabilities::permissive()
        }
    }

    fn validate_function(&self, function: FunctionIntent<'_>) -> Result<(), SigilStitchError> {
        self.validation_calls.fetch_add(1, Ordering::SeqCst);
        assert!(!function.name().is_empty());
        Ok(())
    }

    fn lower_function(
        &self,
        function: ValidatedFunction<'_>,
    ) -> Result<CodeBlock, SigilStitchError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let mut block = CodeBlock::builder();
        block.add("declare %L(", function.name().to_string());
        for (index, parameter) in function.parameters().iter().enumerate() {
            if index > 0 {
                block.add(",%W", ());
            }
            block.add("%T", parameter.param_type().clone());
        }
        block.add(")", ());
        block.build()
    }
}

#[derive(Debug)]
struct LegacyLang;

impl RendererLang for LegacyLang {
    fn file_extension(&self) -> &str {
        "legacy-lowering"
    }

    fn line_comment_prefix(&self) -> &str {
        "//"
    }
}

impl CodeLang for LegacyLang {
    fn function_keyword(&self, _context: DeclarationContext) -> &str {
        "legacy_fn"
    }
}

#[derive(Debug)]
struct SemanticViewLang;

impl RendererLang for SemanticViewLang {
    fn file_extension(&self) -> &str {
        "semantic-view"
    }

    fn line_comment_prefix(&self) -> &str {
        "//"
    }
}

impl CodeLang for SemanticViewLang {
    #[expect(
        deprecated,
        reason = "assert the compatibility metadata exposed by the borrowed view"
    )]
    fn lower_function(
        &self,
        function: ValidatedFunction<'_>,
    ) -> Result<CodeBlock, SigilStitchError> {
        assert_eq!(function.name(), "inspect");
        assert_eq!(function.declaration_context(), DeclarationContext::TopLevel);
        assert_eq!(function.function_context(), FunctionContext::ReceiverMethod);
        assert_eq!(function.form(), FunctionForm::Function);

        let receiver = function.receiver().unwrap();
        assert_eq!(receiver.name(), "self");
        assert_eq!(receiver.param_type(), &TypeName::primitive("Receiver"));
        assert!(receiver.default_value().is_none());
        assert!(!receiver.is_variadic());
        assert!(!receiver.is_property());
        assert!(!receiver.is_mutable_property());

        let parameters = function.parameters();
        assert_eq!(parameters.len(), 2);
        assert_eq!(parameters[0].name(), "readonly_items");
        assert_eq!(parameters[0].param_type(), &TypeName::primitive("Item"));
        assert!(parameters[0].default_value().is_some());
        assert!(parameters[0].is_variadic());
        assert!(parameters[0].is_property());
        assert!(!parameters[0].is_mutable_property());
        assert_eq!(parameters[1].name(), "mutable_item");
        assert!(!parameters[1].is_variadic());
        assert!(!parameters[1].is_property());
        assert!(parameters[1].is_mutable_property());

        assert_eq!(
            function.returns(),
            Some([TypeName::primitive("Output")].as_slice())
        );
        assert!(function.body().is_some());
        assert_eq!(function.modifiers().visibility, Visibility::Public);
        assert!(function.modifiers().is_async);
        assert!(function.modifiers().is_static);
        assert!(function.modifiers().is_abstract);
        assert!(function.modifiers().is_override);
        assert_eq!(function.doc(), &["Semantic view".to_string()]);

        let type_params = function.generic_params().collect::<Vec<_>>();
        assert_eq!(type_params.len(), 2);
        assert_eq!(type_params[0].name(), "'a");
        assert!(matches!(
            type_params[0].domain().as_ref(),
            sigil_stitch::spec::where_spec::GenericParamDomain::Lifetime
        ));
        assert_eq!(type_params[1].name(), "T");
        assert_eq!(type_params[1].bounds(), &[TypeName::primitive("Cloneable")]);
        assert!(matches!(
            type_params[1].legacy_kind(),
            Some(TypeParamKind::Constructor1)
        ));
        assert!(matches!(
            type_params[1].domain().as_ref(),
            sigil_stitch::spec::where_spec::GenericParamDomain::Single { kind: None }
        ));
        assert_eq!(
            type_params[1].context_bounds(),
            &[TypeName::primitive("Ordered")]
        );

        assert_eq!(function.annotations().len(), 1);
        let annotations = function.annotation_specs();
        assert_eq!(annotations.len(), 2);
        assert!(matches!(
            annotations[0].name(),
            AnnotationNameRef::Simple("simple")
        ));
        assert_eq!(annotations[0].arguments(), &["first", "second"]);
        assert!(matches!(
            annotations[1].name(),
            AnnotationNameRef::Importable(name)
                if name == &TypeName::importable("./annotations", "Tracked")
        ));
        assert!(annotations[1].arguments().is_empty());

        assert_eq!(function.suffixes(), &["const"]);
        assert!(function.delegation().is_some());
        let constraints = function.where_constraints();
        assert_eq!(constraints.len(), 1);
        assert_eq!(constraints[0].subject(), &TypeName::primitive("T"));
        assert_eq!(
            constraints[0].bounds(),
            &[TypeName::primitive("Serializable")]
        );

        CodeBlock::of("semantic-view", ())
    }
}

#[test]
fn permissive_adapter_requires_complete_function_lowering() {
    let function = FunSpec::builder("work")
        .add_param(ParameterSpec::of("value", TypeName::primitive("Text")))
        .returns(vec![TypeName::primitive("Result")])
        .build()
        .unwrap();

    assert!(matches!(
        function
            .emit(&LegacyLang, DeclarationContext::TopLevel)
            .unwrap_err(),
        SigilStitchError::MissingFunctionLowerer { .. }
    ));
}

#[test]
fn language_override_dispatches_through_every_function_facade() {
    let lang = NovelLang::permissive();
    let function = FunSpec::builder("work").build().unwrap();

    function.emit(&lang, DeclarationContext::TopLevel).unwrap();
    function.emit_members(&lang).unwrap();
    TypeSpec::builder("Container", TypeKind::Class)
        .add_method(function.clone())
        .build()
        .unwrap()
        .emit(&lang)
        .unwrap();
    FileSpec::builder_with("work.novel", lang.clone())
        .add_function(function)
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    assert_eq!(lang.calls.load(Ordering::SeqCst), 4);
    assert_eq!(lang.validation_calls.load(Ordering::SeqCst), 6);
}

#[test]
fn validation_prevents_invalid_intent_from_reaching_the_lowerer() {
    let lang = NovelLang::strict();
    let function = FunSpec::builder("work").is_async().build().unwrap();

    assert!(function.emit(&lang, DeclarationContext::TopLevel).is_err());
    assert_eq!(lang.validation_calls.load(Ordering::SeqCst), 0);
    assert_eq!(lang.calls.load(Ordering::SeqCst), 0);
}

#[test]
#[allow(
    deprecated,
    reason = "exercise released generic and callable compatibility inputs"
)]
fn default_constraint_validation_is_syntax_independent() {
    let lang = NovelLang::strict();
    let function = FunSpec::builder("copy")
        .add_type_param(TypeParamSpec::new("T"))
        .add_where_constraint(
            TypeName::primitive("U"),
            vec![TypeName::primitive("Cloneable")],
        )
        .build()
        .unwrap();

    function.emit(&lang, DeclarationContext::TopLevel).unwrap();
    assert_eq!(lang.validation_calls.load(Ordering::SeqCst), 1);
    assert_eq!(lang.calls.load(Ordering::SeqCst), 1);
}

#[test]
#[allow(
    deprecated,
    reason = "exercise released generic and callable compatibility inputs"
)]
fn validated_function_exposes_complete_read_only_semantic_intent() {
    let function = FunSpec::builder("inspect")
        .receiver(ParameterSpec::of("self", TypeName::primitive("Receiver")))
        .add_param(
            ParameterSpec::builder("readonly_items", TypeName::primitive("Item"))
                .default_value(CodeBlock::of("default_item", ()).unwrap())
                .variadic()
                .is_property()
                .build()
                .unwrap(),
        )
        .add_param(
            ParameterSpec::builder("mutable_item", TypeName::primitive("Item"))
                .is_mutable_property()
                .build()
                .unwrap(),
        )
        .returns(vec![TypeName::primitive("Output")])
        .body(CodeBlock::of("work", ()).unwrap())
        .visibility(Visibility::Public)
        .is_async()
        .is_static()
        .is_abstract()
        .is_override()
        .doc("Semantic view")
        .add_type_param(TypeParamSpec::lifetime("'a"))
        .add_type_param(
            TypeParamSpec::new("T")
                .with_bound(TypeName::primitive("Cloneable"))
                .with_kind(TypeParamKind::Constructor1)
                .with_context_bound(TypeName::primitive("Ordered")),
        )
        .annotation(CodeBlock::of("@raw", ()).unwrap())
        .annotate(AnnotationSpec::new("simple").args(["first", "second"]))
        .annotate(AnnotationSpec::importable(TypeName::importable(
            "./annotations",
            "Tracked",
        )))
        .suffix("const")
        .delegation(CodeBlock::of("super()", ()).unwrap())
        .add_where_constraint(
            TypeName::primitive("T"),
            vec![TypeName::primitive("Serializable")],
        )
        .build()
        .unwrap();

    let lowered = function
        .emit(&SemanticViewLang, DeclarationContext::TopLevel)
        .unwrap();
    assert!(!lowered.is_empty());
}

#[test]
#[allow(
    deprecated,
    reason = "exercise released generic and callable compatibility inputs"
)]
fn scala_rejects_empty_raw_higher_kinded_function_parameters() {
    let function = FunSpec::builder("transform")
        .add_type_param(TypeParamSpec::new("F").with_kind(TypeParamKind::Raw("".to_string())))
        .add_param(ParameterSpec::of("value", TypeName::primitive("Int")))
        .returns(vec![TypeName::primitive("Unit")])
        .body(CodeBlock::of("()", ()).unwrap())
        .build()
        .unwrap();

    let error = function
        .emit(
            &sigil_stitch::lang::scala::Scala::new(),
            DeclarationContext::TopLevel,
        )
        .unwrap_err();
    assert!(matches!(
        error,
        SigilStitchError::InvalidFunctionTypeParameter { reason, .. }
            if reason.contains("higher-kinded")
    ));
}

#[test]
fn boxed_adapter_preserves_type_refs_and_pretty_layout() {
    let lang = NovelLang::permissive();
    let boxed: Box<dyn CodeLang> = Box::new(lang.clone());
    let function = FunSpec::builder("merge")
        .add_param(ParameterSpec::of(
            "current",
            TypeName::importable("./models", "User"),
        ))
        .add_param(ParameterSpec::of(
            "legacy",
            TypeName::importable("./legacy", "User"),
        ))
        .build()
        .unwrap();

    let boxed_block = function
        .emit(boxed.as_ref(), DeclarationContext::TopLevel)
        .unwrap();
    assert!(!boxed_block.is_empty());

    let direct = FileSpec::builder_with("merge.novel", lang.clone())
        .add_function(function.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap();
    let pretty = FileSpec::builder_with("merge.novel", lang)
        .add_function(function)
        .build()
        .unwrap()
        .render(12)
        .unwrap();

    assert!(
        direct.contains("declare merge(User, LegacyUser)"),
        "{direct}"
    );
    assert!(pretty.contains("User,"), "{pretty}");
    assert!(pretty.contains("LegacyUser"), "{pretty}");
    assert!(pretty.contains('\n'), "{pretty}");
}

#[test]
fn dynamic_languages_lower_complete_function_intent() {
    let lua = FunSpec::builder("work")
        .doc("Run Lua work.")
        .add_param(ParameterSpec::of("end", TypeName::primitive("")))
        .suffix("-- tail")
        .body(CodeBlock::of("return end_", ()).unwrap())
        .build()
        .unwrap();
    assert_eq!(
        render(&lua, &sigil_stitch::lang::lua::Lua::new(), 80),
        "--- Run Lua work.\nfunction work(end_) -- tail\n  return end_\nend\n"
    );

    let ruby = FunSpec::builder("work")
        .doc("Run Ruby work.")
        .add_param(
            ParameterSpec::builder("class", TypeName::primitive(""))
                .default_value(CodeBlock::of("Object.new", ()).unwrap())
                .build()
                .unwrap(),
        )
        .suffix("# tail")
        .body(CodeBlock::of("class_", ()).unwrap())
        .build()
        .unwrap();
    assert_eq!(
        render(&ruby, &sigil_stitch::lang::ruby::Ruby::new(), 80),
        "# Run Ruby work.\ndef work(class_ = Object.new) # tail\n  class_\nend\n"
    );
}

#[test]
fn c_dart_go_haskell_and_java_lower_reachable_function_features() {
    let c = FunSpec::builder("work")
        .add_param(ParameterSpec::of("value", TypeName::primitive("int")))
        .returns(vec![TypeName::primitive("int")])
        .annotation(CodeBlock::of("__attribute__((cold))", ()).unwrap())
        .suffix("__attribute__((noreturn))")
        .body(CodeBlock::of("return value;", ()).unwrap())
        .build()
        .unwrap();
    let c = render(&c, &sigil_stitch::lang::c::C::new(), 80);
    assert!(
        c.contains("__attribute__((cold))\nint work(int value) __attribute__((noreturn)) {"),
        "{c}"
    );

    let dart = FunSpec::builder("work")
        .add_param(ParameterSpec::of("value", TypeName::primitive("Value")))
        .annotate(AnnotationSpec::new("tracked"))
        .suffix("sync*")
        .is_static()
        .body(CodeBlock::of("yield value;", ()).unwrap())
        .build()
        .unwrap();
    let dart = render_in_context(
        &dart,
        &sigil_stitch::lang::dart::Dart::new(),
        DeclarationContext::Member,
        80,
    );
    assert!(
        dart.contains("@tracked\nstatic work(Value value) sync* {"),
        "{dart}"
    );

    let go = FunSpec::builder("Work")
        .add_param(ParameterSpec::of("value", TypeName::primitive("Value")))
        .suffix("/* tail */")
        .body(CodeBlock::of("use(value)", ()).unwrap())
        .build()
        .unwrap();
    let go = render(&go, &sigil_stitch::lang::go::Go::new(), 80);
    assert!(go.contains("func Work(value Value) /* tail */ {"), "{go}");

    let haskell = FunSpec::builder("work")
        .add_param(ParameterSpec::of("value", TypeName::primitive("Value")))
        .returns(vec![TypeName::primitive("Result")])
        .suffix("-- tail")
        .body(CodeBlock::of("use value", ()).unwrap())
        .build()
        .unwrap();
    let haskell = render(&haskell, &sigil_stitch::lang::haskell::Haskell::new(), 80);
    assert_eq!(
        haskell,
        "work :: Value -> Result -- tail\nwork value =\n  use value\n"
    );

    let inferred_haskell = FunSpec::builder("inferred")
        .suffix("-- tail")
        .body(CodeBlock::of("42", ()).unwrap())
        .build()
        .unwrap();
    assert_eq!(
        render(
            &inferred_haskell,
            &sigil_stitch::lang::haskell::Haskell::new(),
            80,
        ),
        "inferred = -- tail\n  42\n"
    );

    let java = FunSpec::builder("work")
        .add_param(ParameterSpec::of("value", TypeName::primitive("Value")))
        .returns(vec![TypeName::primitive("Result")])
        .suffix("throws Failure")
        .body(CodeBlock::of("return use(value);", ()).unwrap())
        .build()
        .unwrap();
    let java = render_in_context(
        &java,
        &sigil_stitch::lang::java::Java::new(),
        DeclarationContext::Member,
        80,
    );
    assert!(
        java.contains("Result work(Value value) throws Failure {"),
        "{java}"
    );
}

#[test]
fn shell_lowerers_preserve_suffix_escape_hatches() {
    let function = FunSpec::builder("work")
        .suffix("# tail")
        .body(CodeBlock::of("return 0", ()).unwrap())
        .build()
        .unwrap();
    for lang in [
        Box::new(sigil_stitch::lang::bash::Bash::new()) as Box<dyn CodeLang>,
        Box::new(sigil_stitch::lang::zsh::Zsh::new()),
    ] {
        let output = render(&function, lang.as_ref(), 80);
        assert!(output.contains("function work() { # tail\n"), "{output}");
    }
}

#[test]
fn tupled_builtin_lowerers_cover_direct_and_pretty_rendering() {
    struct Case {
        lang: Box<dyn CodeLang>,
        context: DeclarationContext,
        typed: bool,
    }

    let cases = [
        Case {
            lang: Box::new(sigil_stitch::lang::c::C::new()),
            context: DeclarationContext::TopLevel,
            typed: true,
        },
        Case {
            lang: Box::new(sigil_stitch::lang::cpp::Cpp::new()),
            context: DeclarationContext::TopLevel,
            typed: true,
        },
        Case {
            lang: Box::new(sigil_stitch::lang::csharp::CSharp::new()),
            context: DeclarationContext::Member,
            typed: true,
        },
        Case {
            lang: Box::new(sigil_stitch::lang::dart::Dart::new()),
            context: DeclarationContext::TopLevel,
            typed: true,
        },
        Case {
            lang: Box::new(sigil_stitch::lang::go::Go::new()),
            context: DeclarationContext::TopLevel,
            typed: true,
        },
        Case {
            lang: Box::new(sigil_stitch::lang::java::Java::new()),
            context: DeclarationContext::Member,
            typed: true,
        },
        Case {
            lang: Box::new(sigil_stitch::lang::javascript::JavaScript::new()),
            context: DeclarationContext::TopLevel,
            typed: false,
        },
        Case {
            lang: Box::new(sigil_stitch::lang::kotlin::Kotlin::new()),
            context: DeclarationContext::TopLevel,
            typed: true,
        },
        Case {
            lang: Box::new(sigil_stitch::lang::lua::Lua::new()),
            context: DeclarationContext::TopLevel,
            typed: false,
        },
        Case {
            lang: Box::new(sigil_stitch::lang::php::Php::new()),
            context: DeclarationContext::TopLevel,
            typed: true,
        },
        Case {
            lang: Box::new(sigil_stitch::lang::python::Python::new()),
            context: DeclarationContext::TopLevel,
            typed: true,
        },
        Case {
            lang: Box::new(sigil_stitch::lang::ruby::Ruby::new()),
            context: DeclarationContext::TopLevel,
            typed: false,
        },
        Case {
            lang: Box::new(sigil_stitch::lang::rust::Rust::new()),
            context: DeclarationContext::TopLevel,
            typed: true,
        },
        Case {
            lang: Box::new(sigil_stitch::lang::scala::Scala::new()),
            context: DeclarationContext::TopLevel,
            typed: true,
        },
        Case {
            lang: Box::new(sigil_stitch::lang::swift::Swift::new()),
            context: DeclarationContext::TopLevel,
            typed: true,
        },
        Case {
            lang: Box::new(sigil_stitch::lang::typescript::TypeScript::new()),
            context: DeclarationContext::TopLevel,
            typed: true,
        },
    ];

    for case in cases {
        let parameter_type = if case.typed { "LongValue" } else { "" };
        let mut direct = FunSpec::builder("work")
            .add_param(ParameterSpec::of(
                "first_argument_name",
                TypeName::primitive(parameter_type),
            ))
            .body(CodeBlock::of("use(first_argument_name)", ()).unwrap());
        if case.typed {
            direct = direct.returns(vec![TypeName::primitive("LongResult")]);
        }
        let direct = direct.build().unwrap();
        render_in_context(&direct, case.lang.as_ref(), case.context, 120);

        let mut pretty = FunSpec::builder("work")
            .add_param(ParameterSpec::of(
                "first_argument_name",
                TypeName::primitive(parameter_type),
            ))
            .add_param(ParameterSpec::of(
                "second_argument_name",
                TypeName::primitive(parameter_type),
            ))
            .body(CodeBlock::of("use(second_argument_name)", ()).unwrap());
        if case.typed {
            pretty = pretty.returns(vec![TypeName::primitive("LongResult")]);
        }
        let pretty = pretty.build().unwrap();
        let output = render_in_context(&pretty, case.lang.as_ref(), case.context, 18);
        assert!(
            output
                .lines()
                .any(|line| { line.contains("second_argument_name") && line.trim_start() != line }),
            ".{} did not wrap and indent its second parameter:\n{output}",
            case.lang.file_extension()
        );
    }
}

#[test]
fn javascript_ocaml_php_and_python_lower_reachable_function_features() {
    let javascript = FunSpec::builder("work")
        .add_param(
            ParameterSpec::builder("value", TypeName::primitive(""))
                .default_value(CodeBlock::of("seed", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_param(
            ParameterSpec::builder("rest", TypeName::primitive(""))
                .variadic()
                .build()
                .unwrap(),
        )
        .suffix("/* tail */")
        .body(CodeBlock::of("return rest;", ()).unwrap())
        .build()
        .unwrap();
    let javascript = render(
        &javascript,
        &sigil_stitch::lang::javascript::JavaScript::new(),
        80,
    );
    assert!(
        javascript.contains("function work(value = seed, ...rest) /* tail */ {"),
        "{javascript}"
    );

    let constructor = FunSpec::builder("constructor")
        .is_constructor()
        .delegation(CodeBlock::of("super()", ()).unwrap())
        .body(CodeBlock::of("initialize();", ()).unwrap())
        .build()
        .unwrap();
    let constructor = render_in_context(
        &constructor,
        &sigil_stitch::lang::javascript::JavaScript::new(),
        DeclarationContext::Member,
        80,
    );
    assert!(
        constructor.contains("constructor() {\n  super();\n  initialize();"),
        "{constructor}"
    );

    let ocaml = FunSpec::builder("work")
        .add_param(ParameterSpec::of("value", TypeName::primitive("value")))
        .returns(vec![TypeName::primitive("result")])
        .suffix("(* tail *)")
        .body(CodeBlock::of("use value", ()).unwrap())
        .build()
        .unwrap();
    let ocaml = render(&ocaml, &sigil_stitch::lang::ocaml::OCaml::new(), 80);
    assert!(
        ocaml.contains("let work (value : value) (* tail *) : result ="),
        "{ocaml}"
    );

    let php = FunSpec::builder("work")
        .add_param(
            ParameterSpec::builder("value", TypeName::primitive("Value"))
                .default_value(CodeBlock::of("null", ()).unwrap())
                .build()
                .unwrap(),
        )
        .returns(vec![TypeName::primitive("Result")])
        .annotation(CodeBlock::of("#[Raw]", ()).unwrap())
        .suffix("/* tail */")
        .is_override()
        .body(CodeBlock::of("return use($value);", ()).unwrap())
        .build()
        .unwrap();
    let php = render_in_context(
        &php,
        &sigil_stitch::lang::php::Php::new(),
        DeclarationContext::Member,
        80,
    );
    assert!(
        php.contains(
            "#[Raw]\n#[Override]\npublic function work(Value $value = null) /* tail */: Result {"
        ),
        "{php}"
    );

    let python = FunSpec::builder("work")
        .add_param(
            ParameterSpec::builder("value", TypeName::primitive("Value"))
                .default_value(CodeBlock::of("None", ()).unwrap())
                .build()
                .unwrap(),
        )
        .annotation(CodeBlock::of("@raw", ()).unwrap())
        .is_async()
        .body(CodeBlock::of("return value", ()).unwrap())
        .build()
        .unwrap();
    let python = render(&python, &sigil_stitch::lang::python::Python::new(), 80);
    assert!(
        python.contains("@raw\nasync def work(value: Value = None):"),
        "{python}"
    );
}

#[test]
fn rust_scala_and_swift_lower_reachable_function_features() {
    let rust = FunSpec::builder("work")
        .add_param(ParameterSpec::of("value", TypeName::primitive("Value")))
        .suffix("/* tail */")
        .build()
        .unwrap();
    let rust = render_in_context(
        &rust,
        &sigil_stitch::lang::rust::Rust::new(),
        DeclarationContext::InterfaceMember,
        80,
    );
    assert_eq!(rust, "fn work(value: Value) /* tail */;\n");

    let scala = FunSpec::builder("work")
        .add_param(
            ParameterSpec::builder("value", TypeName::primitive("Value"))
                .default_value(CodeBlock::of("seed", ()).unwrap())
                .build()
                .unwrap(),
        )
        .annotation(CodeBlock::of("@raw", ()).unwrap())
        .suffix("/* tail */")
        .is_override()
        .body(CodeBlock::of("use(value)", ()).unwrap())
        .build()
        .unwrap();
    let scala = render_in_context(
        &scala,
        &sigil_stitch::lang::scala::Scala::new(),
        DeclarationContext::Member,
        80,
    );
    assert!(
        scala.contains("@raw\noverride def work(value: Value = seed) /* tail */ = {"),
        "{scala}"
    );

    let swift = FunSpec::builder("work")
        .add_param(
            ParameterSpec::builder("value", TypeName::primitive("Value"))
                .default_value(CodeBlock::of("seed", ()).unwrap())
                .build()
                .unwrap(),
        )
        .annotate(AnnotationSpec::new("tracked"))
        .annotation(CodeBlock::of("@raw", ()).unwrap())
        .suffix("throws")
        .is_static()
        .body(CodeBlock::of("use(value)", ()).unwrap())
        .build()
        .unwrap();
    let swift = render_in_context(
        &swift,
        &sigil_stitch::lang::swift::Swift::new(),
        DeclarationContext::Member,
        80,
    );
    assert!(
        swift.contains("@tracked\n@raw\nstatic func work(value: Value = seed) throws {"),
        "{swift}"
    );
}

fn render(function: &FunSpec, lang: &dyn CodeLang, width: usize) -> String {
    render_in_context(function, lang, DeclarationContext::TopLevel, width)
}

fn render_in_context(
    function: &FunSpec,
    lang: &dyn CodeLang,
    context: DeclarationContext,
    width: usize,
) -> String {
    let block = function.emit(lang, context).unwrap();
    let imports = sigil_stitch::import::ImportGroup::new();
    let mut renderer = sigil_stitch::code_renderer::CodeRenderer::new(lang, &imports, width);
    renderer.render(&block).unwrap()
}
