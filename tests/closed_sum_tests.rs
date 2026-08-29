use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::error::SigilStitchError;
use sigil_stitch::lang::capability::{LanguageCapabilities, TypeCapability, TypeCapabilityProfile};
use sigil_stitch::lang::{CodeLang, RendererLang, ValidatedType, VariantIntent};
use sigil_stitch::spec::annotation_spec::AnnotationSpec;
use sigil_stitch::spec::enum_variant_spec::EnumVariantSpec;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::modifiers::{TypeKind, Visibility};
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::spec::where_spec::TypeParamSpec;
use sigil_stitch::type_name::TypeName;

fn mixed_sum(name: &str) -> TypeSpec {
    TypeSpec::closed_sum(name)
        .add_variant(EnumVariantSpec::new("Empty").unwrap())
        .add_variant(
            EnumVariantSpec::builder("Value")
                .positional_payload(TypeName::raw("Payload"))
                .build()
                .unwrap(),
        )
        .add_variant(
            EnumVariantSpec::builder("Failure")
                .record_payload_field(FieldSpec::of("code", TypeName::raw("Code")))
                .record_payload_field(FieldSpec::of("message", TypeName::raw("Message")))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap()
}

fn render(lang: impl CodeLang, filename: &str, type_: TypeSpec, width: usize) -> String {
    FileSpec::builder_with(filename, lang)
        .add_type(type_)
        .build()
        .unwrap()
        .render(width)
        .unwrap()
}

const CLOSED_SUM_CAPABILITIES: &[TypeCapability] = &[TypeCapability::ClosedSum];
const CLOSED_SUM_TYPES: &[TypeCapabilityProfile<'_>] = &[TypeCapabilityProfile::new(
    TypeKind::Enum,
    CLOSED_SUM_CAPABILITIES,
)];

#[derive(Debug)]
struct SemanticClosedSumLang;

impl RendererLang for SemanticClosedSumLang {
    fn file_extension(&self) -> &str {
        "semantic-closed-sum"
    }

    fn line_comment_prefix(&self) -> &str {
        "//"
    }
}

impl CodeLang for SemanticClosedSumLang {
    fn capabilities(&self) -> LanguageCapabilities<'_> {
        LanguageCapabilities::strict().with_types(CLOSED_SUM_TYPES)
    }

    fn validate_type(
        &self,
        type_: sigil_stitch::lang::TypeIntent<'_>,
    ) -> Result<(), SigilStitchError> {
        assert!(type_.is_closed_sum());
        Ok(())
    }

    fn validate_variants(&self, variants: VariantIntent<'_>) -> Result<(), SigilStitchError> {
        assert!(variants.is_closed_sum());
        assert_eq!(variants.variants().len(), 1);
        Ok(())
    }

    fn lower_type(&self, type_: ValidatedType<'_>) -> Result<Vec<CodeBlock>, SigilStitchError> {
        assert!(type_.is_closed_sum());
        assert!(
            type_
                .variants()
                .is_some_and(|variants| variants.is_closed_sum())
        );
        Ok(vec![CodeBlock::of("semantic closed sum", ())?])
    }
}

#[derive(Debug)]
struct StrictCompatibilityLang;

impl RendererLang for StrictCompatibilityLang {
    fn file_extension(&self) -> &str {
        "strict-compatibility"
    }

    fn line_comment_prefix(&self) -> &str {
        "//"
    }
}

impl CodeLang for StrictCompatibilityLang {
    fn capabilities(&self) -> LanguageCapabilities<'_> {
        LanguageCapabilities::strict().with_types(CLOSED_SUM_TYPES)
    }
}

#[derive(Debug)]
struct PermissiveCompatibilityLang;

impl RendererLang for PermissiveCompatibilityLang {
    fn file_extension(&self) -> &str {
        "permissive-compatibility"
    }

    fn line_comment_prefix(&self) -> &str {
        "//"
    }
}

impl CodeLang for PermissiveCompatibilityLang {}

#[test]
fn construction_and_serialization_preserve_the_semantic_distinction() {
    let ordinary = TypeSpec::builder("Ordinary", TypeKind::Enum)
        .add_variant(EnumVariantSpec::new("Value").unwrap())
        .build()
        .unwrap();
    let closed = TypeSpec::closed_sum("Closed").build().unwrap();

    assert!(!ordinary.is_closed_sum());
    assert!(closed.is_closed_sum());
    assert_eq!(closed.kind(), TypeKind::Enum);

    let ordinary_json = serde_json::to_value(&ordinary).unwrap();
    assert!(ordinary_json.get("closed_sum").is_none());
    let closed_json = serde_json::to_value(&closed).unwrap();
    assert_eq!(closed_json["closed_sum"], serde_json::json!(true));
    assert!(
        !serde_json::from_value::<TypeSpec>(ordinary_json)
            .unwrap()
            .is_closed_sum()
    );
    assert!(
        serde_json::from_value::<TypeSpec>(closed_json)
            .unwrap()
            .is_closed_sum()
    );
}

#[test]
fn semantic_views_expose_closed_sum_intent_without_a_parallel_lowerer() {
    let blocks = TypeSpec::closed_sum("Outcome")
        .add_variant(EnumVariantSpec::new("Value").unwrap())
        .build()
        .unwrap()
        .emit(&SemanticClosedSumLang)
        .unwrap();
    assert_eq!(blocks.len(), 1);
}

#[test]
fn compatibility_adapters_reject_closed_sums_before_legacy_enum_lowering() {
    let closed = || {
        TypeSpec::closed_sum("Outcome")
            .add_variant(EnumVariantSpec::new("Value").unwrap())
            .build()
            .unwrap()
    };

    let permissive = closed().emit(&PermissiveCompatibilityLang).unwrap_err();
    assert!(matches!(
        permissive,
        SigilStitchError::UnsupportedTypeCapabilities { capabilities, .. }
            if capabilities == vec![TypeCapability::ClosedSum]
    ));

    let defensive = closed().emit(&StrictCompatibilityLang).unwrap_err();
    assert!(
        matches!(&defensive, SigilStitchError::MissingTypeLowerer { .. }),
        "{defensive:?}"
    );
}

#[test]
#[allow(deprecated)]
fn value_enum_representation_data_is_invalid_for_closed_sum_cases() {
    let cases = [
        EnumVariantSpec::builder("Legacy")
            .value(CodeBlock::of("1", ()).unwrap())
            .build()
            .unwrap(),
        EnumVariantSpec::builder("Discriminated")
            .discriminant(CodeBlock::of("2", ()).unwrap())
            .build()
            .unwrap(),
        EnumVariantSpec::builder("Constructed")
            .constructor_argument(CodeBlock::of("value", ()).unwrap())
            .build()
            .unwrap(),
    ];

    for case in cases {
        let error = TypeSpec::closed_sum("Outcome")
            .add_variant(case)
            .build()
            .unwrap()
            .emit(&sigil_stitch::lang::rust::Rust::new())
            .unwrap_err();
        assert!(matches!(
            error,
            SigilStitchError::InvalidTypeDeclaration { reason, .. }
                if reason.contains("closed-sum case")
        ));
    }
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
fn supported_languages_lower_unit_positional_and_record_cases() {
    let cases = [
        (
            "outcome.rs",
            "Outcome",
            "enum Outcome {\n    Empty,\n    Value(Payload),\n    Failure {\n        code: Code,\n        message: Message,\n    },\n}\n",
        ),
        (
            "Outcome.swift",
            "Outcome",
            "enum Outcome {\n    case Empty\n    case Value(Payload)\n    case Failure(code: Code, message: Message)\n}\n",
        ),
        (
            "Outcome.hs",
            "Outcome",
            "data Outcome =\n  Empty\n  | Value Payload\n  | Failure { code :: Code, message :: Message }\n",
        ),
        (
            "outcome.ml",
            "outcome",
            "type outcome =\n  Empty\n  | Value of Payload\n  | Failure of { code : Code; message : Message }\n",
        ),
        (
            "Outcome.scala",
            "Outcome",
            "enum Outcome {\n  case Empty\n  case Value(value0: Payload)\n  case Failure(code: Code, message: Message)\n}\n",
        ),
        (
            "Outcome.java",
            "Outcome",
            "sealed interface Outcome {\n    enum Empty implements Outcome { INSTANCE }\n\n    record Value(Payload value0) implements Outcome {}\n\n    record Failure(Code code, Message message) implements Outcome {}\n}\n",
        ),
        (
            "Outcome.kt",
            "Outcome",
            "internal sealed class Outcome private constructor() {\n    data object Empty : Outcome()\n\n    data class Value(val value0: Payload) : Outcome()\n\n    data class Failure(val code: Code, val message: Message) : Outcome()\n}\n",
        ),
        (
            "outcome.dart",
            "Outcome",
            "sealed class Outcome {\n  const Outcome._();\n}\n\nfinal class OutcomeEmpty extends Outcome {\n  const OutcomeEmpty._() : super._();\n  static const OutcomeEmpty instance = OutcomeEmpty._();\n}\n\nfinal class OutcomeValue extends Outcome {\n  const OutcomeValue(this.value0) : super._();\n  final Payload value0;\n}\n\nfinal class OutcomeFailure extends Outcome {\n  const OutcomeFailure(this.code, this.message) : super._();\n  final Code code;\n  final Message message;\n}\n",
        ),
    ];

    for (filename, name, expected) in cases {
        let output = FileSpec::builder(filename)
            .add_type(mixed_sum(name))
            .build()
            .unwrap()
            .render(100)
            .unwrap_or_else(|error| panic!("{filename}: {error}"));
        assert_eq!(output, expected, "{filename}");
    }
}

#[test]
fn empty_sums_are_exact_or_rejected() {
    let rust = render(
        sigil_stitch::lang::rust::Rust::new(),
        "empty.rs",
        TypeSpec::closed_sum("Empty").build().unwrap(),
        100,
    );
    assert_eq!(rust, "enum Empty {\n}\n");

    let swift = render(
        sigil_stitch::lang::swift::Swift::new(),
        "Empty.swift",
        TypeSpec::closed_sum("Empty").build().unwrap(),
        100,
    );
    assert_eq!(swift, "enum Empty {\n}\n");

    let ocaml = render(
        sigil_stitch::lang::ocaml::OCaml::new(),
        "empty.ml",
        TypeSpec::closed_sum("empty").build().unwrap(),
        100,
    );
    assert_eq!(ocaml, "type empty = |\n");

    let kotlin = render(
        sigil_stitch::lang::kotlin::Kotlin::new(),
        "Empty.kt",
        TypeSpec::closed_sum("Empty").build().unwrap(),
        100,
    );
    assert_eq!(
        kotlin,
        "internal sealed class Empty private constructor() {\n}\n"
    );

    for lang in [
        Box::new(sigil_stitch::lang::haskell::Haskell::new()) as Box<dyn CodeLang>,
        Box::new(sigil_stitch::lang::scala::Scala::new()),
        Box::new(sigil_stitch::lang::java::Java::new()),
        Box::new(sigil_stitch::lang::dart::Dart::new()),
    ] {
        assert!(
            TypeSpec::closed_sum("Empty")
                .build()
                .unwrap()
                .emit(lang.as_ref())
                .is_err()
        );
    }
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
            let builder = TypeSpec::closed_sum("Outcome");
            let builder = if has_case {
                builder.add_variant(EnumVariantSpec::new("Empty").unwrap())
            } else {
                builder
            };
            let error = builder.build().unwrap().emit(lang.as_ref()).unwrap_err();
            assert!(
                matches!(
                    error,
                    SigilStitchError::UnsupportedTypeCapabilities { ref capabilities, .. }
                        if capabilities.as_slice() == [TypeCapability::ClosedSum]
                ),
                ".{} with has_case={has_case}: {error}",
                lang.file_extension()
            );
        }
    }
}

#[test]
fn malformed_serialized_closed_sums_fail_intrinsic_validation() {
    let ordinary_struct = TypeSpec::builder("Broken", TypeKind::Struct)
        .build()
        .unwrap();
    let mut wrong_carrier = serde_json::to_value(ordinary_struct).unwrap();
    wrong_carrier["closed_sum"] = serde_json::json!(true);
    let wrong_carrier: TypeSpec = serde_json::from_value(wrong_carrier).unwrap();
    assert!(matches!(
        wrong_carrier.validate(&sigil_stitch::lang::rust::Rust::new()),
        Err(SigilStitchError::InvalidTypeDeclaration { reason, .. })
            if reason.contains("enum declaration carrier")
    ));

    let valid = TypeSpec::closed_sum("Broken")
        .add_variant(EnumVariantSpec::new("Case").unwrap())
        .build()
        .unwrap();
    let mut empty_case = serde_json::to_value(valid).unwrap();
    empty_case["variants"][0]["name"] = serde_json::json!("");
    let empty_case: TypeSpec = serde_json::from_value(empty_case).unwrap();
    assert!(matches!(
        empty_case.validate(&sigil_stitch::lang::rust::Rust::new()),
        Err(SigilStitchError::EmptyName {
            builder: "EnumVariantSpec"
        })
    ));

    let empty_annotation = TypeSpec::closed_sum("Broken")
        .add_variant(
            EnumVariantSpec::builder("Case")
                .annotation(CodeBlock::of("", ()).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    assert!(matches!(
        empty_annotation.validate(&sigil_stitch::lang::rust::Rust::new()),
        Err(SigilStitchError::InvalidTypeDeclaration { reason, .. })
            if reason.contains("opaque variant annotation")
    ));
}

#[test]
fn closed_sum_validation_aggregates_case_and_payload_failures_without_output() {
    let invalid = TypeSpec::closed_sum("Outcome")
        .add_variant(
            EnumVariantSpec::builder("class")
                .record_payload_field(FieldSpec::of("_", TypeName::raw("")))
                .record_payload_field(FieldSpec::of("bad-name", TypeName::raw("Code")))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    let file = FileSpec::builder_with("Outcome.java", sigil_stitch::lang::java::Java::new())
        .add_type(invalid)
        .build()
        .unwrap();

    let error = file.validate().unwrap_err();
    let SigilStitchError::FileSpecValidation {
        error_count,
        errors,
        ..
    } = error
    else {
        panic!("unexpected error: {error:?}");
    };
    assert_eq!(error_count, errors.len());
    assert!(error_count >= 5, "{errors:#?}");
    assert!(errors.iter().any(|error| matches!(
        error,
        SigilStitchError::EmptyVariantOperand { operand, .. }
            if operand.contains("record field")
    )));
    assert!(errors.iter().any(|error| matches!(
        error,
        SigilStitchError::MissingRequiredFieldCapabilities { .. }
    )));
    assert!(
        errors
            .iter()
            .filter(|error| matches!(error, SigilStitchError::InvalidField { .. }))
            .count()
            >= 2
    );
    assert!(errors.iter().any(|error| matches!(
        error,
        SigilStitchError::InvalidTypeDeclaration { reason, .. }
            if reason.contains("Java closed-sum case")
    )));
    assert!(file.render(80).is_err());
}

#[test]
fn closed_sum_opaque_members_and_unsupported_case_annotations_fail_closed() {
    let opaque = TypeSpec::closed_sum("Outcome")
        .add_variant(EnumVariantSpec::new("Value").unwrap())
        .extra_member(CodeBlock::of("Other,", ()).unwrap())
        .build()
        .unwrap();
    assert!(matches!(
        opaque.validate(&sigil_stitch::lang::rust::Rust::new()),
        Err(SigilStitchError::InvalidTypeDeclaration { reason, .. })
            if reason.contains("opaque members")
    ));

    let annotated = |name: &str| {
        TypeSpec::closed_sum(name)
            .add_variant(
                EnumVariantSpec::builder("Value")
                    .annotate(AnnotationSpec::new("marker"))
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap()
    };
    for (name, lang) in [
        (
            "Outcome",
            Box::new(sigil_stitch::lang::haskell::Haskell::new()) as Box<dyn CodeLang>,
        ),
        ("outcome", Box::new(sigil_stitch::lang::ocaml::OCaml::new())),
    ] {
        let error = annotated(name).emit(lang.as_ref()).unwrap_err();
        assert!(
            matches!(
                &error,
                SigilStitchError::InvalidTypeDeclaration { reason, .. }
                    if reason.contains("does not support annotations")
            ),
            "{error:?}"
        );
    }
}

#[test]
fn closed_sum_case_identifiers_are_validated_by_each_language() {
    for (owner_name, lang) in [
        (
            "Outcome",
            Box::new(sigil_stitch::lang::dart::Dart::new()) as Box<dyn CodeLang>,
        ),
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
        let invalid = TypeSpec::closed_sum(owner_name)
            .add_variant(EnumVariantSpec::new("bad-name").unwrap())
            .build()
            .unwrap();
        let error = invalid.validate(lang.as_ref()).unwrap_err();
        assert!(
            matches!(
                error,
                SigilStitchError::InvalidTypeDeclaration { ref reason, .. }
                    if reason.contains("closed-sum case")
            ),
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
        let invalid = TypeSpec::closed_sum("Outcome")
            .add_variant(
                EnumVariantSpec::builder("Value")
                    .record_payload_field(field)
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
fn generic_closed_sums_are_preserved_only_for_proven_combinations() {
    let rust = TypeSpec::closed_sum("Maybe")
        .add_type_param(TypeParamSpec::new("T"))
        .add_variant(EnumVariantSpec::new("None").unwrap())
        .add_variant(
            EnumVariantSpec::builder("Some")
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

    let rust_lifetime = TypeSpec::closed_sum("Borrowed")
        .add_type_param(TypeParamSpec::lifetime("'a"))
        .add_variant(
            EnumVariantSpec::builder("Value")
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

    let unused = TypeSpec::closed_sum("Phantom")
        .add_type_param(TypeParamSpec::new("T"))
        .add_variant(EnumVariantSpec::new("Only").unwrap())
        .build()
        .unwrap();
    assert!(matches!(
        unused.emit(&sigil_stitch::lang::rust::Rust::new()),
        Err(SigilStitchError::InvalidTypeParameter { reason, .. })
            if reason.contains("must occur")
    ));

    let haskell = TypeSpec::closed_sum("Maybe")
        .add_type_param(TypeParamSpec::new("a"))
        .add_variant(EnumVariantSpec::new("None").unwrap())
        .add_variant(
            EnumVariantSpec::builder("Some")
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

    let ocaml = TypeSpec::closed_sum("maybe")
        .add_type_param(TypeParamSpec::new("a"))
        .add_variant(EnumVariantSpec::new("None").unwrap())
        .add_variant(
            EnumVariantSpec::builder("Some")
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

    let scala = TypeSpec::closed_sum("Maybe")
        .add_type_param(TypeParamSpec::new("T"))
        .add_variant(EnumVariantSpec::new("None").unwrap())
        .build()
        .unwrap();
    assert!(matches!(
        scala.emit(&sigil_stitch::lang::scala::Scala::new()),
        Err(SigilStitchError::InvalidTypeDeclaration { reason, .. })
            if reason.contains("preserve the root type arguments")
    ));
}

#[test]
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
        let type_ = TypeSpec::closed_sum("Contains")
            .add_type_param(TypeParamSpec::new("T"))
            .add_variant(
                EnumVariantSpec::builder("Value")
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
        TypeSpec::closed_sum("Outcome")
            .visibility(Visibility::Public)
            .add_variant(
                EnumVariantSpec::builder("Value")
                    .positional_payload(TypeName::importable("com.example.model", "Payload"))
                    .build()
                    .unwrap(),
            )
            .add_variant(
                EnumVariantSpec::builder("Failure")
                    .record_payload_field(FieldSpec::of(
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

    let dart_sum = TypeSpec::closed_sum("Outcome")
        .add_variant(
            EnumVariantSpec::builder("Value")
                .positional_payload(TypeName::importable(
                    "package:example/payload.dart",
                    "Payload",
                ))
                .build()
                .unwrap(),
        )
        .add_variant(
            EnumVariantSpec::builder("Failure")
                .record_payload_field(FieldSpec::of(
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

    let aliased = TypeSpec::closed_sum("Outcome")
        .add_variant(
            EnumVariantSpec::builder("Value")
                .positional_payload(
                    TypeName::importable("com.example.model", "Payload").with_alias("WirePayload"),
                )
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    let error = FileSpec::builder_with("Outcome.java", sigil_stitch::lang::java::Java::new())
        .add_type(aliased)
        .build()
        .unwrap()
        .render(100)
        .unwrap_err();
    assert!(matches!(
        error,
        SigilStitchError::InvalidResolvedImports { .. }
    ));

    let conflicting_sum = |first_module: &str, second_module: &str| {
        TypeSpec::closed_sum("Outcome")
            .add_variant(
                EnumVariantSpec::builder("First")
                    .positional_payload(TypeName::importable(first_module, "Payload"))
                    .build()
                    .unwrap(),
            )
            .add_variant(
                EnumVariantSpec::builder("Second")
                    .positional_payload(TypeName::importable(second_module, "Payload"))
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap()
    };

    let java_conflict =
        FileSpec::builder_with("Outcome.java", sigil_stitch::lang::java::Java::new())
            .add_type(conflicting_sum("com.example.first", "com.example.second"))
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
            .add_type(conflicting_sum("com.example.first", "com.example.second"))
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
            .add_type(conflicting_sum(
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
    let mut builder = TypeSpec::closed_sum("ResponseBody");
    for variant in &variants {
        builder = builder.add_variant(
            EnumVariantSpec::builder(variant.case_name)
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
    let type_ = TypeSpec::closed_sum("Outcome")
        .add_variant(
            EnumVariantSpec::builder("Value")
                .record_payload_field(FieldSpec::of("class", TypeName::raw("Payload")))
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
fn closed_sum_payloads_exercise_wide_and_narrow_renderer_paths() {
    let type_ = |name: &str| {
        TypeSpec::closed_sum(name)
            .add_variant(
                EnumVariantSpec::builder("Value")
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
    type RenderFn = fn(TypeSpec, usize) -> String;
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
