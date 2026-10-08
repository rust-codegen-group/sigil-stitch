//! Language-owned closed-sum output and rejection regressions.

use sigil_stitch::error::SigilStitchError;
use sigil_stitch::lang::CodeLang;
use sigil_stitch::spec::closed_sum_case_spec::ClosedSumCaseSpec;
use sigil_stitch::spec::closed_sum_spec::ClosedSumSpec;
use sigil_stitch::spec::enum_variant_spec::EnumVariantSpec;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::modifiers::{TypeKind, Visibility};
use sigil_stitch::spec::type_spec::TypeSpec;
#[allow(
    deprecated,
    reason = "exercise released generic and callable compatibility inputs"
)]
use sigil_stitch::spec::where_spec::TypeParamSpec;
use sigil_stitch::type_name::TypeName;

#[test]
fn haskell_record_selectors_require_consistent_types_across_cases() {
    let sum = |second_type| {
        ClosedSumSpec::builder("Outcome")
            .add_case(
                ClosedSumCaseSpec::record(
                    "First",
                    vec![FieldSpec::of("value", TypeName::raw("Int"))],
                )
                .unwrap(),
            )
            .add_case(
                ClosedSumCaseSpec::record(
                    "Second",
                    vec![FieldSpec::of("value", TypeName::raw(second_type))],
                )
                .unwrap(),
            )
            .build()
            .unwrap()
    };
    assert!(matches!(
        sum("String").emit(&sigil_stitch::lang::haskell::Haskell::new()),
        Err(SigilStitchError::InvalidClosedSumCase { case_name, reason, .. })
            if case_name == "Second" && reason.contains("record selector")
    ));
    let output = render(
        sigil_stitch::lang::haskell::Haskell::new(),
        "Outcome.hs",
        sum("Int"),
        100,
    );
    assert!(output.contains("First { value :: Int }"), "{output}");
    assert!(output.contains("Second { value :: Int }"), "{output}");
}

#[test]
fn unsupported_languages_reject_closed_sums_without_widening() {
    for lang in [
        Box::new(sigil_stitch::lang::c::C::new()) as Box<dyn CodeLang>,
        Box::new(sigil_stitch::lang::cpp::Cpp::new()),
        Box::new(sigil_stitch::lang::csharp::CSharp::new()),
        Box::new(sigil_stitch::lang::go::Go::new()),
        Box::new(sigil_stitch::lang::typescript::TypeScript::new()),
        Box::new(sigil_stitch::lang::python::Python::new()),
        Box::new(sigil_stitch::lang::javascript::JavaScript::new()),
        Box::new(sigil_stitch::lang::php::Php::new()),
        Box::new(sigil_stitch::lang::ruby::Ruby::new()),
        Box::new(sigil_stitch::lang::bash::Bash::new()),
        Box::new(sigil_stitch::lang::zsh::Zsh::new()),
        Box::new(sigil_stitch::lang::lua::Lua::new()),
    ] {
        for has_case in [false, true] {
            let builder = ClosedSumSpec::builder("Outcome");
            let builder = if has_case {
                builder.add_case(ClosedSumCaseSpec::unit("Empty").unwrap())
            } else {
                builder
            };
            let error = builder.build().unwrap().emit(lang.as_ref()).unwrap_err();
            assert!(
                matches!(error, SigilStitchError::UnsupportedClosedSum { .. }),
                ".{} with has_case={has_case}: {error}",
                lang.file_extension()
            );
        }
    }
}

fn render(lang: impl CodeLang, filename: &str, sum: ClosedSumSpec, width: usize) -> String {
    FileSpec::builder_with(filename, lang)
        .add_closed_sum(sum)
        .build()
        .unwrap()
        .render(width)
        .unwrap()
}

#[test]
fn closed_sum_record_fields_do_not_widen_ordinary_java_enums() {
    let ordinary = TypeSpec::builder("Outcome", TypeKind::Enum)
        .add_variant(
            EnumVariantSpec::builder("Failure")
                .record_payload_field(FieldSpec::of("code", TypeName::raw("Code")))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    assert!(matches!(
        ordinary.emit(&sigil_stitch::lang::java::Java::new()),
        Err(SigilStitchError::UnsupportedVariantCapabilities { .. })
    ));
}

#[test]
fn closed_sum_case_identifiers_are_validated_by_each_language() {
    for (owner_name, lang) in [
        (
            "Outcome",
            Box::new(sigil_stitch::lang::dart::Dart::new()) as Box<dyn CodeLang>,
        ),
        ("Outcome", Box::new(sigil_stitch::lang::rust::Rust::new())),
        ("Outcome", Box::new(sigil_stitch::lang::java::Java::new())),
        (
            "Outcome",
            Box::new(sigil_stitch::lang::haskell::Haskell::new()),
        ),
        (
            "Outcome",
            Box::new(sigil_stitch::lang::kotlin::Kotlin::new()),
        ),
        ("outcome", Box::new(sigil_stitch::lang::ocaml::OCaml::new())),
        ("Outcome", Box::new(sigil_stitch::lang::scala::Scala::new())),
        ("Outcome", Box::new(sigil_stitch::lang::swift::Swift::new())),
    ] {
        let invalid = ClosedSumSpec::builder(owner_name)
            .add_case(ClosedSumCaseSpec::unit("bad-name").unwrap())
            .build()
            .unwrap();
        let error = invalid.validate(lang.as_ref()).unwrap_err();
        assert!(
            matches!(error, SigilStitchError::InvalidClosedSumCase { .. }),
            ".{}: {error}",
            lang.file_extension()
        );
    }
}

#[test]
fn closed_sum_record_fields_require_implicit_component_metadata() {
    for lang in [
        Box::new(sigil_stitch::lang::dart::Dart::new()) as Box<dyn CodeLang>,
        Box::new(sigil_stitch::lang::java::Java::new()),
        Box::new(sigil_stitch::lang::kotlin::Kotlin::new()),
        Box::new(sigil_stitch::lang::scala::Scala::new()),
        Box::new(sigil_stitch::lang::swift::Swift::new()),
    ] {
        let field = FieldSpec::builder("value", TypeName::raw("Payload"))
            .visibility(Visibility::Public)
            .doc("component documentation")
            .build()
            .unwrap();
        let invalid = ClosedSumSpec::builder("Outcome")
            .add_case(
                ClosedSumCaseSpec::builder("Value")
                    .record_field(field)
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();
        let error = invalid.validate(lang.as_ref()).unwrap_err();
        assert!(
            matches!(
                error,
                SigilStitchError::InvalidField {
                    context: sigil_stitch::lang::capability::FieldContext::ClosedSumRecordPayload,
                    ..
                }
            ),
            ".{}: {error}",
            lang.file_extension()
        );
    }
}

#[test]
#[allow(
    deprecated,
    reason = "exercise released generic and callable compatibility inputs"
)]
fn generic_closed_sums_are_preserved_only_for_proven_combinations() {
    let rust = ClosedSumSpec::builder("Maybe")
        .add_type_param(TypeParamSpec::new("T"))
        .add_case(ClosedSumCaseSpec::unit("None").unwrap())
        .add_case(
            ClosedSumCaseSpec::builder("Some")
                .positional_payload(TypeName::primitive("T"))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    assert_eq!(
        render(sigil_stitch::lang::rust::Rust::new(), "maybe.rs", rust, 100),
        "enum Maybe<T> {\n    None,\n    Some(T),\n}\n"
    );

    let rust_lifetime = ClosedSumSpec::builder("Borrowed")
        .add_type_param(TypeParamSpec::lifetime("'a"))
        .add_case(
            ClosedSumCaseSpec::builder("Value")
                .positional_payload(TypeName::reference_with_lifetime(
                    TypeName::primitive("str"),
                    "'a",
                ))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    assert!(
        render(
            sigil_stitch::lang::rust::Rust::new(),
            "borrowed.rs",
            rust_lifetime,
            100
        )
        .contains("enum Borrowed<'a> {\n    Value(&'a str),")
    );

    let unused = ClosedSumSpec::builder("Phantom")
        .add_type_param(TypeParamSpec::new("T"))
        .add_case(ClosedSumCaseSpec::unit("Only").unwrap())
        .build()
        .unwrap();
    assert!(matches!(
        unused.emit(&sigil_stitch::lang::rust::Rust::new()),
        Err(SigilStitchError::InvalidTypeParameter { reason, .. })
            if reason.contains("must occur")
    ));

    let haskell = ClosedSumSpec::builder("Maybe")
        .add_type_param(TypeParamSpec::new("a"))
        .add_case(ClosedSumCaseSpec::unit("None").unwrap())
        .add_case(
            ClosedSumCaseSpec::builder("Some")
                .positional_payload(TypeName::primitive("a"))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    assert_eq!(
        render(
            sigil_stitch::lang::haskell::Haskell::new(),
            "Maybe.hs",
            haskell,
            100
        ),
        "data Maybe a =\n  None\n  | Some a\n"
    );

    let ocaml = ClosedSumSpec::builder("maybe")
        .add_type_param(TypeParamSpec::new("a"))
        .add_case(ClosedSumCaseSpec::unit("None").unwrap())
        .add_case(
            ClosedSumCaseSpec::builder("Some")
                .positional_payload(TypeName::primitive("'a"))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    assert_eq!(
        render(
            sigil_stitch::lang::ocaml::OCaml::new(),
            "maybe.ml",
            ocaml,
            100
        ),
        "type 'a maybe =\n  None\n  | Some of 'a\n"
    );

    let scala = ClosedSumSpec::builder("Maybe")
        .add_type_param(TypeParamSpec::new("T"))
        .add_case(ClosedSumCaseSpec::unit("None").unwrap())
        .build()
        .unwrap();
    assert!(matches!(
        scala.emit(&sigil_stitch::lang::scala::Scala::new()),
        Err(SigilStitchError::InvalidTypeDeclaration { reason, .. })
            if reason.contains("preserve the root type arguments")
    ));
}

#[test]
#[allow(
    deprecated,
    reason = "exercise released generic and callable compatibility inputs"
)]
fn rust_closed_sum_parameter_occurrence_traverses_nested_type_names() {
    let parameter = || TypeName::primitive("T");
    let payloads = vec![
        TypeName::array(TypeName::readonly_array(TypeName::pointer(
            TypeName::slice(TypeName::optional(parameter())),
        ))),
        TypeName::generic(
            TypeName::importable("example", "Wrapper"),
            vec![parameter()],
        ),
        TypeName::union(vec![TypeName::string_literal("other"), parameter()]),
        TypeName::intersection(vec![TypeName::raw("Other"), parameter()]),
        TypeName::tuple(vec![TypeName::raw("Other"), parameter()]),
        TypeName::impl_trait(vec![TypeName::raw("Other"), parameter()]),
        TypeName::dyn_trait(vec![TypeName::raw("Other"), parameter()]),
        TypeName::map(TypeName::raw("Key"), parameter()),
        TypeName::function(vec![parameter()], TypeName::raw("Output")),
        TypeName::function(vec![TypeName::raw("Input")], parameter()),
        TypeName::associated_type(TypeName::raw("Base"), Some(parameter()), "Member"),
        TypeName::wildcard_extends(parameter()),
        TypeName::wildcard_super(parameter()),
    ];

    for payload in payloads {
        let type_ = ClosedSumSpec::builder("Contains")
            .add_type_param(TypeParamSpec::new("T"))
            .add_case(
                ClosedSumCaseSpec::builder("Value")
                    .positional_payload(payload)
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();
        type_
            .validate(&sigil_stitch::lang::rust::Rust::new())
            .unwrap();
    }
}

#[test]
fn payload_imports_survive_nested_and_sibling_lowering() {
    let imported_sum = || {
        ClosedSumSpec::builder("Outcome")
            .visibility(Visibility::Public)
            .add_case(
                ClosedSumCaseSpec::builder("Value")
                    .positional_payload(TypeName::importable("com.example.model", "Payload"))
                    .build()
                    .unwrap(),
            )
            .add_case(
                ClosedSumCaseSpec::builder("Failure")
                    .record_field(FieldSpec::of(
                        "code",
                        TypeName::importable("com.example.error", "FailureCode"),
                    ))
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap()
    };

    let java = render(
        sigil_stitch::lang::java::Java::new(),
        "Outcome.java",
        imported_sum(),
        100,
    );
    assert!(
        java.contains("import com.example.error.FailureCode;"),
        "{java}"
    );
    assert!(java.contains("import com.example.model.Payload;"), "{java}");
    assert!(java.contains("record Value(Payload value0)"), "{java}");
    assert!(java.contains("record Failure(FailureCode code)"), "{java}");

    let kotlin = render(
        sigil_stitch::lang::kotlin::Kotlin::new(),
        "Outcome.kt",
        imported_sum(),
        100,
    );
    assert!(
        kotlin.contains("import com.example.error.FailureCode"),
        "{kotlin}"
    );
    assert!(
        kotlin.contains("import com.example.model.Payload"),
        "{kotlin}"
    );
    assert!(
        kotlin.contains("data class Value(val value0: Payload)"),
        "{kotlin}"
    );
    assert!(
        kotlin.contains("data class Failure(val code: FailureCode)"),
        "{kotlin}"
    );

    let dart_sum = ClosedSumSpec::builder("Outcome")
        .add_case(
            ClosedSumCaseSpec::builder("Value")
                .positional_payload(TypeName::importable(
                    "package:example/payload.dart",
                    "Payload",
                ))
                .build()
                .unwrap(),
        )
        .add_case(
            ClosedSumCaseSpec::builder("Failure")
                .record_field(FieldSpec::of(
                    "code",
                    TypeName::importable("package:example/failure.dart", "FailureCode"),
                ))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    let dart = render(
        sigil_stitch::lang::dart::Dart::new(),
        "outcome.dart",
        dart_sum,
        100,
    );
    assert!(
        dart.contains("import 'package:example/failure.dart';"),
        "{dart}"
    );
    assert!(
        dart.contains("import 'package:example/payload.dart';"),
        "{dart}"
    );
    assert!(dart.contains("final Payload value0;"), "{dart}");
    assert!(dart.contains("final FailureCode code;"), "{dart}");

    let aliased = ClosedSumSpec::builder("Outcome")
        .add_case(
            ClosedSumCaseSpec::builder("Value")
                .positional_payload(
                    TypeName::importable("com.example.model", "Payload").with_alias("WirePayload"),
                )
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    let error = FileSpec::builder_with("Outcome.java", sigil_stitch::lang::java::Java::new())
        .add_closed_sum(aliased)
        .build()
        .unwrap()
        .render(100)
        .unwrap_err();
    assert!(matches!(
        error,
        SigilStitchError::InvalidResolvedImports { .. }
    ));

    let conflicting_sum = |first_module: &str, second_module: &str| {
        ClosedSumSpec::builder("Outcome")
            .add_case(
                ClosedSumCaseSpec::builder("First")
                    .positional_payload(TypeName::importable(first_module, "Payload"))
                    .build()
                    .unwrap(),
            )
            .add_case(
                ClosedSumCaseSpec::builder("Second")
                    .positional_payload(TypeName::importable(second_module, "Payload"))
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap()
    };

    let java_conflict =
        FileSpec::builder_with("Outcome.java", sigil_stitch::lang::java::Java::new())
            .add_closed_sum(conflicting_sum("com.example.first", "com.example.second"))
            .build()
            .unwrap()
            .render(100)
            .unwrap_err();
    assert!(matches!(
        java_conflict,
        SigilStitchError::InvalidResolvedImports { .. }
    ));

    let kotlin_conflict =
        FileSpec::builder_with("Outcome.kt", sigil_stitch::lang::kotlin::Kotlin::new())
            .add_closed_sum(conflicting_sum("com.example.first", "com.example.second"))
            .build()
            .unwrap()
            .render(100)
            .unwrap_err();
    assert!(matches!(
        kotlin_conflict,
        SigilStitchError::InvalidResolvedImports { .. }
    ));

    let dart_conflict =
        FileSpec::builder_with("outcome.dart", sigil_stitch::lang::dart::Dart::new())
            .add_closed_sum(conflicting_sum(
                "package:example/first.dart",
                "package:example/second.dart",
            ))
            .build()
            .unwrap()
            .render(100)
            .unwrap_err();
    assert!(matches!(
        dart_conflict,
        SigilStitchError::InvalidResolvedImports { .. }
    ));
}

#[test]
fn payload_imports_survive_every_algebraic_lowerer_and_render_adapter() {
    use sigil_stitch::code_block::CodeBlock;
    use sigil_stitch::spec::emittable::Emittable;

    fn check(
        lang: impl CodeLang,
        filename: &str,
        name: &str,
        modules: [&str; 2],
        expected: [&str; 4],
        pretty: bool,
    ) {
        let sum = ClosedSumSpec::builder(name)
            .add_case(
                ClosedSumCaseSpec::positional(
                    "Value",
                    vec![TypeName::importable(modules[0], "Payload")],
                )
                .unwrap(),
            )
            .add_case(
                ClosedSumCaseSpec::record(
                    "Failure",
                    vec![FieldSpec::of(
                        "code",
                        TypeName::importable(modules[1], "FailureCode"),
                    )],
                )
                .unwrap(),
            )
            .build()
            .unwrap();
        let lowered = if pretty {
            Some(sum.emit_members(&lang).unwrap())
        } else {
            None
        };
        let mut file = FileSpec::builder_with(filename, lang);
        if pretty {
            // A soft break in the containing group selects the pretty adapter
            // for the complete lowered declaration, including nested payloads.
            let mut block = CodeBlock::builder();
            for lowered in lowered.unwrap() {
                block.add_code(lowered);
            }
            block.add("%W", ());
            file = file.add_code(block.build().unwrap());
        } else {
            file = file.add_closed_sum(sum);
        }
        let source = file.build().unwrap().render(100).unwrap();
        for fragment in expected {
            assert!(
                source.contains(fragment),
                "{filename} pretty={pretty}: expected {fragment:?} in {source}"
            );
        }
    }
    for pretty in [false, true] {
        check(
            sigil_stitch::lang::rust::Rust::new(),
            "outcome.rs",
            "Outcome",
            ["model", "error"],
            [
                "use model::Payload;",
                "use error::FailureCode;",
                "Value(Payload)",
                "code: FailureCode",
            ],
            pretty,
        );
        check(
            sigil_stitch::lang::swift::Swift::new(),
            "Outcome.swift",
            "Outcome",
            ["Model", "ErrorModel"],
            [
                "import Model",
                "import ErrorModel",
                "Value(Payload)",
                "code: FailureCode",
            ],
            pretty,
        );
        check(
            sigil_stitch::lang::haskell::Haskell::new(),
            "Outcome.hs",
            "Outcome",
            ["Model", "ErrorModel"],
            [
                "import Model",
                "import ErrorModel",
                "Value Payload",
                "code :: FailureCode",
            ],
            pretty,
        );
        check(
            sigil_stitch::lang::ocaml::OCaml::new(),
            "outcome.ml",
            "outcome",
            ["Model", "ErrorModel"],
            [
                "open Model",
                "open ErrorModel",
                "Value of Payload",
                "code : FailureCode",
            ],
            pretty,
        );
        check(
            sigil_stitch::lang::scala::Scala::new(),
            "Outcome.scala",
            "Outcome",
            ["model", "error"],
            [
                "import model.Payload",
                "import error.FailureCode",
                "value0: Payload",
                "code: FailureCode",
            ],
            pretty,
        );
    }
}

#[test]
fn openapi_consumer_mapping_keeps_wire_tags_outside_the_closed_sum() {
    struct TaggedVariant {
        case_name: &'static str,
        wire_tag: &'static str,
        content_type: TypeName,
    }

    let variants = [
        TaggedVariant {
            case_name: "Json",
            wire_tag: "application/json",
            content_type: TypeName::raw("JsonBody"),
        },
        TaggedVariant {
            case_name: "Text",
            wire_tag: "text/plain",
            content_type: TypeName::raw("TextBody"),
        },
    ];
    let mut builder = ClosedSumSpec::builder("ResponseBody");
    for variant in &variants {
        builder = builder.add_case(
            ClosedSumCaseSpec::builder(variant.case_name)
                .positional_payload(variant.content_type.clone())
                .build()
                .unwrap(),
        );
    }

    let output = render(
        sigil_stitch::lang::java::Java::new(),
        "ResponseBody.java",
        builder.build().unwrap(),
        100,
    );
    assert!(output.contains("record Json(JsonBody value0) implements ResponseBody {}"));
    assert!(output.contains("record Text(TextBody value0) implements ResponseBody {}"));
    for variant in variants {
        assert!(!output.contains(variant.wire_tag));
    }
}

#[test]
fn dart_record_case_constructors_use_the_validated_emitted_field_name() {
    let type_ = ClosedSumSpec::builder("Outcome")
        .add_case(
            ClosedSumCaseSpec::builder("Value")
                .record_field(FieldSpec::of("class", TypeName::raw("Payload")))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    let output = render(
        sigil_stitch::lang::dart::Dart::new(),
        "outcome.dart",
        type_,
        100,
    );
    assert!(output.contains("const OutcomeValue(this.class_) : super._();"));
    assert!(output.contains("final Payload class_;"));
    assert!(!output.contains("this.class)"));
}

#[test]
#[allow(
    deprecated,
    reason = "exercise released generic and callable compatibility inputs"
)]
fn closed_sum_payloads_exercise_wide_and_narrow_renderer_paths() {
    let type_ = |name: &str| {
        ClosedSumSpec::builder(name)
            .add_case(
                ClosedSumCaseSpec::builder("Value")
                    .positional_payload(TypeName::generic(
                        TypeName::raw("Container"),
                        vec![
                            TypeName::raw("VeryLongFirstPayload"),
                            TypeName::raw("VeryLongSecondPayload"),
                        ],
                    ))
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap()
    };
    type RenderFn = fn(ClosedSumSpec, usize) -> String;
    let languages = [
        (
            "outcome.rs",
            "Outcome",
            true,
            (|value, width| {
                render(
                    sigil_stitch::lang::rust::Rust::new(),
                    "outcome.rs",
                    value,
                    width,
                )
            }) as RenderFn,
        ),
        (
            "Outcome.swift",
            "Outcome",
            true,
            (|value, width| {
                render(
                    sigil_stitch::lang::swift::Swift::new(),
                    "Outcome.swift",
                    value,
                    width,
                )
            }) as RenderFn,
        ),
        (
            "Outcome.hs",
            "Outcome",
            false,
            (|value, width| {
                render(
                    sigil_stitch::lang::haskell::Haskell::new(),
                    "Outcome.hs",
                    value,
                    width,
                )
            }) as RenderFn,
        ),
        (
            "outcome.ml",
            "outcome",
            true,
            (|value, width| {
                render(
                    sigil_stitch::lang::ocaml::OCaml::new(),
                    "outcome.ml",
                    value,
                    width,
                )
            }) as RenderFn,
        ),
        (
            "Outcome.scala",
            "Outcome",
            true,
            (|value, width| {
                render(
                    sigil_stitch::lang::scala::Scala::new(),
                    "Outcome.scala",
                    value,
                    width,
                )
            }) as RenderFn,
        ),
        (
            "Outcome.java",
            "Outcome",
            true,
            (|value, width| {
                render(
                    sigil_stitch::lang::java::Java::new(),
                    "Outcome.java",
                    value,
                    width,
                )
            }) as RenderFn,
        ),
        (
            "Outcome.kt",
            "Outcome",
            true,
            (|value, width| {
                render(
                    sigil_stitch::lang::kotlin::Kotlin::new(),
                    "Outcome.kt",
                    value,
                    width,
                )
            }) as RenderFn,
        ),
        (
            "outcome.dart",
            "Outcome",
            true,
            (|value, width| {
                render(
                    sigil_stitch::lang::dart::Dart::new(),
                    "outcome.dart",
                    value,
                    width,
                )
            }) as RenderFn,
        ),
    ];

    for (filename, name, expects_break, render_value) in languages {
        let wide = render_value(type_(name), 120);
        let narrow = render_value(type_(name), 18);
        if expects_break {
            assert_ne!(
                wide, narrow,
                "{filename} did not exercise both renderer paths"
            );
        } else {
            assert_eq!(wide, narrow, "{filename} has no soft-break type grammar");
        }
        assert!(narrow.contains(name), "{filename}: {narrow}");
        assert!(
            narrow.contains("VeryLongFirstPayload"),
            "{filename}: {narrow}"
        );
        assert!(
            narrow.contains("VeryLongSecondPayload"),
            "{filename}: {narrow}"
        );
    }
}
