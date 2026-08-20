use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::error::SigilStitchError;
use sigil_stitch::lang::c::C;
use sigil_stitch::lang::capability::{
    FunctionCapability, FunctionCapabilityProfile, FunctionContext, FunctionForm,
    LanguageCapabilities,
};
use sigil_stitch::lang::cpp::Cpp;
use sigil_stitch::lang::csharp::CSharp;
use sigil_stitch::lang::dart::Dart;
use sigil_stitch::lang::go::Go;
use sigil_stitch::lang::haskell::Haskell;
use sigil_stitch::lang::java::Java;
use sigil_stitch::lang::javascript::JavaScript;
use sigil_stitch::lang::kotlin::Kotlin;
use sigil_stitch::lang::lua::Lua;
use sigil_stitch::lang::php::Php;
use sigil_stitch::lang::python::Python;
use sigil_stitch::lang::ruby::Ruby;
use sigil_stitch::lang::rust::Rust;
use sigil_stitch::lang::swift::Swift;
use sigil_stitch::lang::typescript::TypeScript;
use sigil_stitch::lang::{CodeLang, RendererLang};
use sigil_stitch::spec::annotation_spec::AnnotationSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::{DeclarationContext, TypeKind, Visibility};
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::spec::where_spec::TypeParamSpec;
use sigil_stitch::type_name::TypeName;

#[derive(Debug)]
struct LegacyVirtualLang;

impl RendererLang for LegacyVirtualLang {
    fn file_extension(&self) -> &str {
        "legacy"
    }

    fn line_comment_prefix(&self) -> &str {
        "//"
    }
}

impl CodeLang for LegacyVirtualLang {
    #[allow(deprecated)] // Exercises the frozen 0.6.8 compatibility lowerer.
    fn function_syntax(&self) -> sigil_stitch::lang::config::FunctionSyntaxConfig<'_> {
        sigil_stitch::lang::config::FunctionSyntaxConfig {
            abstract_keyword: "virtual ",
            ..Default::default()
        }
    }
}

#[derive(Debug)]
struct StrictReceiverLang;

impl RendererLang for StrictReceiverLang {
    fn file_extension(&self) -> &str {
        "strict-receiver"
    }

    fn line_comment_prefix(&self) -> &str {
        "//"
    }
}

const STRICT_RECEIVER_FUNCTIONS: &[FunctionCapabilityProfile<'_>] =
    &[FunctionCapabilityProfile::new(
        FunctionContext::ReceiverMethod,
        FunctionForm::Function,
        &[],
    )];

impl CodeLang for StrictReceiverLang {
    fn capabilities(&self) -> LanguageCapabilities<'_> {
        LanguageCapabilities::strict().with_functions(STRICT_RECEIVER_FUNCTIONS)
    }
}

fn async_function(name: &str) -> FunSpec {
    FunSpec::builder(name)
        .is_async()
        .body(CodeBlock::of("return nil", ()).unwrap())
        .build()
        .unwrap()
}

fn render_function(
    fun: &FunSpec,
    lang: &dyn sigil_stitch::lang::CodeLang,
    context: DeclarationContext,
) -> String {
    let block = fun.emit(lang, context).unwrap();
    let imports = sigil_stitch::import::ImportGroup::new();
    let mut renderer = sigil_stitch::code_renderer::CodeRenderer::new(lang, &imports, 80);
    renderer.render(&block).unwrap()
}

#[test]
fn validate_and_emit_report_the_same_missing_capability() {
    let lua = Lua::new();
    let fun = async_function("load");

    let validate_error = fun
        .validate(&lua, DeclarationContext::TopLevel)
        .unwrap_err();
    let emit_error = fun.emit(&lua, DeclarationContext::TopLevel).unwrap_err();

    for error in [&validate_error, &emit_error] {
        assert!(matches!(
            error,
            SigilStitchError::UnsupportedFunctionCapabilities {
                language,
                function_name,
                context: FunctionContext::TopLevel,
                form: FunctionForm::Function,
                capabilities,
            } if language == "lua"
                && function_name == "load"
                && capabilities == &vec![FunctionCapability::AsyncEffect]
        ));
    }
}

#[test]
fn unsupported_function_context_has_its_own_error() {
    let fun = FunSpec::builder("work").build().unwrap();

    let error = fun
        .validate(&Java::new(), DeclarationContext::TopLevel)
        .unwrap_err();

    assert!(matches!(
        error,
        SigilStitchError::UnsupportedFunctionContext {
            context: FunctionContext::TopLevel,
            ..
        }
    ));
}

#[test]
fn go_distinguishes_generic_free_functions_from_receiver_methods() {
    let type_param = TypeParamSpec::new("T");
    let free = FunSpec::builder("Map")
        .add_type_param(type_param.clone())
        .body(CodeBlock::of("return", ()).unwrap())
        .build()
        .unwrap();
    assert!(
        free.validate(&Go::new(), DeclarationContext::TopLevel)
            .is_ok()
    );

    let receiver = ParameterSpec::new("s", TypeName::primitive("Server")).unwrap();
    let method = FunSpec::builder("Map")
        .receiver(receiver)
        .add_type_param(type_param)
        .body(CodeBlock::of("return", ()).unwrap())
        .build()
        .unwrap();
    let error = method
        .validate(&Go::new(), DeclarationContext::TopLevel)
        .unwrap_err();

    assert!(matches!(
        error,
        SigilStitchError::UnsupportedFunctionCapabilities {
            context: FunctionContext::ReceiverMethod,
            capabilities,
            ..
        } if capabilities == vec![FunctionCapability::ParametricPolymorphism]
    ));
}

#[test]
fn typed_receivers_request_the_typed_parameters_capability() {
    let receiver = ParameterSpec::new("s", TypeName::primitive("Server")).unwrap();
    let method = FunSpec::builder("Start")
        .receiver(receiver)
        .build()
        .unwrap();

    assert!(matches!(
        method.validate(&StrictReceiverLang, DeclarationContext::TopLevel),
        Err(SigilStitchError::UnsupportedFunctionCapabilities {
            context: FunctionContext::ReceiverMethod,
            capabilities,
            ..
        }) if capabilities == vec![FunctionCapability::TypedParameters]
    ));
}

#[test]
fn haskell_interface_methods_require_a_type_signature() {
    let method = FunSpec::builder("empty").build().unwrap();

    assert!(matches!(
        method.validate(&Haskell::new(), DeclarationContext::InterfaceMember),
        Err(SigilStitchError::MissingRequiredFunctionCapabilities {
            capabilities,
            ..
        }) if capabilities == vec![FunctionCapability::ExplicitReturnType]
    ));
}

#[test]
fn receiver_is_rejected_inside_a_type_body() {
    let receiver = ParameterSpec::builder("s", TypeName::primitive(""))
        .default_value(CodeBlock::of("Server{}", ()).unwrap())
        .variadic()
        .is_property()
        .build()
        .unwrap();
    let method = FunSpec::builder("Start")
        .receiver(receiver)
        .build()
        .unwrap();

    let error = method
        .validate(&Go::new(), DeclarationContext::Member)
        .unwrap_err();

    assert!(matches!(
        error,
        SigilStitchError::InvalidFunctionPlacement {
            context: DeclarationContext::Member,
            ..
        }
    ));
}

#[test]
fn receiver_parameter_features_are_rejected_before_rendering() {
    let receiver = ParameterSpec::builder("s", TypeName::primitive("Server"))
        .default_value(CodeBlock::of("Server{}", ()).unwrap())
        .variadic()
        .is_property()
        .build()
        .unwrap();
    let method = FunSpec::builder("Start")
        .receiver(receiver)
        .build()
        .unwrap();

    let error = method
        .validate(&Go::new(), DeclarationContext::TopLevel)
        .unwrap_err();

    assert!(matches!(
        error,
        SigilStitchError::InvalidReceiverCapabilities {
            capabilities,
            ..
        } if capabilities == vec![
            FunctionCapability::DefaultParameters,
            FunctionCapability::VariadicParameters,
            FunctionCapability::ConstructorProperties,
        ]
    ));
}

#[test]
fn constructor_names_are_classified_and_validated_in_type_context() {
    let async_constructor = FunSpec::builder("constructor").is_async().build().unwrap();
    assert!(matches!(
        async_constructor.validate(&TypeScript::new(), DeclarationContext::Member),
        Err(SigilStitchError::UnsupportedFunctionCapabilities {
            form: FunctionForm::Constructor,
            capabilities,
            ..
        }) if capabilities == vec![FunctionCapability::AsyncEffect]
    ));

    let invalid_typescript = TypeSpec::builder("Widget", TypeKind::Class)
        .add_method(
            FunSpec::builder("make")
                .is_constructor()
                .body(CodeBlock::of("return", ()).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    assert!(matches!(
        invalid_typescript.validate(&TypeScript::new()),
        Err(SigilStitchError::InvalidConstructorName {
            type_name,
            constructor_name,
            ..
        }) if type_name.as_deref() == Some("Widget") && constructor_name == "make"
    ));

    let invalid_java = TypeSpec::builder("Widget", TypeKind::Class)
        .add_method(
            FunSpec::builder("Other")
                .is_constructor()
                .body(CodeBlock::of("return;", ()).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    assert!(matches!(
        invalid_java.validate(&Java::new()),
        Err(SigilStitchError::InvalidConstructorName {
            type_name,
            constructor_name,
            ..
        }) if type_name.as_deref() == Some("Widget") && constructor_name == "Other"
    ));

    let java_method_named_like_owner = TypeSpec::builder("Widget", TypeKind::Class)
        .add_method(
            FunSpec::builder("Widget")
                .returns(TypeName::primitive("void"))
                .body(CodeBlock::of("return;", ()).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    assert!(java_method_named_like_owner.validate(&Java::new()).is_ok());

    for lang in [
        &CSharp::new() as &dyn sigil_stitch::lang::CodeLang,
        &Cpp::new(),
        &Dart::new(),
    ] {
        let invalid = TypeSpec::builder("Widget", TypeKind::Class)
            .add_method(
                FunSpec::builder("Widget")
                    .returns(TypeName::primitive("int"))
                    .body(CodeBlock::of("return 0;", ()).unwrap())
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();
        assert!(matches!(
            invalid.validate(lang),
            Err(SigilStitchError::UnsupportedFunctionCapabilities {
                form: FunctionForm::Constructor,
                capabilities,
                ..
            }) if capabilities == vec![FunctionCapability::ExplicitReturnType]
        ));
    }
}

#[test]
fn direct_fixed_name_constructors_reject_invalid_names_without_an_owner() {
    let constructor = FunSpec::builder("make")
        .is_constructor()
        .body(CodeBlock::of("return", ()).unwrap())
        .build()
        .unwrap();

    assert!(matches!(
        constructor.validate(&TypeScript::new(), DeclarationContext::Member),
        Err(SigilStitchError::InvalidConstructorName {
            type_name: None,
            constructor_name,
            ..
        }) if constructor_name == "make"
    ));
}

#[test]
fn direct_constructor_inference_respects_context_profiles() {
    let interface_method = FunSpec::builder("constructor").build().unwrap();
    assert!(
        interface_method
            .validate(&TypeScript::new(), DeclarationContext::InterfaceMember)
            .is_ok()
    );
}

#[test]
fn fixed_constructor_names_remain_functions_outside_instance_member_context() {
    let javascript = FunSpec::builder("constructor")
        .body(CodeBlock::of("return", ()).unwrap())
        .build()
        .unwrap();
    assert!(
        javascript
            .validate(&JavaScript::new(), DeclarationContext::TopLevel)
            .is_ok()
    );

    let python = FunSpec::builder("__init__")
        .body(CodeBlock::of("return None", ()).unwrap())
        .build()
        .unwrap();
    assert!(
        python
            .validate(&Python::new(), DeclarationContext::TopLevel)
            .is_ok()
    );

    let static_javascript = TypeSpec::builder("Widget", TypeKind::Class)
        .add_method(
            FunSpec::builder("constructor")
                .is_static()
                .body(CodeBlock::of("return", ()).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    assert!(static_javascript.validate(&JavaScript::new()).is_ok());
}

#[test]
fn static_constructor_names_follow_language_specific_classification() {
    for (lang, name) in [
        (&Swift::new() as &dyn sigil_stitch::lang::CodeLang, "init"),
        (&Php::new(), "__construct"),
        (&Dart::new(), "Widget"),
    ] {
        let invalid = TypeSpec::builder("Widget", TypeKind::Class)
            .add_method(
                FunSpec::builder(name)
                    .is_static()
                    .body(CodeBlock::of("return", ()).unwrap())
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();

        assert!(matches!(
            invalid.validate(lang),
            Err(SigilStitchError::UnsupportedFunctionCapabilities {
                form: FunctionForm::Constructor,
                capabilities,
                ..
            }) if capabilities == vec![FunctionCapability::StaticConstructor]
        ));
    }
}

#[test]
fn class_backed_contracts_accept_their_constructor_profiles() {
    let javascript = TypeSpec::builder("Widget", TypeKind::Interface)
        .add_method(
            FunSpec::builder("constructor")
                .body(CodeBlock::of("return", ()).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    let cpp = TypeSpec::builder("Widget", TypeKind::Interface)
        .add_method(FunSpec::builder("Widget").build().unwrap())
        .build()
        .unwrap();
    let dart = TypeSpec::builder("Widget", TypeKind::Interface)
        .add_method(FunSpec::builder("Widget").build().unwrap())
        .build()
        .unwrap();
    let php = TypeSpec::builder("Widget", TypeKind::Interface)
        .add_method(FunSpec::builder("__construct").build().unwrap())
        .build()
        .unwrap();

    assert!(javascript.emit(&JavaScript::new()).is_ok());
    assert!(cpp.emit(&Cpp::new()).is_ok());
    assert!(dart.emit(&Dart::new()).is_ok());
    assert!(php.emit(&Php::new()).is_ok());
}

#[test]
fn php_traits_use_concrete_member_rules_without_becoming_concrete_types() {
    let trait_spec = TypeSpec::builder("Reusable", TypeKind::Trait)
        .add_method(
            FunSpec::builder("work")
                .visibility(sigil_stitch::spec::modifiers::Visibility::Private)
                .returns(TypeName::primitive("void"))
                .body(CodeBlock::of("return", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("deferred")
                .is_abstract()
                .returns(TypeName::primitive("void"))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let output = FileSpec::builder_with("Reusable.php", Php::new())
        .add_type(trait_spec)
        .build()
        .unwrap()
        .render(80)
        .unwrap();
    assert!(output.contains("private function work(): void"), "{output}");
    assert!(output.contains("abstract"), "{output}");
    assert!(output.contains("function deferred(): void;"), "{output}");
}

#[test]
fn ruby_module_backed_types_preserve_member_visibility() {
    let trait_spec = TypeSpec::builder("Reusable", TypeKind::Trait)
        .add_method(
            FunSpec::builder("work")
                .visibility(sigil_stitch::spec::modifiers::Visibility::Private)
                .body(CodeBlock::of("nil", ()).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    let output = FileSpec::builder_with("reusable.rb", Ruby::new())
        .add_type(trait_spec)
        .build()
        .unwrap()
        .render(80)
        .unwrap();
    assert!(output.contains("private\n  def work()"), "{output}");
}

#[test]
fn dart_and_swift_validate_constructor_name_grammar() {
    for name in ["Widget.", "Widget..named", "Widget.named.extra"] {
        let invalid = TypeSpec::builder("Widget", TypeKind::Class)
            .add_method(
                FunSpec::builder(name)
                    .is_constructor()
                    .body(CodeBlock::of("return;", ()).unwrap())
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();
        assert!(matches!(
            invalid.validate(&Dart::new()),
            Err(SigilStitchError::InvalidConstructorName { constructor_name, .. })
                if constructor_name == name
        ));

        let direct = FunSpec::builder(name)
            .is_constructor()
            .body(CodeBlock::of("return;", ()).unwrap())
            .build()
            .unwrap();
        assert!(matches!(
            direct.validate(&Dart::new(), DeclarationContext::Member),
            Err(SigilStitchError::InvalidConstructorName {
                type_name: None,
                constructor_name,
                ..
            }) if constructor_name == name
        ));
    }

    for name in ["init?", "init!"] {
        let initializer = TypeSpec::builder("Widget", TypeKind::Class)
            .add_method(
                FunSpec::builder(name)
                    .body(CodeBlock::of("return", ()).unwrap())
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();
        assert!(initializer.validate(&Swift::new()).is_ok());
    }
}

#[test]
fn abstract_type_modifiers_follow_language_and_type_kind_rules() {
    for kind in [TypeKind::Interface, TypeKind::Trait] {
        let contract = TypeSpec::builder("Contract", kind)
            .is_abstract()
            .build()
            .unwrap();
        let output = FileSpec::builder_with("Contract.java", Java::new())
            .add_type(contract)
            .build()
            .unwrap()
            .render(80)
            .unwrap();
        assert!(output.contains("abstract interface Contract"), "{output}");
    }

    for lang in [
        &Java::new() as &dyn sigil_stitch::lang::CodeLang,
        &Kotlin::new(),
        &Php::new(),
    ] {
        let invalid = TypeSpec::builder("Bad", TypeKind::Enum)
            .is_abstract()
            .build()
            .unwrap();
        assert!(matches!(
            invalid.validate(lang),
            Err(SigilStitchError::InvalidAbstractType {
                kind: TypeKind::Enum,
                ..
            })
        ));
    }

    for lang in [
        &Cpp::new() as &dyn sigil_stitch::lang::CodeLang,
        &Python::new(),
        &Rust::new(),
    ] {
        let invalid = TypeSpec::builder("Bad", TypeKind::Class)
            .is_abstract()
            .build()
            .unwrap();
        assert!(matches!(
            invalid.validate(lang),
            Err(SigilStitchError::InvalidAbstractType {
                kind: TypeKind::Class,
                ..
            })
        ));
    }
}

#[test]
fn abstract_methods_require_an_abstract_concrete_type() {
    for lang in [
        &Java::new() as &dyn sigil_stitch::lang::CodeLang,
        &CSharp::new(),
        &Dart::new(),
        &Kotlin::new(),
        &Php::new(),
        &sigil_stitch::lang::scala::Scala::new(),
        &TypeScript::new(),
    ] {
        let concrete = TypeSpec::builder("Widget", TypeKind::Class)
            .add_method(
                FunSpec::builder("work")
                    .is_abstract()
                    .returns(TypeName::primitive("void"))
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();
        assert!(matches!(
            concrete.validate(lang),
            Err(SigilStitchError::AbstractMethodInConcreteType {
                type_name,
                function_name,
                ..
            }) if type_name == "Widget" && function_name == "work"
        ));
    }

    let cpp = TypeSpec::builder("Widget", TypeKind::Class)
        .add_method(
            FunSpec::builder("work")
                .is_abstract()
                .returns(TypeName::primitive("void"))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    assert!(cpp.validate(&Cpp::new()).is_ok());
}

#[test]
fn constructor_features_are_rejected_on_ordinary_methods() {
    let property = ParameterSpec::builder("name", TypeName::primitive("string"))
        .is_property()
        .build()
        .unwrap();
    let method = FunSpec::builder("rename")
        .add_param(property)
        .delegation(CodeBlock::of("super(name)", ()).unwrap())
        .build()
        .unwrap();

    let error = method
        .validate(&TypeScript::new(), DeclarationContext::Member)
        .unwrap_err();

    assert!(matches!(
        error,
        SigilStitchError::InvalidConstructorFeaturePlacement {
            capabilities,
            ..
        } if capabilities == vec![
            FunctionCapability::ConstructorDelegation,
            FunctionCapability::ConstructorProperties,
        ]
    ));
}

#[test]
fn abstract_function_with_a_body_is_rejected() {
    let method = FunSpec::builder("draw")
        .is_abstract()
        .body(CodeBlock::of("return", ()).unwrap())
        .build()
        .unwrap();

    assert!(matches!(
        method.validate(&Java::new(), DeclarationContext::Member),
        Err(SigilStitchError::AbstractFunctionWithBody { .. })
    ));
}

#[test]
fn cplusplus_virtual_method_with_a_body_preserves_legacy_semantics() {
    let method = FunSpec::builder("draw")
        .is_abstract()
        .returns(TypeName::primitive("void"))
        .body(CodeBlock::of("draw_impl();", ()).unwrap())
        .build()
        .unwrap();

    let output = render_function(&method, &Cpp::new(), DeclarationContext::Member);
    assert!(output.starts_with("virtual void draw() {"), "{output}");
    assert!(output.contains("draw_impl();"), "{output}");
}

#[test]
fn permissive_legacy_adapter_accepts_abstract_function_with_a_body() {
    let method = FunSpec::builder("draw")
        .is_abstract()
        .body(CodeBlock::of("draw_impl();", ()).unwrap())
        .build()
        .unwrap();

    assert!(
        method
            .validate(&LegacyVirtualLang, DeclarationContext::Member)
            .is_ok()
    );
    let output = render_function(&method, &LegacyVirtualLang, DeclarationContext::Member);
    assert!(output.starts_with("virtual draw() {"), "{output}");
    assert!(output.contains("draw_impl();"), "{output}");
}

#[test]
fn permissive_legacy_adapter_preserves_pre_validation_function_shapes() {
    let receiver = ParameterSpec::builder("self", TypeName::primitive("Widget"))
        .default_value(CodeBlock::of("Widget{}", ()).unwrap())
        .variadic()
        .is_property()
        .build()
        .unwrap();
    let property = ParameterSpec::builder("name", TypeName::primitive("String"))
        .is_property()
        .build()
        .unwrap();
    let method = FunSpec::builder("draw")
        .receiver(receiver)
        .add_param(property)
        .delegation(CodeBlock::of("legacy_delegate()", ()).unwrap())
        .add_where_constraint(
            TypeName::primitive("Undeclared"),
            vec![TypeName::primitive("Bound")],
        )
        .body(CodeBlock::of("draw_impl();", ()).unwrap())
        .build()
        .unwrap();

    assert!(
        method
            .emit(&LegacyVirtualLang, DeclarationContext::Member)
            .is_ok()
    );
}

#[test]
fn inline_where_constraints_are_merged_into_type_parameters() {
    let fun = FunSpec::builder("copy")
        .add_type_param(TypeParamSpec::new("T"))
        .add_where_constraint(
            TypeName::primitive("T"),
            vec![TypeName::primitive("Cloneable")],
        )
        .returns(TypeName::primitive("void"))
        .body(CodeBlock::of("return;", ()).unwrap())
        .build()
        .unwrap();

    let output = render_function(&fun, &Java::new(), DeclarationContext::Member);
    assert!(output.contains("<T extends Cloneable>"), "{output}");
}

#[test]
fn duplicate_inline_where_constraints_are_merged_once() {
    let cloneable = TypeName::primitive("Cloneable");
    let fun = FunSpec::builder("copy")
        .add_type_param(TypeParamSpec::new("T").with_bound(cloneable.clone()))
        .add_where_constraint(TypeName::primitive("T"), vec![cloneable])
        .returns(TypeName::primitive("void"))
        .body(CodeBlock::of("return;", ()).unwrap())
        .build()
        .unwrap();

    let output = render_function(&fun, &Java::new(), DeclarationContext::Member);
    assert_eq!(output.matches("Cloneable").count(), 1, "{output}");
}

#[test]
fn unmatched_inline_where_constraint_fails_closed() {
    let fun = FunSpec::builder("copy")
        .add_type_param(TypeParamSpec::new("T"))
        .add_where_constraint(
            TypeName::primitive("U"),
            vec![TypeName::primitive("Cloneable")],
        )
        .returns(TypeName::primitive("void"))
        .body(CodeBlock::of("return;", ()).unwrap())
        .build()
        .unwrap();

    let lang = Java::new();
    let validation_error = fun.validate(&lang, DeclarationContext::Member).unwrap_err();
    assert!(matches!(
        validation_error,
        SigilStitchError::InvalidFunctionConstraintSubject { subject, .. }
            if subject == "U"
    ));

    let error = fun.emit(&lang, DeclarationContext::Member).unwrap_err();
    assert!(matches!(
        error,
        SigilStitchError::InvalidFunctionConstraintSubject { subject, .. }
            if subject == "U"
    ));
}

#[test]
fn typescript_rejects_decorators_on_top_level_functions() {
    let fun = FunSpec::builder("work")
        .annotation(CodeBlock::of("@logged", ()).unwrap())
        .build()
        .unwrap();

    let error = fun
        .emit(&TypeScript::new(), DeclarationContext::TopLevel)
        .unwrap_err();
    assert!(matches!(
        error,
        SigilStitchError::UnsupportedFunctionCapabilities { capabilities, .. }
            if capabilities == vec![FunctionCapability::Attributes]
    ));
}

#[test]
fn cplusplus_renders_function_capabilities_in_their_grammar_positions() {
    let abstract_method = FunSpec::builder("draw")
        .is_abstract()
        .returns(TypeName::primitive("void"))
        .build()
        .unwrap();
    assert_eq!(
        render_function(&abstract_method, &Cpp::new(), DeclarationContext::Member),
        "virtual void draw();\n"
    );

    let explicit_pure_virtual_suffix = FunSpec::builder("erase")
        .is_abstract()
        .suffix("= 0")
        .returns(TypeName::primitive("void"))
        .build()
        .unwrap();
    assert_eq!(
        render_function(
            &explicit_pure_virtual_suffix,
            &Cpp::new(),
            DeclarationContext::Member,
        ),
        "virtual void erase() = 0;\n"
    );

    let pure_virtual_override = FunSpec::builder("replace")
        .is_abstract()
        .is_override()
        .suffix("= 0")
        .returns(TypeName::primitive("void"))
        .build()
        .unwrap();
    assert_eq!(
        render_function(
            &pure_virtual_override,
            &Cpp::new(),
            DeclarationContext::Member,
        ),
        "virtual void replace() override = 0;\n"
    );

    let defaulted_destructor = FunSpec::builder("~Shape")
        .is_abstract()
        .suffix("= default")
        .build()
        .unwrap();
    assert_eq!(
        render_function(
            &defaulted_destructor,
            &Cpp::new(),
            DeclarationContext::Member,
        ),
        "virtual ~Shape() = default;\n"
    );

    let override_method = FunSpec::builder("draw")
        .is_override()
        .returns(TypeName::primitive("void"))
        .build()
        .unwrap();
    assert_eq!(
        render_function(&override_method, &Cpp::new(), DeclarationContext::Member),
        "void draw() override;\n"
    );
}

#[test]
fn dart_rejects_unrepresentable_defaults_and_python_keeps_decorator_statics() {
    let default_param = ParameterSpec::builder("count", TypeName::primitive("int"))
        .default_value(CodeBlock::of("1", ()).unwrap())
        .build()
        .unwrap();
    let dart_function = FunSpec::builder("count")
        .add_param(default_param)
        .build()
        .unwrap();
    let dart_error = dart_function
        .validate(&Dart::new(), DeclarationContext::TopLevel)
        .unwrap_err();
    assert!(matches!(
        dart_error,
        SigilStitchError::UnsupportedFunctionCapabilities { capabilities, .. }
            if capabilities == vec![FunctionCapability::DefaultParameters]
    ));

    let python_method = FunSpec::builder("create")
        .is_static()
        .annotation(CodeBlock::of("@classmethod", ()).unwrap())
        .build()
        .unwrap();
    assert!(
        python_method
            .validate(&Python::new(), DeclarationContext::Member)
            .is_ok()
    );

    let fragmented_python_method = FunSpec::builder("create")
        .is_static()
        .annotation(CodeBlock::of("@%L", "classmethod").unwrap())
        .build()
        .unwrap();
    assert!(
        fragmented_python_method
            .validate(&Python::new(), DeclarationContext::Member)
            .is_ok(),
        "equivalent opaque decorator blocks must not depend on node shape"
    );

    let bare_python_method = FunSpec::builder("create").is_static().build().unwrap();
    let python_error = bare_python_method
        .validate(&Python::new(), DeclarationContext::Member)
        .unwrap_err();
    assert!(matches!(
        python_error,
        SigilStitchError::UnsupportedFunctionCapabilities { capabilities, .. }
            if capabilities == vec![FunctionCapability::StaticMethod]
    ));

    let unrelated_decorator = FunSpec::builder("create")
        .is_static()
        .annotate(AnnotationSpec::new("deprecated"))
        .build()
        .unwrap();
    let decorator_error = unrelated_decorator
        .validate(&Python::new(), DeclarationContext::Member)
        .unwrap_err();
    assert!(matches!(
        decorator_error,
        SigilStitchError::UnsupportedFunctionCapabilities { capabilities, .. }
            if capabilities == vec![FunctionCapability::StaticMethod]
    ));

    let bare_annotation_name = FunSpec::builder("create")
        .is_static()
        .annotation(CodeBlock::of("classmethod", ()).unwrap())
        .build()
        .unwrap();
    let annotation_error = bare_annotation_name
        .validate(&Python::new(), DeclarationContext::Member)
        .unwrap_err();
    assert!(matches!(
        annotation_error,
        SigilStitchError::UnsupportedFunctionCapabilities { capabilities, .. }
            if capabilities == vec![FunctionCapability::StaticMethod]
    ));

    let annotated_static_constructor = FunSpec::builder("__init__")
        .is_constructor()
        .is_static()
        .annotation(CodeBlock::of("@classmethod", ()).unwrap())
        .build()
        .unwrap();
    let constructor_error = annotated_static_constructor
        .validate(&Python::new(), DeclarationContext::Member)
        .unwrap_err();
    assert!(matches!(
        constructor_error,
        SigilStitchError::UnsupportedFunctionCapabilities { capabilities, .. }
            if capabilities == vec![FunctionCapability::StaticConstructor]
    ));
}

#[test]
fn java_rejects_static_constructors_without_a_dedicated_capability() {
    let constructor = FunSpec::builder("User")
        .is_constructor()
        .is_static()
        .build()
        .unwrap();

    let error = constructor
        .validate(&Java::new(), DeclarationContext::Member)
        .unwrap_err();
    assert!(matches!(
        error,
        SigilStitchError::UnsupportedFunctionCapabilities { capabilities, .. }
            if capabilities == vec![FunctionCapability::StaticConstructor]
    ));
}

#[test]
fn csharp_static_constructors_enforce_language_rules() {
    let valid = TypeSpec::builder("Widget", TypeKind::Class)
        .add_method(
            FunSpec::builder("Widget")
                .is_constructor()
                .is_static()
                .body(CodeBlock::of("Initialize();", ()).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    let output = FileSpec::builder_with("Widget.cs", CSharp::new())
        .add_type(valid)
        .build()
        .unwrap()
        .render(80)
        .unwrap();
    assert!(output.contains("\n    static Widget() {\n"), "{output}");
    assert!(!output.contains("internal static Widget()"), "{output}");

    let parameter = ParameterSpec::builder("value", TypeName::primitive("int"))
        .build()
        .unwrap();
    let with_parameter = TypeSpec::builder("Widget", TypeKind::Class)
        .add_method(
            FunSpec::builder("Widget")
                .is_constructor()
                .is_static()
                .add_param(parameter)
                .body(CodeBlock::of("Initialize();", ()).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    assert!(matches!(
        with_parameter.validate(&CSharp::new()),
        Err(SigilStitchError::TooManyFunctionParameters {
            maximum: 0,
            actual: 1,
            ..
        })
    ));

    let public = TypeSpec::builder("Widget", TypeKind::Class)
        .add_method(
            FunSpec::builder("Widget")
                .is_constructor()
                .is_static()
                .visibility(sigil_stitch::spec::modifiers::Visibility::Public)
                .body(CodeBlock::of("Initialize();", ()).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    assert!(matches!(
        public.validate(&CSharp::new()),
        Err(SigilStitchError::InvalidFunctionVisibility {
            visibility: sigil_stitch::spec::modifiers::Visibility::Public,
            ..
        })
    ));

    let bodyless = TypeSpec::builder("Widget", TypeKind::Class)
        .add_method(
            FunSpec::builder("Widget")
                .is_constructor()
                .is_static()
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    assert!(matches!(
        bodyless.validate(&CSharp::new()),
        Err(SigilStitchError::FunctionBodyRequired {
            form: FunctionForm::Constructor,
            ..
        })
    ));

    let wrong_name = TypeSpec::builder("Widget", TypeKind::Class)
        .add_method(
            FunSpec::builder("Other")
                .is_constructor()
                .is_static()
                .body(CodeBlock::of("Initialize();", ()).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    assert!(matches!(
        wrong_name.validate(&CSharp::new()),
        Err(SigilStitchError::InvalidConstructorName { .. })
    ));
}

#[test]
fn built_in_constructors_reject_abstract_modifiers() {
    let abstract_constructor = FunSpec::builder("User")
        .is_constructor()
        .is_abstract()
        .build()
        .unwrap();

    for (lang, expected) in [
        (
            &Java::new() as &dyn sigil_stitch::lang::CodeLang,
            FunctionCapability::AbstractMethod,
        ),
        (&Cpp::new(), FunctionCapability::VirtualMethod),
    ] {
        let error = abstract_constructor
            .validate(lang, DeclarationContext::Member)
            .unwrap_err();
        assert!(matches!(
            error,
            SigilStitchError::UnsupportedFunctionCapabilities { capabilities, .. }
                if capabilities == vec![expected]
        ));
    }

    let override_constructor = FunSpec::builder("constructor")
        .is_constructor()
        .is_override()
        .build()
        .unwrap();
    let error = override_constructor
        .validate(&TypeScript::new(), DeclarationContext::Member)
        .unwrap_err();
    assert!(matches!(
        error,
        SigilStitchError::UnsupportedFunctionCapabilities { capabilities, .. }
            if capabilities == vec![FunctionCapability::Override]
    ));
}

#[test]
fn async_and_overriding_constructors_use_form_profiles() {
    for (lang, name) in [
        (
            &TypeScript::new() as &dyn sigil_stitch::lang::CodeLang,
            "constructor",
        ),
        (&Kotlin::new(), "constructor"),
    ] {
        let async_constructor = FunSpec::builder(name)
            .is_constructor()
            .is_async()
            .body(CodeBlock::of("initialize()", ()).unwrap())
            .build()
            .unwrap();
        let error = async_constructor
            .validate(lang, DeclarationContext::Member)
            .unwrap_err();
        assert!(matches!(
            error,
            SigilStitchError::UnsupportedFunctionCapabilities { capabilities, .. }
                if capabilities == vec![FunctionCapability::AsyncEffect]
        ));
    }

    let rust_async_constructor = FunSpec::builder("new")
        .is_constructor()
        .is_async()
        .body(CodeBlock::of("Self {}", ()).unwrap())
        .build()
        .unwrap();
    let output = render_function(
        &rust_async_constructor,
        &Rust::new(),
        DeclarationContext::Member,
    );
    assert!(output.starts_with("async fn new()"), "{output}");

    let swift_initializer = FunSpec::builder("init")
        .is_constructor()
        .is_async()
        .is_override()
        .body(CodeBlock::of("initialize()", ()).unwrap())
        .build()
        .unwrap();
    let output = render_function(
        &swift_initializer,
        &Swift::new(),
        DeclarationContext::Member,
    );
    assert!(output.starts_with("override init() async {"), "{output}");
}

#[test]
fn constructor_form_separates_generic_and_attribute_capabilities() {
    for (lang, name) in [
        (
            &TypeScript::new() as &dyn sigil_stitch::lang::CodeLang,
            "constructor",
        ),
        (&Kotlin::new(), "constructor"),
        (&CSharp::new(), "Widget"),
        (&Dart::new(), "Widget"),
    ] {
        let generic_constructor = FunSpec::builder(name)
            .is_constructor()
            .add_type_param(TypeParamSpec::new("T"))
            .body(CodeBlock::of("this.value = null;", ()).unwrap())
            .build()
            .unwrap();
        let error = generic_constructor
            .validate(lang, DeclarationContext::Member)
            .unwrap_err();
        assert!(matches!(
            error,
            SigilStitchError::UnsupportedFunctionCapabilities {
                form: FunctionForm::Constructor,
                capabilities,
                ..
            } if capabilities == vec![FunctionCapability::ParametricPolymorphism]
        ));
    }

    let java = render_function(
        &FunSpec::builder("Widget")
            .is_constructor()
            .add_type_param(TypeParamSpec::new("T"))
            .body(CodeBlock::of("this.value = null;", ()).unwrap())
            .build()
            .unwrap(),
        &Java::new(),
        DeclarationContext::Member,
    );
    assert!(java.starts_with("<T> Widget()"), "{java}");

    let rust = render_function(
        &FunSpec::builder("new")
            .is_constructor()
            .add_type_param(TypeParamSpec::new("T"))
            .returns(TypeName::primitive("Self"))
            .body(CodeBlock::of("Self", ()).unwrap())
            .build()
            .unwrap(),
        &Rust::new(),
        DeclarationContext::Member,
    );
    assert!(rust.starts_with("fn new<T>() -> Self"), "{rust}");

    let swift = render_function(
        &FunSpec::builder("init")
            .is_constructor()
            .add_type_param(TypeParamSpec::new("T"))
            .body(CodeBlock::of("initialize()", ()).unwrap())
            .build()
            .unwrap(),
        &Swift::new(),
        DeclarationContext::Member,
    );
    assert!(swift.starts_with("init<T>()"), "{swift}");

    let annotated_constructor = FunSpec::builder("constructor")
        .is_constructor()
        .annotation(CodeBlock::of("@inject", ()).unwrap())
        .build()
        .unwrap();
    let error = annotated_constructor
        .validate(&TypeScript::new(), DeclarationContext::Member)
        .unwrap_err();
    assert!(matches!(
        error,
        SigilStitchError::UnsupportedFunctionCapabilities {
            form: FunctionForm::Constructor,
            capabilities,
            ..
        } if capabilities == vec![FunctionCapability::Attributes]
    ));

    let java = render_function(
        &FunSpec::builder("Widget")
            .is_constructor()
            .annotation(CodeBlock::of("@Inject", ()).unwrap())
            .body(CodeBlock::of("this.value = null;", ()).unwrap())
            .build()
            .unwrap(),
        &Java::new(),
        DeclarationContext::Member,
    );
    assert!(java.starts_with("@Inject\nWidget()"), "{java}");
}

#[test]
fn explicit_return_types_fail_closed_when_the_form_cannot_render_them() {
    let typed_function = FunSpec::builder("value")
        .returns(TypeName::primitive("String"))
        .build()
        .unwrap();

    for lang in [
        &JavaScript::new() as &dyn sigil_stitch::lang::CodeLang,
        &Lua::new(),
    ] {
        let error = typed_function
            .validate(lang, DeclarationContext::TopLevel)
            .unwrap_err();
        assert!(matches!(
            error,
            SigilStitchError::UnsupportedFunctionCapabilities { capabilities, .. }
                if capabilities == vec![FunctionCapability::ExplicitReturnType]
        ));
    }

    for (lang, name) in [
        (&Cpp::new() as &dyn sigil_stitch::lang::CodeLang, "Widget"),
        (&Java::new(), "Widget"),
        (&TypeScript::new(), "constructor"),
        (&Swift::new(), "init"),
        (&Php::new(), "__construct"),
    ] {
        let typed_constructor = FunSpec::builder(name)
            .is_constructor()
            .returns(TypeName::primitive("Widget"))
            .build()
            .unwrap();
        let error = typed_constructor
            .validate(lang, DeclarationContext::Member)
            .unwrap_err();
        assert!(matches!(
            error,
            SigilStitchError::UnsupportedFunctionCapabilities {
                form: FunctionForm::Constructor,
                capabilities,
                ..
            } if capabilities == vec![FunctionCapability::ExplicitReturnType]
        ));
    }

    let python_none = FunSpec::builder("__init__")
        .is_constructor()
        .returns(TypeName::primitive("None"))
        .build()
        .unwrap();
    assert!(
        python_none
            .validate(&Python::new(), DeclarationContext::Member)
            .is_ok()
    );

    let python_value = FunSpec::builder("__init__")
        .is_constructor()
        .returns(TypeName::primitive("str"))
        .build()
        .unwrap();
    assert!(matches!(
        python_value
            .validate(&Python::new(), DeclarationContext::Member)
            .unwrap_err(),
        SigilStitchError::InvalidConstructorReturnType { .. }
    ));
}

#[test]
fn incompatible_method_modifiers_fail_closed_by_profile() {
    let cpp = Cpp::new();
    let csharp = CSharp::new();
    let dart = Dart::new();
    let java = Java::new();
    let swift = Swift::new();
    let typescript = TypeScript::new();
    let cases: Vec<(
        &dyn sigil_stitch::lang::CodeLang,
        FunSpec,
        Vec<FunctionCapability>,
    )> = vec![
        (
            &cpp,
            FunSpec::builder("work")
                .is_static()
                .is_abstract()
                .returns(TypeName::primitive("void"))
                .build()
                .unwrap(),
            vec![
                FunctionCapability::StaticMethod,
                FunctionCapability::VirtualMethod,
            ],
        ),
        (
            &cpp,
            FunSpec::builder("work")
                .is_static()
                .is_override()
                .returns(TypeName::primitive("void"))
                .build()
                .unwrap(),
            vec![
                FunctionCapability::StaticMethod,
                FunctionCapability::Override,
            ],
        ),
        (
            &csharp,
            FunSpec::builder("Work")
                .is_abstract()
                .is_async()
                .returns(TypeName::primitive("void"))
                .build()
                .unwrap(),
            vec![
                FunctionCapability::AbstractMethod,
                FunctionCapability::AsyncEffect,
            ],
        ),
        (
            &dart,
            FunSpec::builder("work")
                .is_abstract()
                .is_static()
                .returns(TypeName::primitive("void"))
                .build()
                .unwrap(),
            vec![
                FunctionCapability::AbstractMethod,
                FunctionCapability::StaticMethod,
            ],
        ),
        (
            &java,
            FunSpec::builder("work")
                .is_static()
                .is_override()
                .returns(TypeName::primitive("void"))
                .build()
                .unwrap(),
            vec![
                FunctionCapability::StaticMethod,
                FunctionCapability::Override,
            ],
        ),
        (
            &typescript,
            FunSpec::builder("work")
                .is_abstract()
                .is_async()
                .returns(TypeName::primitive("void"))
                .build()
                .unwrap(),
            vec![
                FunctionCapability::AbstractMethod,
                FunctionCapability::AsyncEffect,
            ],
        ),
        (
            &swift,
            FunSpec::builder("work")
                .is_static()
                .is_override()
                .returns(TypeName::primitive("void"))
                .build()
                .unwrap(),
            vec![
                FunctionCapability::StaticMethod,
                FunctionCapability::Override,
            ],
        ),
    ];

    for (lang, function, expected) in cases {
        let error = function
            .validate(lang, DeclarationContext::Member)
            .unwrap_err();
        assert!(matches!(
            error,
            SigilStitchError::IncompatibleFunctionCapabilities { capabilities, .. }
                if capabilities == expected
        ));
    }

    let php = render_function(
        &FunSpec::builder("work")
            .is_abstract()
            .is_static()
            .build()
            .unwrap(),
        &Php::new(),
        DeclarationContext::Member,
    );
    assert_eq!(php, "public abstract static function work();\n");
}

#[test]
fn php_supports_abstract_constructor_contracts() {
    let constructor = FunSpec::builder("__construct")
        .is_constructor()
        .is_abstract()
        .build()
        .unwrap();
    let output = render_function(&constructor, &Php::new(), DeclarationContext::Member);
    assert_eq!(output, "public abstract function __construct();\n");

    let interface_constructor = FunSpec::builder("__construct")
        .is_constructor()
        .is_abstract()
        .build()
        .unwrap();
    assert!(matches!(
        interface_constructor.validate(&Php::new(), DeclarationContext::InterfaceMember),
        Err(SigilStitchError::UnsupportedFunctionCapabilities {
            capabilities,
            ..
        }) if capabilities == vec![FunctionCapability::AbstractMethod]
    ));
}

#[test]
fn variadic_parameters_cannot_also_have_defaults() {
    let rest = ParameterSpec::builder("values", TypeName::primitive("string"))
        .variadic()
        .default_value(CodeBlock::of("[]", ()).unwrap())
        .build()
        .unwrap();
    let function = FunSpec::builder("collect").add_param(rest).build().unwrap();

    for lang in [
        &TypeScript::new() as &dyn sigil_stitch::lang::CodeLang,
        &JavaScript::new(),
    ] {
        let error = function
            .validate(lang, DeclarationContext::TopLevel)
            .unwrap_err();
        assert!(matches!(
            error,
            SigilStitchError::IncompatibleParameterCapabilities {
                parameter_name,
                capabilities,
                ..
            } if parameter_name == "values"
                && capabilities == vec![
                    FunctionCapability::VariadicParameters,
                    FunctionCapability::DefaultParameters,
                ]
        ));
    }
}

#[test]
fn required_parameters_cannot_follow_defaults_in_ordered_default_languages() {
    let defaulted = || {
        ParameterSpec::builder("first", TypeName::primitive("int"))
            .default_value(CodeBlock::of("0", ()).unwrap())
            .build()
            .unwrap()
    };
    let required = || ParameterSpec::new("second", TypeName::primitive("int")).unwrap();

    for (lang, context, name, return_type) in [
        (
            &Cpp::new() as &dyn sigil_stitch::lang::CodeLang,
            DeclarationContext::TopLevel,
            "work",
            Some(TypeName::primitive("void")),
        ),
        (
            &CSharp::new(),
            DeclarationContext::Member,
            "Work",
            Some(TypeName::primitive("void")),
        ),
        (&Python::new(), DeclarationContext::TopLevel, "work", None),
        (
            &TypeScript::new(),
            DeclarationContext::TopLevel,
            "work",
            Some(TypeName::primitive("void")),
        ),
        (
            &TypeScript::new(),
            DeclarationContext::Member,
            "work",
            Some(TypeName::primitive("void")),
        ),
    ] {
        let mut builder = FunSpec::builder(name)
            .add_param(defaulted())
            .add_param(required())
            .body(CodeBlock::of("return", ()).unwrap());
        if let Some(return_type) = return_type {
            builder = builder.returns(return_type);
        }
        let function = builder.build().unwrap();
        assert!(matches!(
            function.validate(lang, context),
            Err(SigilStitchError::RequiredParameterAfterDefault {
                parameter_name,
                ..
            }) if parameter_name == "second"
        ));
    }

    for (lang, name) in [
        (&Cpp::new() as &dyn sigil_stitch::lang::CodeLang, "Widget"),
        (&CSharp::new(), "Widget"),
        (&Python::new(), "__init__"),
        (&TypeScript::new(), "constructor"),
    ] {
        let constructor = FunSpec::builder(name)
            .is_constructor()
            .add_param(defaulted())
            .add_param(required())
            .body(CodeBlock::of("return", ()).unwrap())
            .build()
            .unwrap();
        assert!(matches!(
            constructor.validate(lang, DeclarationContext::Member),
            Err(SigilStitchError::RequiredParameterAfterDefault {
                parameter_name,
                ..
            }) if parameter_name == "second"
        ));
    }

    let javascript = FunSpec::builder("work")
        .add_param(
            ParameterSpec::builder("first", TypeName::primitive(""))
                .default_value(CodeBlock::of("0", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_param(ParameterSpec::new("second", TypeName::primitive("")).unwrap())
        .body(CodeBlock::of("return", ()).unwrap())
        .build()
        .unwrap();
    assert!(
        javascript
            .validate(&JavaScript::new(), DeclarationContext::TopLevel)
            .is_ok()
    );
}

#[test]
fn variadic_constructor_parameters_cannot_be_promoted_to_properties() {
    for parameter in [
        ParameterSpec::builder("values", TypeName::primitive("string[]"))
            .variadic()
            .is_property()
            .build()
            .unwrap(),
        ParameterSpec::builder("values", TypeName::primitive("string[]"))
            .variadic()
            .is_mutable_property()
            .build()
            .unwrap(),
    ] {
        let constructor = FunSpec::builder("constructor")
            .is_constructor()
            .add_param(parameter)
            .body(CodeBlock::of("return", ()).unwrap())
            .build()
            .unwrap();
        assert!(matches!(
            constructor.validate(&TypeScript::new(), DeclarationContext::Member),
            Err(SigilStitchError::IncompatibleParameterCapabilities {
                parameter_name,
                capabilities,
                ..
            }) if parameter_name == "values"
                && capabilities == vec![
                    FunctionCapability::VariadicParameters,
                    FunctionCapability::ConstructorProperties,
                ]
        ));
    }
}

#[test]
fn constructor_property_mutability_markers_are_mutually_exclusive() {
    let parameter = || {
        ParameterSpec::builder("value", TypeName::primitive("String"))
            .is_property()
            .is_mutable_property()
            .build()
            .unwrap()
    };

    for lang in [
        &TypeScript::new() as &dyn sigil_stitch::lang::CodeLang,
        &Kotlin::new(),
    ] {
        let constructor = FunSpec::builder("constructor")
            .is_constructor()
            .add_param(parameter())
            .body(CodeBlock::of("return", ()).unwrap())
            .build()
            .unwrap();
        assert!(matches!(
            constructor.validate(lang, DeclarationContext::Member),
            Err(SigilStitchError::ConflictingConstructorPropertyMutability {
                function_name,
                parameter_name,
            }) if function_name == "constructor" && parameter_name == "value"
        ));
    }
}

#[test]
fn typescript_mutable_constructor_properties_render_as_promoted_properties() {
    let property = ParameterSpec::builder("value", TypeName::primitive("string"))
        .is_mutable_property()
        .build()
        .unwrap();
    let constructor = FunSpec::builder("constructor")
        .is_constructor()
        .add_param(property)
        .body(CodeBlock::of("return", ()).unwrap())
        .build()
        .unwrap();
    let output = render_function(&constructor, &TypeScript::new(), DeclarationContext::Member);
    assert!(
        output.starts_with("constructor(public value: string)"),
        "{output}"
    );
}

#[test]
fn csharp_static_abstract_interface_members_are_supported() {
    let parameter = ParameterSpec::new("input", TypeName::primitive("string")).unwrap();
    let contract = TypeSpec::builder("IParsable", TypeKind::Interface)
        .add_method(
            FunSpec::builder("Parse")
                .is_static()
                .is_abstract()
                .add_param(parameter)
                .returns(TypeName::primitive("bool"))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    let output = FileSpec::builder_with("IParsable.cs", CSharp::new())
        .add_type(contract)
        .build()
        .unwrap()
        .render(80)
        .unwrap();
    assert!(output.contains("static"), "{output}");
    assert!(output.contains("abstract"), "{output}");
    assert!(output.contains("bool Parse(string input);"), "{output}");
}

#[test]
fn scala_function_annotations_are_supported_in_all_contexts() {
    let function = FunSpec::builder("work")
        .annotate(AnnotationSpec::new("deprecated"))
        .returns(TypeName::primitive("Unit"))
        .body(CodeBlock::of("()", ()).unwrap())
        .build()
        .unwrap();

    for context in [
        DeclarationContext::TopLevel,
        DeclarationContext::Member,
        DeclarationContext::InterfaceMember,
    ] {
        let output = render_function(&function, &sigil_stitch::lang::scala::Scala::new(), context);
        assert!(output.starts_with("@deprecated\n"), "{output}");
    }
}

#[test]
fn required_return_types_fail_closed_without_rejecting_cpp_destructors() {
    let c_function = FunSpec::builder("work").build().unwrap();
    assert!(matches!(
        c_function.validate(&C::new(), DeclarationContext::TopLevel),
        Err(SigilStitchError::MissingRequiredFunctionCapabilities {
            capabilities,
            ..
        }) if capabilities == vec![FunctionCapability::ExplicitReturnType]
    ));

    let member = |body| {
        let mut method = FunSpec::builder("work");
        if let Some(body) = body {
            method = method.body(body);
        }
        TypeSpec::builder("Widget", TypeKind::Class)
            .add_method(method.build().unwrap())
            .build()
            .unwrap()
    };
    for (lang, type_spec) in [
        (
            &Java::new() as &dyn sigil_stitch::lang::CodeLang,
            member(Some(CodeBlock::of("return;", ()).unwrap())),
        ),
        (&CSharp::new(), member(None)),
        (&Cpp::new(), member(None)),
    ] {
        assert!(matches!(
            type_spec.validate(lang),
            Err(SigilStitchError::MissingRequiredFunctionCapabilities {
                capabilities,
                ..
            }) if capabilities == vec![FunctionCapability::ExplicitReturnType]
        ));
    }

    let destructor = FunSpec::builder("~Widget")
        .suffix("= default")
        .build()
        .unwrap();
    assert_eq!(
        render_function(&destructor, &Cpp::new(), DeclarationContext::Member),
        "~Widget() = default;\n"
    );
}

#[test]
fn cpp_destructors_require_zero_parameters_and_the_declaring_type_name() {
    let parameterized = FunSpec::builder("~Widget")
        .is_constructor()
        .add_param(ParameterSpec::new("value", TypeName::primitive("int")).unwrap())
        .build()
        .unwrap();
    assert!(matches!(
        parameterized.validate(&Cpp::new(), DeclarationContext::Member),
        Err(SigilStitchError::TooManyFunctionParameters {
            form: FunctionForm::Destructor,
            maximum: 0,
            actual: 1,
            ..
        })
    ));

    let mismatched = TypeSpec::builder("Widget", TypeKind::Class)
        .add_method(FunSpec::builder("~Other").build().unwrap())
        .build()
        .unwrap();
    assert!(matches!(
        mismatched.validate(&Cpp::new()),
        Err(SigilStitchError::InvalidDestructorName {
            type_name,
            destructor_name,
            ..
        }) if type_name == "Widget" && destructor_name == "~Other"
    ));
}

#[test]
fn c_family_and_java_require_types_on_declared_parameters() {
    let parameter = ParameterSpec::new("value", TypeName::primitive("")).unwrap();
    let c = FunSpec::builder("work")
        .add_param(parameter.clone())
        .returns(TypeName::primitive("void"))
        .build()
        .unwrap();
    let cpp = FunSpec::builder("work")
        .add_param(parameter.clone())
        .returns(TypeName::primitive("void"))
        .build()
        .unwrap();
    let cpp_constructor = FunSpec::builder("Widget")
        .is_constructor()
        .add_param(parameter.clone())
        .build()
        .unwrap();
    let java = FunSpec::builder("work")
        .add_param(parameter.clone())
        .returns(TypeName::primitive("void"))
        .body(CodeBlock::of("return;", ()).unwrap())
        .build()
        .unwrap();
    let java_constructor = FunSpec::builder("Widget")
        .is_constructor()
        .add_param(parameter.clone())
        .body(CodeBlock::of("return;", ()).unwrap())
        .build()
        .unwrap();
    let csharp = FunSpec::builder("Work")
        .add_param(parameter.clone())
        .returns(TypeName::primitive("void"))
        .build()
        .unwrap();
    let csharp_constructor = FunSpec::builder("Widget")
        .is_constructor()
        .add_param(parameter)
        .build()
        .unwrap();

    for (lang, function, context) in [
        (
            &C::new() as &dyn sigil_stitch::lang::CodeLang,
            &c,
            DeclarationContext::TopLevel,
        ),
        (&Cpp::new(), &cpp, DeclarationContext::Member),
        (&Cpp::new(), &cpp_constructor, DeclarationContext::Member),
        (&Java::new(), &java, DeclarationContext::Member),
        (&Java::new(), &java_constructor, DeclarationContext::Member),
        (&CSharp::new(), &csharp, DeclarationContext::Member),
        (
            &CSharp::new(),
            &csharp_constructor,
            DeclarationContext::Member,
        ),
    ] {
        assert!(matches!(
            function.validate(lang, context),
            Err(SigilStitchError::MissingRequiredFunctionCapabilities {
                capabilities,
                ..
            }) if capabilities == vec![FunctionCapability::TypedParameters]
        ));
    }
}

#[test]
fn body_policies_reject_missing_implementations_and_interface_bodies() {
    let java_method = FunSpec::builder("work")
        .returns(TypeName::primitive("void"))
        .build()
        .unwrap();
    let java_constructor = FunSpec::builder("Widget").is_constructor().build().unwrap();

    for function in [&java_method, &java_constructor] {
        assert!(matches!(
            function.validate(&Java::new(), DeclarationContext::Member),
            Err(SigilStitchError::FunctionBodyRequired { .. })
        ));
    }

    let interface_method = FunSpec::builder("work")
        .body(CodeBlock::of("return;", ()).unwrap())
        .build()
        .unwrap();
    for lang in [
        &TypeScript::new() as &dyn sigil_stitch::lang::CodeLang,
        &Php::new(),
        &Swift::new(),
    ] {
        assert!(matches!(
            interface_method.validate(lang, DeclarationContext::InterfaceMember),
            Err(SigilStitchError::FunctionBodyForbidden { .. })
        ));
    }

    let abstract_java_method = FunSpec::builder("work")
        .is_abstract()
        .returns(TypeName::primitive("void"))
        .build()
        .unwrap();
    assert!(
        abstract_java_method
            .validate(&Java::new(), DeclarationContext::Member)
            .is_ok()
    );

    let java_interface_body = FunSpec::builder("work")
        .returns(TypeName::primitive("void"))
        .body(CodeBlock::of("return;", ()).unwrap())
        .build()
        .unwrap();
    assert!(matches!(
        java_interface_body.validate(&Java::new(), DeclarationContext::InterfaceMember),
        Err(SigilStitchError::FunctionBodyForbidden { .. })
    ));

    let java_static_declaration = FunSpec::builder("work")
        .is_static()
        .returns(TypeName::primitive("void"))
        .build()
        .unwrap();
    assert!(matches!(
        java_static_declaration.validate(&Java::new(), DeclarationContext::InterfaceMember),
        Err(SigilStitchError::FunctionBodyRequired { .. })
    ));

    let java_static_definition = FunSpec::builder("work")
        .is_static()
        .returns(TypeName::primitive("void"))
        .body(CodeBlock::of("return;", ()).unwrap())
        .build()
        .unwrap();
    assert!(
        java_static_definition
            .validate(&Java::new(), DeclarationContext::InterfaceMember)
            .is_ok()
    );
}

#[test]
fn static_csharp_and_dart_contract_members_require_bodies() {
    let declaration = FunSpec::builder("work")
        .is_static()
        .returns(TypeName::primitive("void"))
        .build()
        .unwrap();
    let definition = FunSpec::builder("work")
        .is_static()
        .returns(TypeName::primitive("void"))
        .body(CodeBlock::of("return;", ()).unwrap())
        .build()
        .unwrap();

    for lang in [
        &CSharp::new() as &dyn sigil_stitch::lang::CodeLang,
        &Dart::new(),
    ] {
        assert!(matches!(
            declaration.validate(lang, DeclarationContext::InterfaceMember),
            Err(SigilStitchError::FunctionBodyRequired {
                context: FunctionContext::InterfaceMember,
                ..
            })
        ));
        assert!(
            definition
                .validate(lang, DeclarationContext::InterfaceMember)
                .is_ok()
        );
    }
}

#[test]
fn kotlin_secondary_constructors_require_bodies() {
    let declaration = FunSpec::builder("constructor").build().unwrap();
    let delegated = FunSpec::builder("constructor")
        .delegation(CodeBlock::of("this(0)", ()).unwrap())
        .build()
        .unwrap();

    for constructor in [&declaration, &delegated] {
        assert!(matches!(
            constructor.validate(&Kotlin::new(), DeclarationContext::Member),
            Err(SigilStitchError::FunctionBodyRequired {
                form: FunctionForm::Constructor,
                ..
            })
        ));
    }
}

#[test]
fn shell_function_specs_reject_declared_parameters() {
    let function = FunSpec::builder("load")
        .add_param(ParameterSpec::new("value", TypeName::primitive("")).unwrap())
        .body(CodeBlock::of("return", ()).unwrap())
        .build()
        .unwrap();

    for lang in [
        &sigil_stitch::lang::bash::Bash::new() as &dyn sigil_stitch::lang::CodeLang,
        &sigil_stitch::lang::zsh::Zsh::new(),
    ] {
        assert!(matches!(
            function.validate(lang, DeclarationContext::TopLevel),
            Err(SigilStitchError::TooManyFunctionParameters {
                maximum: 0,
                actual: 1,
                ..
            })
        ));
    }
}

#[test]
fn untyped_languages_reject_parameter_annotations() {
    let function = FunSpec::builder("work")
        .add_param(ParameterSpec::new("value", TypeName::primitive("string")).unwrap())
        .build()
        .unwrap();

    for lang in [
        &JavaScript::new() as &dyn sigil_stitch::lang::CodeLang,
        &Lua::new(),
        &Ruby::new(),
    ] {
        assert!(matches!(
            function.validate(lang, DeclarationContext::TopLevel),
            Err(SigilStitchError::UnsupportedFunctionCapabilities {
                capabilities,
                ..
            }) if capabilities == vec![FunctionCapability::TypedParameters]
        ));
    }
}

#[test]
fn rest_parameters_are_unique_and_last() {
    let rest = || {
        ParameterSpec::builder("rest", TypeName::primitive(""))
            .variadic()
            .build()
            .unwrap()
    };
    let duplicate = FunSpec::builder("collect")
        .add_param(rest())
        .add_param(rest())
        .build()
        .unwrap();
    assert!(matches!(
        duplicate.validate(&JavaScript::new(), DeclarationContext::TopLevel),
        Err(SigilStitchError::MultipleVariadicParameters { .. })
    ));

    let trailing = FunSpec::builder("collect")
        .add_param(rest())
        .add_param(ParameterSpec::new("after", TypeName::primitive("")).unwrap())
        .build()
        .unwrap();
    assert!(matches!(
        trailing.validate(&JavaScript::new(), DeclarationContext::TopLevel),
        Err(SigilStitchError::VariadicParameterNotLast { .. })
    ));
}

#[test]
fn kotlin_places_function_type_parameters_before_the_name() {
    let function = FunSpec::builder("identity")
        .add_type_param(TypeParamSpec::new("T"))
        .body(CodeBlock::of("TODO()", ()).unwrap())
        .build()
        .unwrap();
    let output = render_function(&function, &Kotlin::new(), DeclarationContext::TopLevel);
    assert!(output.contains("fun <T> identity()"), "{output}");
}

#[test]
fn typescript_rejects_invalid_abstract_combinations() {
    let abstract_static = FunSpec::builder("work")
        .is_abstract()
        .is_static()
        .build()
        .unwrap();
    assert!(matches!(
        abstract_static.validate(&TypeScript::new(), DeclarationContext::Member),
        Err(SigilStitchError::IncompatibleFunctionCapabilities {
            capabilities,
            ..
        }) if capabilities == vec![
            FunctionCapability::AbstractMethod,
            FunctionCapability::StaticMethod,
        ]
    ));

    let defaulted = ParameterSpec::builder("value", TypeName::primitive("string"))
        .default_value(CodeBlock::of("\"value\"", ()).unwrap())
        .build()
        .unwrap();
    let abstract_default = FunSpec::builder("work")
        .is_abstract()
        .add_param(defaulted)
        .build()
        .unwrap();
    assert!(matches!(
        abstract_default.validate(&TypeScript::new(), DeclarationContext::Member),
        Err(SigilStitchError::IncompatibleFunctionCapabilities {
            capabilities,
            ..
        }) if capabilities == vec![
            FunctionCapability::AbstractMethod,
            FunctionCapability::DefaultParameters,
        ]
    ));
}

#[test]
fn csharp_interface_async_fails_closed() {
    let function = FunSpec::builder("Work")
        .is_async()
        .returns(TypeName::primitive("Task"))
        .build()
        .unwrap();
    assert!(matches!(
        function.validate(&CSharp::new(), DeclarationContext::InterfaceMember),
        Err(SigilStitchError::UnsupportedFunctionCapabilities {
            capabilities,
            ..
        }) if capabilities == vec![FunctionCapability::AsyncEffect]
    ));
}

#[test]
fn permissive_body_style_delegation_without_a_body_remains_a_declaration() {
    let constructor = FunSpec::builder("Legacy")
        .is_constructor()
        .delegation(CodeBlock::of("legacy_delegate()", ()).unwrap())
        .build()
        .unwrap();
    let output = render_function(&constructor, &LegacyVirtualLang, DeclarationContext::Member);
    assert_eq!(output, "Legacy();\n");
}

#[test]
fn kotlin_secondary_constructors_reject_property_parameters() {
    let property = ParameterSpec::builder("name", TypeName::primitive("String"))
        .is_property()
        .build()
        .unwrap();
    let constructor = FunSpec::builder("constructor")
        .is_constructor()
        .add_param(property)
        .build()
        .unwrap();

    let error = constructor
        .emit(&Kotlin::new(), DeclarationContext::Member)
        .unwrap_err();
    assert!(matches!(
        error,
        SigilStitchError::UnsupportedFunctionCapabilities { capabilities, .. }
            if capabilities == vec![FunctionCapability::ConstructorProperties]
    ));
}

#[test]
fn interface_methods_use_the_interface_profile() {
    let method = FunSpec::builder("Bad").is_constructor().build().unwrap();
    let direct_error = method
        .emit(&Java::new(), DeclarationContext::InterfaceMember)
        .unwrap_err();
    let invalid = TypeSpec::builder("Bad", TypeKind::Interface)
        .add_method(method)
        .build()
        .unwrap();
    let type_error = invalid.emit(&Java::new()).unwrap_err();
    let file = FileSpec::builder_with("Bad.java", Java::new())
        .add_type(invalid)
        .build()
        .unwrap();
    let SigilStitchError::FileSpecValidation { errors, .. } = file.render(80).unwrap_err() else {
        panic!("expected FileSpecValidation");
    };

    for error in [&direct_error, &type_error, &errors[0]] {
        assert!(matches!(
            error,
            SigilStitchError::UnsupportedFunctionForm {
                context: FunctionContext::InterfaceMember,
                form: FunctionForm::Constructor,
                ..
            }
        ));
    }
}

#[test]
fn corrected_built_in_profiles_accept_existing_emission_paths() {
    let async_fun = async_function("load");
    assert!(
        async_fun
            .validate(&Rust::new(), DeclarationContext::TopLevel)
            .is_ok()
    );
    assert!(
        async_fun
            .validate(&Rust::new(), DeclarationContext::Member)
            .is_ok()
    );

    let static_method = FunSpec::builder("shared")
        .is_static()
        .body(CodeBlock::of("return", ()).unwrap())
        .build()
        .unwrap();
    assert!(
        static_method
            .validate(&Dart::new(), DeclarationContext::Member)
            .is_ok()
    );
    assert!(
        static_method
            .validate(&Swift::new(), DeclarationContext::Member)
            .is_ok()
    );

    let override_method = FunSpec::builder("render")
        .is_override()
        .body(CodeBlock::of("return", ()).unwrap())
        .build()
        .unwrap();
    assert!(
        override_method
            .validate(&TypeScript::new(), DeclarationContext::Member)
            .is_ok()
    );

    let property = ParameterSpec::builder("name", TypeName::primitive("string"))
        .is_property()
        .build()
        .unwrap();
    let constructor = FunSpec::builder("constructor")
        .is_constructor()
        .add_param(property)
        .body(CodeBlock::of("this.name = name", ()).unwrap())
        .build()
        .unwrap();
    assert!(
        constructor
            .validate(&TypeScript::new(), DeclarationContext::Member)
            .is_ok()
    );
}

#[test]
fn file_validation_aggregates_top_level_function_errors() {
    let file = FileSpec::builder_with("load.bash", sigil_stitch::lang::bash::Bash::new())
        .add_function(async_function("load"))
        .add_function(async_function("reload"))
        .build()
        .unwrap();

    let error = file.validate().unwrap_err();
    let SigilStitchError::FileSpecValidation {
        filename,
        error_count,
        errors,
    } = error
    else {
        panic!("expected FileSpecValidation, got {error:?}");
    };

    assert_eq!(filename, "load.bash");
    assert_eq!(error_count, 2);
    assert_eq!(errors.len(), 2);
    assert!(errors.iter().all(|error| matches!(
        error,
        SigilStitchError::UnsupportedFunctionCapabilities { .. }
    )));
}

#[test]
fn type_error_does_not_hide_contained_method_error() {
    let invalid = TypeSpec::builder("Bad", TypeKind::Class)
        .add_method(async_function("load"))
        .build()
        .unwrap();
    let file = FileSpec::builder_with("bad.bash", sigil_stitch::lang::bash::Bash::new())
        .add_type(invalid)
        .build()
        .unwrap();

    let SigilStitchError::FileSpecValidation {
        error_count,
        errors,
        ..
    } = file.validate().unwrap_err()
    else {
        panic!("expected FileSpecValidation");
    };

    assert_eq!(error_count, 2);
    assert!(matches!(
        errors[0],
        SigilStitchError::UnsupportedTypeKind { .. }
    ));
    assert!(matches!(
        errors[1],
        SigilStitchError::UnsupportedFunctionContext {
            context: FunctionContext::Member,
            ..
        }
    ));
}

#[test]
fn csharp_direct_type_parameter_bounds_use_where_clauses() {
    let function = FunSpec::builder("Convert")
        .add_type_param(TypeParamSpec::new("T").with_bound(TypeName::primitive("IFoo")))
        .returns(TypeName::primitive("void"))
        .body(CodeBlock::of("return;", ()).unwrap())
        .build()
        .unwrap();

    let output = render_function(&function, &CSharp::new(), DeclarationContext::Member);
    assert!(output.contains("Convert<T>()"), "{output}");
    assert!(!output.contains("<T : IFoo>"), "{output}");
    assert!(output.contains("where T : IFoo"), "{output}");
}

#[test]
fn csharp_rejects_constraints_on_non_parameter_subjects() {
    let function = FunSpec::builder("Convert")
        .add_type_param(TypeParamSpec::new("T"))
        .add_where_constraint(
            TypeName::generic(TypeName::primitive("List"), vec![TypeName::primitive("T")]),
            vec![TypeName::primitive("IFoo")],
        )
        .returns(TypeName::primitive("void"))
        .body(CodeBlock::of("return;", ()).unwrap())
        .build()
        .unwrap();

    assert!(matches!(
        function.validate(&CSharp::new(), DeclarationContext::Member),
        Err(SigilStitchError::InvalidFunctionConstraintSubject { .. })
    ));
}

#[test]
fn kotlin_multiple_type_parameter_bounds_use_a_where_clause() {
    let function = FunSpec::builder("convert")
        .add_type_param(
            TypeParamSpec::new("T")
                .with_bound(TypeName::primitive("IFoo"))
                .with_bound(TypeName::primitive("IBar")),
        )
        .body(CodeBlock::of("TODO()", ()).unwrap())
        .build()
        .unwrap();

    let output = render_function(&function, &Kotlin::new(), DeclarationContext::TopLevel);
    assert!(output.contains("fun <T> convert()"), "{output}");
    assert!(output.contains("where T : IFoo, T : IBar"), "{output}");
    assert!(!output.contains("<T : IFoo, IBar>"), "{output}");
}

#[test]
fn strict_local_lowerers_preserve_or_reject_interface_visibility() {
    let public_typescript = FunSpec::builder("work")
        .visibility(Visibility::Public)
        .returns(TypeName::primitive("void"))
        .build()
        .unwrap();
    let output = render_function(
        &public_typescript,
        &TypeScript::new(),
        DeclarationContext::InterfaceMember,
    );
    assert!(!output.contains("public "), "{output}");

    let public_csharp = FunSpec::builder("Work")
        .visibility(Visibility::Public)
        .returns(TypeName::primitive("void"))
        .build()
        .unwrap();
    let output = render_function(
        &public_csharp,
        &CSharp::new(),
        DeclarationContext::InterfaceMember,
    );
    assert!(!output.contains("public "), "{output}");

    let public_kotlin = FunSpec::builder("work")
        .visibility(Visibility::Public)
        .returns(TypeName::primitive("Unit"))
        .build()
        .unwrap();
    let output = render_function(
        &public_kotlin,
        &Kotlin::new(),
        DeclarationContext::InterfaceMember,
    );
    assert!(!output.contains("public "), "{output}");

    for visibility in [Visibility::Private, Visibility::Protected] {
        for lang in [
            &TypeScript::new() as &dyn CodeLang,
            &CSharp::new(),
            &Kotlin::new(),
        ] {
            let function = FunSpec::builder("work")
                .visibility(visibility)
                .returns(TypeName::primitive("void"))
                .build()
                .unwrap();
            assert!(matches!(
                function.validate(lang, DeclarationContext::InterfaceMember),
                Err(SigilStitchError::InvalidFunctionVisibility { .. })
            ));
        }
    }
}

#[test]
fn cpp_rejects_explicit_function_visibility() {
    let function = FunSpec::builder("work")
        .visibility(Visibility::Public)
        .returns(TypeName::primitive("void"))
        .build()
        .unwrap();

    assert!(matches!(
        function.validate(&Cpp::new(), DeclarationContext::Member),
        Err(SigilStitchError::InvalidFunctionVisibility {
            visibility: Visibility::Public,
            ..
        })
    ));

    assert!(
        function
            .validate(&Cpp::new(), DeclarationContext::TopLevel)
            .is_ok(),
        "public top-level visibility maps to ordinary external linkage"
    );
}

#[test]
fn kotlin_rejects_protected_top_level_functions() {
    let protected_kotlin = FunSpec::builder("work")
        .visibility(Visibility::Protected)
        .body(CodeBlock::of("TODO()", ()).unwrap())
        .build()
        .unwrap();
    assert!(matches!(
        protected_kotlin.validate(&Kotlin::new(), DeclarationContext::TopLevel),
        Err(SigilStitchError::InvalidFunctionVisibility {
            visibility: Visibility::Protected,
            ..
        })
    ));
}

#[test]
fn compatibility_lowerers_reject_unrepresentable_visibility() {
    for lang in [
        &Dart::new() as &dyn CodeLang,
        &Haskell::new(),
        &Lua::new(),
        &Python::new(),
        &sigil_stitch::lang::bash::Bash::new(),
        &sigil_stitch::lang::ocaml::OCaml::new(),
        &sigil_stitch::lang::zsh::Zsh::new(),
    ] {
        let function = FunSpec::builder("work")
            .visibility(Visibility::Private)
            .build()
            .unwrap();
        assert!(matches!(
            function.validate(lang, DeclarationContext::TopLevel),
            Err(SigilStitchError::InvalidFunctionVisibility {
                visibility: Visibility::Private,
                ..
            })
        ));
    }

    let private_javascript = FunSpec::builder("work")
        .visibility(Visibility::Private)
        .body(CodeBlock::of("return", ()).unwrap())
        .build()
        .unwrap();
    assert!(matches!(
        private_javascript.validate(&JavaScript::new(), DeclarationContext::Member),
        Err(SigilStitchError::InvalidFunctionVisibility {
            visibility: Visibility::Private,
            ..
        })
    ));
}

#[test]
fn go_visibility_must_match_identifier_export_status() {
    for (name, visibility) in [
        ("hidden", Visibility::Public),
        ("Exported", Visibility::Private),
    ] {
        let function = FunSpec::builder(name)
            .visibility(visibility)
            .body(CodeBlock::of("return", ()).unwrap())
            .build()
            .unwrap();
        assert!(matches!(
            function.validate(&Go::new(), DeclarationContext::TopLevel),
            Err(SigilStitchError::InvalidFunctionVisibility { .. })
        ));
    }

    let exported = FunSpec::builder("Exported")
        .visibility(Visibility::Public)
        .body(CodeBlock::of("return", ()).unwrap())
        .build()
        .unwrap();
    assert!(
        exported
            .validate(&Go::new(), DeclarationContext::TopLevel)
            .is_ok()
    );
}

#[test]
fn csharp_inherited_function_visibility_emits_no_modifier() {
    let function = FunSpec::builder("Work")
        .returns(TypeName::primitive("void"))
        .body(CodeBlock::of("return;", ()).unwrap())
        .build()
        .unwrap();

    let output = render_function(&function, &CSharp::new(), DeclarationContext::Member);
    assert!(output.starts_with("void Work()"), "{output}");
    assert!(!output.contains("internal "), "{output}");
}

#[test]
fn scala_rejects_crate_visibility_instead_of_narrowing_it() {
    let function = FunSpec::builder("work")
        .visibility(Visibility::PublicCrate)
        .body(CodeBlock::of("()", ()).unwrap())
        .build()
        .unwrap();

    assert!(matches!(
        function.validate(
            &sigil_stitch::lang::scala::Scala::new(),
            DeclarationContext::Member,
        ),
        Err(SigilStitchError::InvalidFunctionVisibility {
            visibility: Visibility::PublicCrate,
            ..
        })
    ));
}

#[test]
fn kotlin_inherited_function_visibility_emits_no_modifier() {
    let function = FunSpec::builder("work")
        .returns(TypeName::primitive("Unit"))
        .body(CodeBlock::of("return", ()).unwrap())
        .build()
        .unwrap();

    let output = render_function(&function, &Kotlin::new(), DeclarationContext::Member);
    assert!(output.starts_with("fun work()"), "{output}");
    assert!(!output.contains("internal "), "{output}");
}

#[test]
fn scala_rejects_inexact_function_visibility_mappings() {
    for (context, visibility) in [
        (DeclarationContext::Member, Visibility::PublicSuper),
        (DeclarationContext::TopLevel, Visibility::Protected),
    ] {
        let function = FunSpec::builder("work")
            .visibility(visibility)
            .body(CodeBlock::of("()", ()).unwrap())
            .build()
            .unwrap();

        assert!(matches!(
            function.validate(&sigil_stitch::lang::scala::Scala::new(), context),
            Err(SigilStitchError::InvalidFunctionVisibility { .. })
        ));
    }
}

#[test]
fn swift_rejects_parent_module_visibility_instead_of_file_visibility() {
    let function = FunSpec::builder("work")
        .visibility(Visibility::PublicSuper)
        .body(CodeBlock::of("return", ()).unwrap())
        .build()
        .unwrap();

    assert!(matches!(
        function.validate(&Swift::new(), DeclarationContext::Member),
        Err(SigilStitchError::InvalidFunctionVisibility {
            visibility: Visibility::PublicSuper,
            ..
        })
    ));
}
