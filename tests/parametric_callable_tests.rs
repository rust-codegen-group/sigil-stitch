use sigil_stitch::code_renderer::CodeRenderer;
use sigil_stitch::import::ImportGroup;
use sigil_stitch::lang::CodeLang;
use sigil_stitch::lang::haskell::Haskell;
use sigil_stitch::lang::typescript::TypeScript;
use sigil_stitch::prelude::*;

#[path = "shared/languages.rs"]
mod languages_registry;

#[test]
fn callable_return_sequences_keep_native_empty_and_cardinality_rules() {
    let parameter = CallableParam::Single {
        name: None,
        type_name: TypeName::primitive("Input"),
        presence: CallableParamPresence::Required,
    };
    for descriptor in languages_registry::BUILT_IN_LANGUAGES {
        let lang = descriptor.adapter();
        let native = match descriptor.id {
            "cpp" | "dart" | "typescript" => Some("void"),
            "kotlin" | "scala" => Some("Unit"),
            "swift" => Some("Void"),
            "python" => Some("None"),
            "haskell" | "rust" => Some("()"),
            "ocaml" => Some("unit"),
            "go" => Some(""),
            _ => None,
        };
        let empty = TypeName::callable(vec![parameter.clone()], vec![]);
        let multiple = TypeName::callable(
            vec![parameter.clone()],
            vec![TypeName::primitive("First"), TypeName::primitive("Second")],
        );
        if let Some(native) = native {
            let empty_output = render_type(lang.as_ref(), &empty);
            if descriptor.id == "go" {
                assert_eq!(empty_output, "func(Input)");
                assert_eq!(
                    render_type(lang.as_ref(), &multiple),
                    "func(Input) (First, Second)"
                );
            } else {
                assert_eq!(
                    empty_output,
                    render_type(
                        lang.as_ref(),
                        &TypeName::callable(
                            vec![parameter.clone()],
                            vec![TypeName::primitive(native)]
                        )
                    ),
                    "{}",
                    descriptor.id
                );
                assert!(
                    matches!(
                        lang.lower_type_name(&multiple),
                        Err(SigilStitchError::UnsupportedTypeName { .. })
                    ),
                    "{}",
                    descriptor.id
                );
            }
        } else {
            for ty in [empty, multiple] {
                assert!(
                    matches!(
                        lang.lower_type_name(&ty),
                        Err(SigilStitchError::UnsupportedTypeName { .. })
                    ),
                    "{}",
                    descriptor.id
                );
            }
        }
    }
    for id in ["haskell", "ocaml"] {
        assert!(
            languages_registry::adapter_for(id)
                .lower_type_name(&TypeName::callable(vec![], vec![]))
                .unwrap_err()
                .to_string()
                .contains("nullary")
        );
    }
}

#[test]
fn go_nested_callable_results_preserve_import_aliases_and_full_file_layout() {
    let returns = vec![
        TypeName::importable("alpha", "Value"),
        TypeName::importable("beta", "Value"),
    ];
    let callable = TypeName::callable(vec![], returns.clone());
    let decoded: TypeName =
        serde_json::from_value(serde_json::to_value(&callable).unwrap()).unwrap();
    assert_eq!(decoded, callable);
    for width in [8, 120] {
        let file = FileSpec::builder("results.go")
            .add_function(
                FunSpec::builder("fetch")
                    .returns(returns.clone())
                    .body(CodeBlock::of("body", ()).unwrap())
                    .build()
                    .unwrap(),
            )
            .add_code(CodeBlock::of("var callbacks %T", TypeName::slice(callable.clone())).unwrap())
            .build()
            .unwrap();
        let output = file.render(width).unwrap();
        assert!(
            output.contains("\"alpha\"") && output.contains("\"beta\""),
            "{output}"
        );
        let collapsed = output.split_whitespace().collect::<String>();
        assert!(output.contains("BetaValue \"beta\""), "{output}");
        assert!(
            collapsed.contains("fetch()(alpha.Value,BetaValue.Value)"),
            "{output}"
        );
        assert!(
            collapsed.contains("[]func()(alpha.Value,BetaValue.Value)"),
            "{output}"
        );
    }
}

#[test]
fn later_callable_result_slots_are_validated_and_lowered() {
    let lang = languages_registry::adapter_for("go");
    let invalid = TypeName::callable(
        vec![],
        vec![TypeName::primitive("int"), TypeName::parameter("")],
    );
    let render = |ty| {
        FileSpec::builder("invalid.go")
            .add_code(CodeBlock::of("%T", ty).unwrap())
            .build()
            .unwrap()
            .render(120)
    };
    assert!(
        matches!(render(invalid.clone()), Err(SigilStitchError::InvalidTypeName { context, .. }) if context.contains("callable.returns[1]"))
    );
    let unsupported = TypeName::callable(
        vec![],
        vec![
            TypeName::primitive("int"),
            TypeName::tuple(vec![TypeName::primitive("bool")]),
        ],
    );
    assert!(matches!(
        lang.lower_type_name(&unsupported),
        Err(SigilStitchError::UnsupportedTypeName { .. })
    ));
    let encoded = serde_json::to_value(&invalid).unwrap();
    let decoded: TypeName = serde_json::from_value(encoded).unwrap();
    assert!(render(decoded).is_err());
}

#[test]
fn go_complete_files_lower_nested_return_sequences_at_both_widths() {
    for returns in [
        vec![],
        vec![TypeName::primitive("int")],
        vec![TypeName::primitive("int"), TypeName::primitive("bool")],
    ] {
        let callable = TypeName::callable(vec![], returns.clone());
        let nested =
            TypeName::callable(vec![], vec![callable.clone(), TypeName::primitive("error")]);
        for width in [8, 120] {
            for ty in [
                callable.clone(),
                TypeName::optional(callable.clone()),
                TypeName::application(
                    TypeName::primitive("Container"),
                    vec![TypeArgument::Single(callable.clone())],
                ),
                nested.clone(),
            ] {
                let output = FileSpec::builder("nested.go")
                    .add_code(CodeBlock::of("var callback %T", ty).unwrap())
                    .build()
                    .unwrap()
                    .render(width)
                    .unwrap();
                let normalized = output.split_whitespace().collect::<String>();
                let expected = match returns.len() {
                    0 => "func()",
                    1 => "func()int",
                    _ => "func()(int,bool)",
                };
                assert!(normalized.contains(expected), "{output}");
                assert_eq!(
                    normalized.matches("bool").count(),
                    usize::from(returns.len() > 1),
                    "{output}"
                );
            }
        }
    }
    let tuple = TypeName::callable(
        vec![],
        vec![TypeName::tuple(vec![
            TypeName::primitive("i32"),
            TypeName::primitive("bool"),
        ])],
    );
    assert_eq!(
        render_type(&sigil_stitch::lang::rust::Rust::new(), &tuple),
        "fn() -> (i32, bool)"
    );
}

#[test]
fn later_callable_return_slots_count_as_declaration_binder_uses_before_lowering() {
    let callable = |name| {
        TypeName::callable(
            vec![],
            vec![TypeName::primitive("Int"), TypeName::parameter(name)],
        )
    };
    let function = FunSpec::builder("work")
        .add_generic_param(GenericParamSpec::single("a").unwrap())
        .returns(vec![callable("a")])
        .body(CodeBlock::of("undefined", ()).unwrap())
        .build()
        .unwrap();
    assert!(
        function
            .validate(&Haskell::new(), DeclarationContext::TopLevel)
            .is_ok()
    );
    assert!(
        function
            .emit(&Haskell::new(), DeclarationContext::TopLevel)
            .is_ok()
    );
    let sum = ClosedSumSpec::builder("Choice")
        .add_generic_param(GenericParamSpec::single("T").unwrap())
        .add_case(ClosedSumCaseSpec::positional("Callback", vec![callable("T")]).unwrap())
        .build()
        .unwrap();
    assert!(sum.validate(&sigil_stitch::lang::rust::Rust::new()).is_ok());
    // Validation sees stored binders; complete type lowering still rejects multiple Rust slots.
    assert!(
        FileSpec::builder("choice.rs")
            .add_closed_sum(sum)
            .build()
            .unwrap()
            .render(120)
            .is_err()
    );
}

#[test]
#[allow(
    deprecated,
    reason = "compare released and modern input meanings through complete native files"
)]
fn ordinary_modern_type_inputs_preserve_nested_native_output_and_imports() {
    let leaf = || TypeName::importable("Values", "Value");
    let result = || TypeName::importable("Results", "Result");
    let old_callable = TypeName::function(vec![leaf()], result());
    let modern_callable = TypeName::callable(
        vec![CallableParam::Single {
            name: None,
            type_name: leaf(),
            presence: CallableParamPresence::Required,
        }],
        vec![result()],
    );
    let old_application = TypeName::generic(
        TypeName::importable("Containers", "Container"),
        vec![old_callable.clone()],
    );
    let modern_application = TypeName::application(
        TypeName::importable("Containers", "Container"),
        vec![TypeArgument::Single(modern_callable.clone())],
    );
    for descriptor in languages_registry::BUILT_IN_LANGUAGES {
        if ![
            "cpp",
            "dart",
            "go",
            "haskell",
            "kotlin",
            "ocaml",
            "python",
            "rust",
            "scala",
            "swift",
            "typescript",
        ]
        .contains(&descriptor.id)
        {
            continue;
        }
        for (legacy, modern) in [
            (old_application.clone(), modern_application.clone()),
            (
                TypeName::optional(old_callable.clone()),
                TypeName::optional(modern_callable.clone()),
            ),
        ] {
            for width in [8, 120] {
                let render = |type_name| {
                    FileSpec::builder(&format!("input.{}", descriptor.extension))
                        .add_code(CodeBlock::of("type%W%T", (type_name,)).unwrap())
                        .build()
                        .unwrap()
                        .render(width)
                        .unwrap()
                };
                assert_eq!(
                    render(legacy.clone()),
                    render(modern.clone()),
                    "{}",
                    descriptor.id
                );
            }
        }
    }
}

fn render_type(lang: &dyn CodeLang, ty: &TypeName) -> String {
    let block = lang.lower_type_name(ty).unwrap();
    CodeRenderer::new(lang, &ImportGroup::new(), 80)
        .render(&block)
        .unwrap()
}

#[test]
#[allow(
    deprecated,
    reason = "empty applications must be rejected for both input representations"
)]
fn empty_applications_follow_target_grammar_in_complete_rendering() {
    for extension in ["ts", "scala", "cpp"] {
        for ty in [
            TypeName::application(TypeName::primitive("Bundle"), vec![]),
            TypeName::generic(TypeName::primitive("Bundle"), vec![]),
        ] {
            for width in [8, 120] {
                let result = FileSpec::builder(&format!("Empty.{extension}"))
                    .add_code(CodeBlock::of("%T%W", (ty.clone(),)).unwrap())
                    .build()
                    .unwrap()
                    .render(width);
                if extension == "cpp" {
                    assert_eq!(result.unwrap().trim(), "Bundle<>");
                } else {
                    let error = result.unwrap_err();
                    assert!(
                        error
                            .to_string()
                            .contains("type argument lists must not be empty"),
                        "{error}"
                    );
                }
            }
        }
    }
}

#[test]
fn native_type_lowerers_reject_unrepresentable_callable_slots_and_expansions() {
    for id in [
        "cpp", "csharp", "dart", "go", "haskell", "java", "kotlin", "ocaml", "python", "rust",
        "scala", "swift",
    ] {
        let lang = languages_registry::adapter_for(id);
        for parameter in [
            CallableParam::Single {
                name: Some("value".into()),
                type_name: TypeName::parameter("T"),
                presence: CallableParamPresence::Required,
            },
            CallableParam::Single {
                name: None,
                type_name: TypeName::parameter("T"),
                presence: CallableParamPresence::Optional,
            },
            CallableParam::Repeated {
                name: None,
                element_type: TypeName::parameter("T"),
            },
            CallableParam::Expansion {
                name: None,
                pattern: TypeName::parameter("Ts"),
            },
        ] {
            let ty = TypeName::callable(vec![parameter], vec![TypeName::parameter("R")]);
            let error = lang.lower_type_name(&ty).unwrap_err();
            assert!(
                matches!(error, SigilStitchError::UnsupportedTypeName { .. }),
                "{id}: {error}"
            );
        }
        if id != "cpp" {
            let ty = TypeName::application(
                TypeName::parameter("F"),
                vec![TypeArgument::Expansion {
                    pattern: TypeName::parameter("Ts"),
                }],
            );
            let error = lang.lower_type_name(&ty).unwrap_err();
            assert!(error.to_string().contains("expansion"), "{id}: {error}");
        }
    }
    for id in ["go", "kotlin", "ocaml", "python", "swift"] {
        let lang = languages_registry::adapter_for(id);
        let error = lang
            .lower_type_name(&TypeName::application(TypeName::parameter("F"), vec![]))
            .unwrap_err();
        assert!(error.to_string().contains("empty"), "{id}: {error}");
    }
    let ocaml = languages_registry::adapter_for("ocaml");
    assert!(
        ocaml
            .lower_type_name(&TypeName::callable(
                vec![],
                vec![TypeName::primitive("int")]
            ))
            .unwrap_err()
            .to_string()
            .contains("nullary")
    );
}

#[test]
fn generic_binding_builder_validates_bounds_and_nested_kind_intent() {
    let domain = GenericParamDomain::Pack {
        element_kind: Some(KindExpr::Constructor {
            parameters: vec![KindExpr::Type],
            result: Box::new(KindExpr::Named(TypeName::parameter("K"))),
        }),
    };
    let binding = GenericParamSpec::builder("Ts", domain.clone())
        .with_bound(TypeName::primitive("Clone"))
        .with_context_bound(TypeName::primitive("Show"))
        .build()
        .unwrap();
    assert_eq!(binding.name(), "Ts");
    assert_eq!(binding.domain(), &domain);
    assert_eq!(binding.bounds(), &[TypeName::primitive("Clone")]);
    assert_eq!(binding.context_bounds(), &[TypeName::primitive("Show")]);
    assert!(GenericParamSpec::single(" ").is_err());
    assert!(GenericParamSpec::single("T\n").is_err());
    assert!(
        GenericParamSpec::single("T")
            .unwrap()
            .with_context_bound(TypeName::parameter(""))
            .is_err()
    );
    assert!(
        GenericParamSpec::builder("T", GenericParamDomain::Single { kind: None })
            .with_bound(TypeName::parameter(""))
            .build()
            .is_err()
    );
}

#[test]
fn scala_rejects_nullary_constructor_kinds_for_supported_declaration_owners() {
    let nullary = KindExpr::Constructor {
        parameters: vec![],
        result: Box::new(KindExpr::Type),
    };
    for kind in [
        nullary.clone(),
        KindExpr::Constructor {
            parameters: vec![nullary],
            result: Box::new(KindExpr::Type),
        },
    ] {
        let binding =
            GenericParamSpec::new("F", GenericParamDomain::Single { kind: Some(kind) }).unwrap();
        let function = FunSpec::builder("wrap")
            .add_generic_param(binding.clone())
            .returns(vec![TypeName::primitive("Int")])
            .body(CodeBlock::of("???", ()).unwrap())
            .build()
            .unwrap();
        let ty = TypeSpec::builder("Box", TypeKind::Class)
            .add_generic_param(binding.clone())
            .build()
            .unwrap();
        for file in [
            FileSpec::builder("Wrap.scala")
                .add_function(function)
                .build()
                .unwrap(),
            FileSpec::builder("Box.scala").add_type(ty).build().unwrap(),
        ] {
            let error = file.render(120).unwrap_err();
            assert!(error.to_string().contains("nonempty parameters"), "{error}");
        }
    }
}

#[test]
fn rust_modern_lifetime_constraints_are_checked_for_every_declaration_owner() {
    for (subject, bound, valid) in [
        (
            TypeName::parameter("'missing"),
            TypeName::parameter("'static"),
            false,
        ),
        (
            TypeName::parameter("'a"),
            TypeName::primitive("Clone"),
            false,
        ),
        (
            TypeName::application(
                TypeName::parameter("'a"),
                vec![TypeArgument::Single(TypeName::primitive("i32"))],
            ),
            TypeName::parameter("'static"),
            false,
        ),
        (
            TypeName::application(
                TypeName::primitive("'a"),
                vec![TypeArgument::Single(TypeName::primitive("i32"))],
            ),
            TypeName::parameter("'static"),
            false,
        ),
        (
            TypeName::parameter("'a"),
            TypeName::parameter("'static"),
            true,
        ),
    ] {
        let binding = GenericParamSpec::lifetime("'a").unwrap();
        let payload = TypeName::reference_with_lifetime(TypeName::primitive("i32"), "'a");
        let function = FunSpec::builder("borrow")
            .add_generic_param(binding.clone())
            .add_where_constraint(subject.clone(), vec![bound.clone()])
            .add_param(ParameterSpec::new("value", payload.clone()).unwrap())
            .returns(vec![TypeName::primitive("()")])
            .body(CodeBlock::of("()", ()).unwrap())
            .build()
            .unwrap();
        let ty = TypeSpec::builder("Borrowed", TypeKind::Struct)
            .add_generic_param(binding.clone())
            .add_where_constraint(subject.clone(), vec![bound.clone()])
            .add_field(
                FieldSpec::builder("value", payload.clone())
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();
        let sum = ClosedSumSpec::builder("Choice")
            .add_generic_param(binding)
            .add_where_constraint(subject, vec![bound])
            .add_case(ClosedSumCaseSpec::positional("Value", vec![payload]).unwrap())
            .build()
            .unwrap();
        for file in [
            FileSpec::builder("borrow.rs")
                .add_function(function)
                .build()
                .unwrap(),
            FileSpec::builder("borrowed.rs")
                .add_type(ty)
                .build()
                .unwrap(),
            FileSpec::builder("choice.rs")
                .add_closed_sum(sum)
                .build()
                .unwrap(),
        ] {
            let result = file.render(120);
            if valid {
                let output = result.unwrap();
                assert!(output.contains("'a: 'static"), "{output}");
            } else {
                let error = result.unwrap_err();
                assert!(
                    error.to_string().contains("lifetime")
                        || error.to_string().contains("ConstraintSubject")
                        || error.to_string().contains("constraint subject"),
                    "{error}"
                );
            }
        }
    }
}

#[test]
fn same_version_nested_parametric_values_roundtrip_and_revalidate() {
    let callable = TypeName::callable(
        vec![
            CallableParam::Single {
                name: Some("text".into()),
                type_name: TypeName::primitive("string"),
                presence: CallableParamPresence::Required,
            },
            CallableParam::Single {
                name: Some("count".into()),
                type_name: TypeName::primitive("number"),
                presence: CallableParamPresence::Optional,
            },
            CallableParam::Repeated {
                name: Some("items".into()),
                element_type: TypeName::importable_type("./items", "Item"),
            },
        ],
        vec![TypeName::application(
            TypeName::primitive("Promise"),
            vec![TypeArgument::Single(TypeName::primitive("void"))],
        )],
    );
    let decoded: TypeName =
        serde_json::from_value(serde_json::to_value(&callable).unwrap()).unwrap();
    assert_eq!(decoded, callable);
    assert_eq!(
        render_type(&TypeScript::new(), &decoded),
        render_type(&TypeScript::new(), &callable)
    );
    let function = FunSpec::builder("handler")
        .returns(vec![callable])
        .body(CodeBlock::of("throw new Error('fixture');", ()).unwrap())
        .build()
        .unwrap();
    let decoded: FunSpec =
        serde_json::from_value(serde_json::to_value(&function).unwrap()).unwrap();
    let render_function = |function| {
        FileSpec::builder("handler.ts")
            .add_function(function)
            .build()
            .unwrap()
            .render(120)
            .unwrap()
    };
    assert_eq!(render_function(decoded), render_function(function));

    let binding = || {
        GenericParamSpec::new(
            "n",
            GenericParamDomain::Single {
                kind: Some(KindExpr::Named(TypeName::importable("GHC.TypeNats", "Nat"))),
            },
        )
        .unwrap()
    };
    let type_ = TypeSpec::builder("Indexed", TypeKind::Struct)
        .add_generic_param(binding())
        .build()
        .unwrap();
    let decoded: TypeSpec = serde_json::from_value(serde_json::to_value(&type_).unwrap()).unwrap();
    let render_type_owner = |type_| {
        FileSpec::builder("Indexed.hs")
            .add_type(type_)
            .build()
            .unwrap()
            .render(120)
            .unwrap()
    };
    assert_eq!(render_type_owner(decoded), render_type_owner(type_));
    let sum = ClosedSumSpec::builder("Choice")
        .add_generic_param(binding())
        .add_case(ClosedSumCaseSpec::unit("Empty").unwrap())
        .build()
        .unwrap();
    let decoded: ClosedSumSpec =
        serde_json::from_value(serde_json::to_value(&sum).unwrap()).unwrap();
    let render_sum = |sum| {
        FileSpec::builder("Choice.hs")
            .add_closed_sum(sum)
            .build()
            .unwrap()
            .render(120)
            .unwrap()
    };
    assert_eq!(render_sum(decoded), render_sum(sum));

    let expanded = TypeName::application(
        TypeName::primitive("Bundle"),
        vec![TypeArgument::Expansion {
            pattern: TypeName::parameter("Ts"),
        }],
    );
    let decoded: TypeName =
        serde_json::from_value(serde_json::to_value(&expanded).unwrap()).unwrap();
    assert_eq!(decoded, expanded);
    assert_eq!(
        render_type(&sigil_stitch::lang::cpp::Cpp::new(), &decoded),
        "Bundle<Ts...>"
    );
}

#[test]
fn java_modern_application_bounds_preserve_erasure_rejection() {
    let bound = |element| {
        TypeName::application(
            TypeName::importable("constraints", "Container"),
            vec![TypeArgument::Single(TypeName::primitive(element))],
        )
    };
    let binding = GenericParamSpec::single("T")
        .unwrap()
        .with_bound(bound("String"))
        .unwrap();
    let type_ = TypeSpec::builder("Box", TypeKind::Class)
        .add_generic_param(binding.clone())
        .add_where_constraint(TypeName::parameter("T"), vec![bound("Integer")])
        .build()
        .unwrap();
    let function = FunSpec::builder("work")
        .add_generic_param(binding)
        .add_where_constraint(TypeName::parameter("T"), vec![bound("Integer")])
        .returns(vec![TypeName::parameter("T")])
        .body(CodeBlock::of("return null;", ()).unwrap())
        .build()
        .unwrap();
    let language = sigil_stitch::lang::java::Java::new();
    assert!(
        type_
            .emit(&language)
            .unwrap_err()
            .to_string()
            .contains("same erased type")
    );
    assert!(
        function
            .emit(&language, DeclarationContext::Member)
            .unwrap_err()
            .to_string()
            .contains("same erased type")
    );
}

#[test]
#[allow(
    deprecated,
    reason = "verify constraint identity across released and modern type inputs"
)]
fn csharp_application_bounds_merge_across_representations_without_losing_qualification() {
    let base = || TypeName::importable("Constraints", "IContainer");
    let value = || TypeName::importable_type("Values", "Value");
    let legacy = TypeName::generic(base(), vec![value()]);
    let modern = TypeName::application(
        base(),
        vec![TypeArgument::Single(TypeName::qualified("Values", "Value"))],
    );
    for (first, second) in [
        (legacy.clone(), modern.clone()),
        (modern.clone(), legacy),
        (modern.clone(), modern),
    ] {
        let function = FunSpec::builder("Work")
            .add_generic_param(
                GenericParamSpec::single("T")
                    .unwrap()
                    .with_bound(first)
                    .unwrap(),
            )
            .add_where_constraint(TypeName::parameter("T"), vec![second])
            .returns(vec![TypeName::primitive("void")])
            .body(CodeBlock::of("return;", ()).unwrap())
            .build()
            .unwrap();
        let output = FileSpec::builder("Work.cs")
            .add_type(
                TypeSpec::builder("Owner", TypeKind::Class)
                    .add_method(function)
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap()
            .render(120)
            .unwrap();
        assert_eq!(
            output.matches("IContainer<Values.Value>").count(),
            1,
            "{output}"
        );
    }
}

#[test]
fn cpp_function_templates_preserve_multiple_independent_packs() {
    let tuple = |name| {
        TypeName::application(
            TypeName::primitive("std::tuple"),
            vec![TypeArgument::Expansion {
                pattern: TypeName::parameter(name),
            }],
        )
    };
    let function = FunSpec::builder("combine")
        .add_generic_param(GenericParamSpec::pack("As").unwrap())
        .add_generic_param(GenericParamSpec::pack("Bs").unwrap())
        .add_param(ParameterSpec::new("left", tuple("As")).unwrap())
        .add_param(ParameterSpec::new("right", tuple("Bs")).unwrap())
        .returns(vec![TypeName::primitive("void")])
        .build()
        .unwrap();
    let output = FileSpec::builder("combine.cpp")
        .add_function(function)
        .build()
        .unwrap()
        .render(120)
        .unwrap();
    assert!(
        output.contains("template<class... As, class... Bs>"),
        "{output}"
    );
    assert!(
        output.contains("void combine(std::tuple<As...> left, std::tuple<Bs...> right);"),
        "{output}"
    );
}

#[test]
fn haskell_indexed_function_preserves_kinds_and_operator_imports() {
    let binding = |name| {
        GenericParamSpec::new(
            name,
            GenericParamDomain::Single {
                kind: Some(KindExpr::Named(TypeName::importable("GHC.TypeNats", "Nat"))),
            },
        )
        .unwrap()
    };
    let array = |index| {
        TypeName::application(
            TypeName::primitive("Array"),
            vec![
                TypeArgument::Single(index),
                TypeArgument::Single(TypeName::parameter("a")),
            ],
        )
    };
    let sum = TypeName::application(
        TypeName::importable_type("GHC.TypeNats", "+"),
        vec![
            TypeArgument::Single(TypeName::parameter("n")),
            TypeArgument::Single(TypeName::parameter("m")),
        ],
    );
    let function = FunSpec::builder("append")
        .add_generic_param(binding("n"))
        .add_generic_param(binding("m"))
        .add_generic_param(
            GenericParamSpec::new(
                "a",
                GenericParamDomain::Single {
                    kind: Some(KindExpr::Type),
                },
            )
            .unwrap(),
        )
        .add_param(ParameterSpec::new("left", array(TypeName::parameter("n"))).unwrap())
        .add_param(ParameterSpec::new("right", array(TypeName::parameter("m"))).unwrap())
        .returns(vec![array(sum)])
        .body(CodeBlock::of("undefined", ()).unwrap())
        .build()
        .unwrap();
    let output = FileSpec::builder("Indexed.hs")
        .add_function(function)
        .build()
        .unwrap()
        .render(120)
        .unwrap();
    assert!(output.contains("import Data.Kind (Type)"), "{output}");
    assert!(
        output.contains("import GHC.TypeNats (Nat, type (+))"),
        "{output}"
    );
    assert!(output.contains("append :: forall (n :: Nat) (m :: Nat) (a :: Type). Array n a -> Array m a -> Array ((+) n m) a"), "{output}");
}

#[test]
fn scala_modern_constructor_kinds_do_not_use_legacy_suffix_storage() {
    let constructor = KindExpr::Constructor {
        parameters: vec![KindExpr::Type],
        result: Box::new(KindExpr::Type),
    };
    let function = FunSpec::builder("wrap")
        .add_generic_param(
            GenericParamSpec::new(
                "F",
                GenericParamDomain::Single {
                    kind: Some(constructor),
                },
            )
            .unwrap(),
        )
        .add_generic_param(GenericParamSpec::single("A").unwrap())
        .add_param(
            ParameterSpec::new(
                "value",
                TypeName::application(
                    TypeName::parameter("F"),
                    vec![TypeArgument::Single(TypeName::parameter("A"))],
                ),
            )
            .unwrap(),
        )
        .returns(vec![TypeName::parameter("A")])
        .body(CodeBlock::of("???", ()).unwrap())
        .build()
        .unwrap();
    let output = FileSpec::builder("Wrap.scala")
        .add_function(function)
        .build()
        .unwrap()
        .render(120)
        .unwrap();
    assert!(
        output.contains("def wrap[F[_], A](value: F[A]): A"),
        "{output}"
    );
}

#[test]
fn modern_domains_are_borrowed_and_not_projected_into_compatibility_metadata() {
    let function = FunSpec::builder("pack")
        .add_generic_param(GenericParamSpec::pack("Ts").unwrap())
        .build()
        .unwrap();
    let parameter = function.generic_params().next().unwrap();
    assert!(matches!(
        parameter.domain(),
        std::borrow::Cow::Borrowed(GenericParamDomain::Pack { element_kind: None })
    ));
    #[expect(
        deprecated,
        reason = "verify modern bindings have no legacy-kind metadata"
    )]
    {
        assert!(parameter.legacy_kind().is_none());
    }
}

#[test]
fn named_kind_annotations_fail_closed_on_non_kind_declaration_consumers() {
    let binding = || {
        GenericParamSpec::new(
            "T",
            GenericParamDomain::Single {
                kind: Some(KindExpr::Named(TypeName::primitive("Nat"))),
            },
        )
        .unwrap()
    };
    let function = FunSpec::builder("identity")
        .add_generic_param(binding())
        .add_param(ParameterSpec::new("value", TypeName::parameter("T")).unwrap())
        .returns(vec![TypeName::parameter("T")])
        .body(CodeBlock::of("return value;", ()).unwrap())
        .build()
        .unwrap();
    assert!(
        FileSpec::builder("identity.ts")
            .add_function(function)
            .build()
            .unwrap()
            .render(80)
            .is_err()
    );
    let type_ = TypeSpec::builder("Box", TypeKind::Class)
        .add_generic_param(binding())
        .build()
        .unwrap();
    assert!(type_.validate(&TypeScript::new()).is_err());
    let sum = ClosedSumSpec::builder("Choice")
        .add_generic_param(binding())
        .add_case(ClosedSumCaseSpec::positional("Value", vec![TypeName::parameter("T")]).unwrap())
        .build()
        .unwrap();
    assert!(
        sum.validate(&sigil_stitch::lang::rust::Rust::new())
            .is_err()
    );
}

#[test]
fn haskell_type_and_closed_sum_binders_preserve_imported_kinds() {
    let binding = || {
        GenericParamSpec::new(
            "n",
            GenericParamDomain::Single {
                kind: Some(KindExpr::Named(TypeName::importable("GHC.TypeNats", "Nat"))),
            },
        )
        .unwrap()
    };
    let type_ = TypeSpec::builder("Indexed", TypeKind::Struct)
        .add_generic_param(binding())
        .build()
        .unwrap();
    let sum = ClosedSumSpec::builder("Choice")
        .add_generic_param(binding())
        .add_case(ClosedSumCaseSpec::unit("Empty").unwrap())
        .build()
        .unwrap();
    let output = FileSpec::builder("Kinds.hs")
        .add_type(type_)
        .add_closed_sum(sum)
        .build()
        .unwrap()
        .render(80)
        .unwrap();
    assert!(output.contains("import GHC.TypeNats (Nat)"), "{output}");
    assert!(output.contains("data Indexed (n :: Nat)"), "{output}");
    assert!(output.contains("data Choice (n :: Nat)"), "{output}");
}

#[test]
#[allow(
    deprecated,
    reason = "exercise released generic and callable compatibility inputs"
)]
fn all_binding_owners_reject_duplicate_names_across_input_origins() {
    let legacy = || TypeParamSpec::new("T");
    let modern = || GenericParamSpec::single("T").unwrap();
    let rust = sigil_stitch::lang::rust::Rust::new();
    let function = FunSpec::builder("identity")
        .add_type_param(legacy())
        .add_generic_param(modern())
        .build()
        .unwrap();
    assert!(matches!(
        function.validate(&rust, DeclarationContext::TopLevel),
        Err(SigilStitchError::DuplicateFunctionTypeParameterName { .. })
    ));
    let type_ = TypeSpec::builder("Box", TypeKind::Struct)
        .add_type_param(legacy())
        .add_generic_param(modern())
        .build()
        .unwrap();
    assert!(type_.validate(&rust).is_err());
    let sum = ClosedSumSpec::builder("Choice")
        .add_type_param(legacy())
        .add_generic_param(modern())
        .build()
        .unwrap();
    assert!(sum.validate(&rust).is_err());
}

#[test]
fn deserialized_modern_kinds_are_revalidated_by_type_and_closed_sum_owners() {
    let binding = || GenericParamSpec::single("T").unwrap();
    let type_ = TypeSpec::builder("Box", TypeKind::Struct)
        .add_generic_param(binding())
        .build()
        .unwrap();
    let sum = ClosedSumSpec::builder("Choice")
        .add_generic_param(binding())
        .build()
        .unwrap();
    let malformed = |mut encoded: serde_json::Value| {
        encoded["generic_entries"][0]["Modern"]["domain"] =
            serde_json::json!({"Single": {"kind": {"Named": {"Raw": "Nat"}}}});
        encoded
    };
    let type_: TypeSpec =
        serde_json::from_value(malformed(serde_json::to_value(type_).unwrap())).unwrap();
    let sum: ClosedSumSpec =
        serde_json::from_value(malformed(serde_json::to_value(sum).unwrap())).unwrap();
    let rust = sigil_stitch::lang::rust::Rust::new();
    for error in [
        type_.validate(&rust).unwrap_err(),
        sum.validate(&rust).unwrap_err(),
    ] {
        assert!(
            error
                .to_string()
                .contains("named kind requires a named leaf"),
            "{error}"
        );
    }
}

#[test]
fn external_function_adapter_needs_complete_lowering_for_modern_bindings() {
    #[derive(Debug)]
    struct CompatibilityAdapter;
    impl sigil_stitch::lang::RendererLang for CompatibilityAdapter {
        fn file_extension(&self) -> &str {
            "compatibility"
        }
        fn line_comment_prefix(&self) -> &str {
            "//"
        }
    }
    impl CodeLang for CompatibilityAdapter {}
    let function = FunSpec::builder("pack")
        .add_generic_param(GenericParamSpec::pack("Ts").unwrap())
        .build()
        .unwrap();
    let error = function
        .emit(&CompatibilityAdapter, DeclarationContext::TopLevel)
        .unwrap_err();
    assert!(
        matches!(error, SigilStitchError::MissingFunctionLowerer { .. }),
        "{error}"
    );
}

#[test]
fn typescript_lowers_named_optional_and_repeated_callable_slots() {
    let callable = TypeName::callable(
        vec![
            CallableParam::Single {
                name: Some("value".into()),
                type_name: TypeName::parameter("T"),
                presence: CallableParamPresence::Required,
            },
            CallableParam::Single {
                name: Some("fallback".into()),
                type_name: TypeName::primitive("string"),
                presence: CallableParamPresence::Optional,
            },
            CallableParam::Repeated {
                name: Some("rest".into()),
                element_type: TypeName::primitive("number"),
            },
        ],
        vec![TypeName::parameter("R")],
    );
    assert_eq!(
        render_type(&TypeScript::new(), &callable),
        "(value: T, fallback?: string, ...rest: number[]) => R"
    );
}

#[test]
fn typescript_rejects_application_expansions() {
    let application = TypeName::application(
        TypeName::primitive("Tuple"),
        vec![
            TypeArgument::Single(TypeName::primitive("Head")),
            TypeArgument::Expansion {
                pattern: TypeName::parameter("Tail"),
            },
        ],
    );
    assert!(TypeScript::new().lower_type_name(&application).is_err());
}

#[test]
fn haskell_lowers_basic_parameter_application_and_callable() {
    let applied = TypeName::application(
        TypeName::primitive("Array"),
        vec![TypeArgument::Single(TypeName::parameter("a"))],
    );
    assert_eq!(render_type(&Haskell::new(), &applied), "Array a");

    let callable = TypeName::callable(
        vec![CallableParam::Single {
            name: None,
            type_name: TypeName::parameter("a"),
            presence: CallableParamPresence::Required,
        }],
        vec![TypeName::parameter("b")],
    );
    assert_eq!(render_type(&Haskell::new(), &callable), "a -> b");
}

#[test]
#[allow(
    deprecated,
    reason = "exercise released generic and callable compatibility inputs"
)]
fn declaration_views_preserve_mixed_order_through_same_version_roundtrip() {
    let function = FunSpec::builder("identity")
        .add_type_param(TypeParamSpec::new("Legacy"))
        .add_generic_param(GenericParamSpec::single("Modern").unwrap())
        .build()
        .unwrap();
    let names: Vec<_> = function
        .generic_params()
        .map(|parameter| parameter.name().to_string())
        .collect();
    assert_eq!(names, ["Legacy", "Modern"]);

    let encoded = serde_json::to_value(&function).unwrap();
    assert!(encoded.get("type_params").is_none());
    let decoded: FunSpec = serde_json::from_value(encoded).unwrap();
    assert_eq!(
        decoded
            .generic_params()
            .map(|parameter| parameter.name().to_string())
            .collect::<Vec<_>>(),
        names
    );
}

#[test]
fn cpp_application_retains_complete_pack_patterns_and_empty_arguments() {
    use sigil_stitch::lang::cpp::Cpp;
    let pair = TypeName::application(
        TypeName::primitive("Pair"),
        vec![
            TypeArgument::Single(TypeName::parameter("As")),
            TypeArgument::Single(TypeName::parameter("Bs")),
        ],
    );
    let tuple = TypeName::application(
        TypeName::primitive("Tuple"),
        vec![TypeArgument::Expansion { pattern: pair }],
    );
    assert_eq!(render_type(&Cpp::new(), &tuple), "Tuple<Pair<As, Bs>...>");
    assert_eq!(
        render_type(
            &Cpp::new(),
            &TypeName::application(TypeName::primitive("Bundle"), vec![])
        ),
        "Bundle<>"
    );
}

#[test]
fn haskell_rejects_optional_presence_and_groups_nested_applications() {
    let optional = TypeName::callable(
        vec![CallableParam::Single {
            name: None,
            type_name: TypeName::parameter("a"),
            presence: CallableParamPresence::Optional,
        }],
        vec![TypeName::parameter("b")],
    );
    assert!(Haskell::new().lower_type_name(&optional).is_err());
    let nested = TypeName::application(
        TypeName::primitive("Array"),
        vec![
            TypeArgument::Single(TypeName::application(
                TypeName::primitive("F"),
                vec![TypeArgument::Single(TypeName::parameter("n"))],
            )),
            TypeArgument::Single(TypeName::parameter("a")),
        ],
    );
    assert_eq!(render_type(&Haskell::new(), &nested), "Array (F n) a");
}

#[test]
fn typescript_callable_generated_labels_are_unique_and_repetition_preserves_precedence() {
    let ty = TypeName::callable(
        vec![
            CallableParam::Single {
                name: None,
                type_name: TypeName::primitive("number"),
                presence: CallableParamPresence::Required,
            },
            CallableParam::Single {
                name: Some("arg0".into()),
                type_name: TypeName::primitive("number"),
                presence: CallableParamPresence::Required,
            },
            CallableParam::Repeated {
                name: None,
                element_type: TypeName::union(vec![
                    TypeName::primitive("number"),
                    TypeName::primitive("string"),
                ]),
            },
        ],
        vec![TypeName::primitive("void")],
    );
    assert_eq!(
        render_type(&TypeScript::new(), &ty),
        "(arg0_: number, arg0: number, ...arg2: (number | string)[]) => void"
    );
    assert_eq!(
        render_type(
            &TypeScript::new(),
            &TypeName::array(TypeName::callable(
                vec![],
                vec![TypeName::primitive("void")]
            ))
        ),
        "(() => void)[]"
    );
}

#[test]
fn named_kind_rejects_raw_and_compound_source_at_construction() {
    for value in [
        TypeName::raw("Nat"),
        TypeName::array(TypeName::primitive("Nat")),
    ] {
        assert!(
            GenericParamSpec::builder(
                "n",
                GenericParamDomain::Single {
                    kind: Some(KindExpr::Named(value))
                }
            )
            .build()
            .is_err()
        );
    }
}

#[test]
fn typescript_rejects_invalid_labels_and_slot_order_without_changing_supplied_names() {
    fn slot(name: &str, presence: CallableParamPresence) -> CallableParam {
        CallableParam::Single {
            name: Some(name.into()),
            type_name: TypeName::primitive("number"),
            presence,
        }
    }
    let required = CallableParamPresence::Required;
    let optional = CallableParamPresence::Optional;
    for parameters in [
        vec![slot(" ", required)],
        vec![slot("value", required), slot("value", required)],
        vec![slot("first", optional), slot("second", required)],
        vec![
            CallableParam::Repeated {
                name: None,
                element_type: TypeName::primitive("number"),
            },
            slot("last", required),
        ],
    ] {
        let callable = TypeName::callable(parameters, vec![TypeName::primitive("void")]);
        assert!(TypeScript::new().lower_type_name(&callable).is_err());
    }
}

#[test]
fn typescript_callable_labels_are_preserved_or_rejected_before_rendering() {
    let slot = |label: &str| CallableParam::Single {
        name: Some(label.into()),
        type_name: TypeName::primitive("number"),
        presence: CallableParamPresence::Required,
    };
    for parameters in [
        vec![slot("class")],
        vec![slot("class"), slot("class_")],
        vec![CallableParam::Repeated {
            name: Some("class".into()),
            element_type: TypeName::primitive("number"),
        }],
        vec![CallableParam::Expansion {
            name: Some("class".into()),
            pattern: TypeName::array(TypeName::primitive("number")),
        }],
    ] {
        let ty = TypeName::callable(parameters, vec![TypeName::primitive("void")]);
        for width in [8, 120] {
            let error = FileSpec::builder("labels.ts")
                .add_code(CodeBlock::of("type Handler = %T;", (ty.clone(),)).unwrap())
                .build()
                .unwrap()
                .render(width)
                .unwrap_err();
            assert!(error.to_string().contains("callable labels"), "{error}");
        }
    }
    let ty = TypeName::callable(
        vec![slot("class_"), slot("value")],
        vec![TypeName::primitive("void")],
    );
    assert_eq!(
        render_type(&TypeScript::new(), &ty),
        "(class_: number, value: number) => void"
    );
}

#[test]
#[allow(
    deprecated,
    reason = "invalid applied bases must fail for both input representations"
)]
fn typescript_rejects_non_named_bases_but_preserves_nested_arguments() {
    let leaf = TypeName::primitive("string");
    let inner = TypeName::application(
        TypeName::parameter("Foo"),
        vec![TypeArgument::Single(leaf.clone())],
    );
    let legacy_inner = TypeName::generic(TypeName::parameter("Foo"), vec![leaf]);
    for base in [
        inner.clone(),
        legacy_inner,
        TypeName::AssociatedType {
            base: Box::new(TypeName::parameter("Container")),
            qualifier: None,
            member: "value".into(),
        },
    ] {
        for ty in [
            TypeName::application(
                base.clone(),
                vec![TypeArgument::Single(TypeName::primitive("number"))],
            ),
            TypeName::generic(base, vec![TypeName::primitive("number")]),
        ] {
            for width in [8, 120] {
                let error = FileSpec::builder("application.ts")
                    .add_code(CodeBlock::of("type Bad = %T;", (ty.clone(),)).unwrap())
                    .build()
                    .unwrap()
                    .render(width)
                    .unwrap_err();
                assert!(error.to_string().contains("base"), "{error}");
            }
        }
    }
    let nested = TypeName::application(
        TypeName::parameter("Foo"),
        vec![TypeArgument::Single(inner)],
    );
    assert_eq!(render_type(&TypeScript::new(), &nested), "Foo<Foo<string>>");
}

#[test]
fn deserialized_named_kind_is_revalidated_before_declaration_emission() {
    let function = FunSpec::builder("identity")
        .add_generic_param(GenericParamSpec::single("T").unwrap())
        .returns(vec![TypeName::primitive("void")])
        .build()
        .unwrap();
    let mut encoded = serde_json::to_value(&function).unwrap();
    encoded["generic_entries"][0]["Modern"]["domain"] = serde_json::json!({
        "Single": { "kind": { "Named": { "Raw": "Nat" } } }
    });
    let decoded: FunSpec = serde_json::from_value(encoded).unwrap();
    let error = decoded
        .emit(&TypeScript::new(), DeclarationContext::TopLevel)
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("named kind requires a named leaf")
    );
}

#[test]
fn haskell_symbolic_application_preserves_type_imports_and_conflict_qualification() {
    let operator = |module| {
        TypeName::application(
            TypeName::importable_type(module, "+"),
            vec![
                TypeArgument::Single(TypeName::parameter("n")),
                TypeArgument::Single(TypeName::parameter("m")),
            ],
        )
    };
    let block = CodeBlock::of(
        "left :: %T\nright :: %T",
        (operator("GHC.TypeNats"), operator("GHC.TypeLits")),
    )
    .unwrap();
    let output = FileSpec::builder("Indexed.hs")
        .add_code(block)
        .build()
        .unwrap()
        .render(80)
        .unwrap();
    assert!(
        output.contains("import GHC.TypeNats (type (+))"),
        "{output}"
    );
    assert!(
        output.contains("import qualified GHC.TypeLits (type (+))"),
        "{output}"
    );
    assert!(output.contains("left :: (+) n m"), "{output}");
    assert!(output.contains("right :: (GHC.TypeLits.+) n m"), "{output}");
}

#[test]
fn haskell_mixed_operator_requests_keep_both_import_forms() {
    let block = CodeBlock::of(
        "%T\n%T",
        (
            TypeName::application(
                TypeName::importable_type("Operators", "+"),
                vec![TypeArgument::Single(TypeName::parameter("n"))],
            ),
            TypeName::application(
                TypeName::importable("Operators", "+"),
                vec![TypeArgument::Single(TypeName::parameter("m"))],
            ),
        ),
    )
    .unwrap();
    let output = FileSpec::builder("Both.hs")
        .add_code(block)
        .build()
        .unwrap()
        .render(80)
        .unwrap();
    assert!(
        output.contains("import Operators ((+), type (+))"),
        "{output}"
    );
}

#[test]
fn caller_resolver_takes_precedence_over_haskell_default() {
    use sigil_stitch::import::{
        ImportAliasAssignment, ImportAliasConflictResolver, ImportAliasConflicts,
        ImportAliasRejection,
    };
    struct Reject;
    impl ImportAliasConflictResolver for Reject {
        fn resolve(
            &self,
            _: &ImportAliasConflicts<'_>,
        ) -> Result<Vec<ImportAliasAssignment>, ImportAliasRejection> {
            Err(ImportAliasRejection::new("explicit resolver selected"))
        }
    }
    let block = CodeBlock::of(
        "%T\n%T",
        (
            TypeName::importable_type("First", "+"),
            TypeName::importable_type("Second", "+"),
        ),
    )
    .unwrap();
    let file = FileSpec::builder("Operators.hs")
        .add_code(block)
        .build()
        .unwrap();
    assert!(file.render(80).is_ok());
    let error = file
        .render_with_import_alias_resolver(80, &Reject)
        .unwrap_err();
    assert!(
        matches!(error, SigilStitchError::ImportAliasResolverRejected { reason } if reason == "explicit resolver selected")
    );
}
