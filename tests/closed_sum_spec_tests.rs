use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::error::SigilStitchError;
use sigil_stitch::lang::capability::{
    ClosedSumCapabilityProfile, ClosedSumCaseForm, LanguageCapabilities,
};
use sigil_stitch::lang::dart::Dart;
use sigil_stitch::lang::haskell::Haskell;
use sigil_stitch::lang::java::Java;
use sigil_stitch::lang::kotlin::Kotlin;
use sigil_stitch::lang::ocaml::OCaml;
use sigil_stitch::lang::rust::Rust;
use sigil_stitch::lang::scala::Scala;
use sigil_stitch::lang::swift::Swift;
use sigil_stitch::lang::{CodeLang, RendererLang, ValidatedClosedSum};
use sigil_stitch::spec::annotation_spec::AnnotationSpec;
use sigil_stitch::spec::closed_sum_case_spec::ClosedSumCaseSpec;
use sigil_stitch::spec::closed_sum_spec::ClosedSumSpec;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::modifiers::Visibility;
use sigil_stitch::spec::project_spec::ProjectSpec;
use sigil_stitch::spec::where_spec::TypeParamSpec;
use sigil_stitch::type_name::TypeName;

fn mixed_sum(name: &str) -> ClosedSumSpec {
    ClosedSumSpec::builder(name)
        .add_case(ClosedSumCaseSpec::unit("Empty").unwrap())
        .add_case(ClosedSumCaseSpec::positional("Value", vec![TypeName::raw("Payload")]).unwrap())
        .add_case(
            ClosedSumCaseSpec::record(
                "Failure",
                vec![
                    FieldSpec::of("code", TypeName::raw("Code")),
                    FieldSpec::of("message", TypeName::raw("Message")),
                ],
            )
            .unwrap(),
        )
        .build()
        .unwrap()
}

#[derive(Debug)]
struct CustomClosedSum;

const FORMS: &[ClosedSumCaseForm] = &[ClosedSumCaseForm::Unit];
const PROFILE: ClosedSumCapabilityProfile<'static> =
    ClosedSumCapabilityProfile::new(&[], FORMS, false);
const RECORD_FORMS: &[ClosedSumCaseForm] = &[ClosedSumCaseForm::RecordPayload];
const RECORD_PROFILE_WITHOUT_FIELDS: ClosedSumCapabilityProfile<'static> =
    ClosedSumCapabilityProfile::new(&[], RECORD_FORMS, false);

impl RendererLang for CustomClosedSum {
    fn file_extension(&self) -> &str {
        "custom-sum"
    }
    fn line_comment_prefix(&self) -> &str {
        "//"
    }
}

impl CodeLang for CustomClosedSum {
    fn capabilities(&self) -> LanguageCapabilities<'_> {
        LanguageCapabilities::permissive().with_closed_sum(PROFILE)
    }

    fn lower_closed_sum(
        &self,
        closed_sum: ValidatedClosedSum<'_>,
    ) -> Result<Vec<CodeBlock>, SigilStitchError> {
        let cases = closed_sum.cases().count();
        Ok(vec![CodeBlock::of(
            &format!("sum {} {}", closed_sum.name(), cases),
            (),
        )?])
    }
}

#[test]
fn closed_sum_spec_emits_through_its_dedicated_seam() {
    let spec = ClosedSumSpec::builder("Outcome")
        .add_case(ClosedSumCaseSpec::unit("Empty").unwrap())
        .add_case(ClosedSumCaseSpec::positional("Value", vec![TypeName::primitive("i32")]).unwrap())
        .build()
        .unwrap();

    let output = FileSpec::builder_with("outcome.rs", Rust::new())
        .add_closed_sum(spec)
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    assert!(output.contains("enum Outcome"));
    assert!(output.contains("Value(i32)"));
}

#[test]
fn closed_sum_record_payload_uses_scoped_field_capability() {
    let field = FieldSpec::builder("value", TypeName::primitive("i32"))
        .build()
        .unwrap();
    let spec = ClosedSumSpec::builder("Outcome")
        .add_case(ClosedSumCaseSpec::record("Value", vec![field]).unwrap())
        .build()
        .unwrap();

    let output = FileSpec::builder_with("outcome.rs", Rust::new())
        .add_closed_sum(spec)
        .build()
        .unwrap()
        .render(80)
        .unwrap();
    assert!(output.contains("value"));
}

#[derive(Debug)]
struct MissingRecordProfile;

impl RendererLang for MissingRecordProfile {
    fn file_extension(&self) -> &str {
        "missing-record"
    }
    fn line_comment_prefix(&self) -> &str {
        "//"
    }
}

impl CodeLang for MissingRecordProfile {
    fn capabilities(&self) -> LanguageCapabilities<'_> {
        LanguageCapabilities::permissive().with_closed_sum(RECORD_PROFILE_WITHOUT_FIELDS)
    }
}

#[test]
fn record_cases_require_the_scoped_closed_sum_field_profile() {
    let field = FieldSpec::builder("value", TypeName::primitive("i32"))
        .build()
        .unwrap();
    let spec = ClosedSumSpec::builder("Outcome")
        .add_case(ClosedSumCaseSpec::record("Value", vec![field]).unwrap())
        .build()
        .unwrap();
    assert!(matches!(
        spec.validate(&MissingRecordProfile),
        Err(SigilStitchError::UnsupportedFieldContext { .. })
    ));
}

#[test]
fn extension_and_first_class_file_routes_share_closed_sum_lowering() {
    let spec = ClosedSumSpec::builder("Outcome")
        .add_case(ClosedSumCaseSpec::unit("Empty").unwrap())
        .build()
        .unwrap();
    let direct = FileSpec::builder_with("outcome.rs", Rust::new())
        .add_closed_sum(spec.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap();
    let extension = FileSpec::builder_with("outcome.rs", Rust::new())
        .add_spec(spec)
        .build()
        .unwrap()
        .render(80)
        .unwrap();
    assert_eq!(direct, extension);
}

#[test]
fn permissive_closed_sum_opt_in_keeps_lowering_on_the_new_seam() {
    let spec = ClosedSumSpec::builder("Outcome")
        .add_case(ClosedSumCaseSpec::unit("Empty").unwrap())
        .build()
        .unwrap();
    let output = FileSpec::builder_with("outcome.custom-sum", CustomClosedSum)
        .add_closed_sum(spec)
        .build()
        .unwrap()
        .render(80)
        .unwrap();
    assert!(output.contains("sum Outcome 1"));
}

#[derive(Debug)]
struct MissingClosedSumLowerer;

impl RendererLang for MissingClosedSumLowerer {
    fn file_extension(&self) -> &str {
        "missing-sum"
    }
    fn line_comment_prefix(&self) -> &str {
        "//"
    }
}

impl CodeLang for MissingClosedSumLowerer {
    fn capabilities(&self) -> LanguageCapabilities<'_> {
        LanguageCapabilities::strict().with_closed_sum(PROFILE)
    }
}

#[test]
fn advertised_closed_sum_without_a_lowerer_fails_closed() {
    let spec = ClosedSumSpec::builder("Outcome")
        .add_case(ClosedSumCaseSpec::unit("Empty").unwrap())
        .build()
        .unwrap();
    assert!(matches!(
        spec.emit(&MissingClosedSumLowerer),
        Err(SigilStitchError::MissingClosedSumLowerer { .. })
    ));
}

#[test]
fn built_in_closed_sum_adapters_use_the_new_declaration_family() {
    for (extension, filename) in [
        ("rs", "outcome.rs"),
        ("hs", "outcome.hs"),
        ("kt", "outcome.kt"),
        ("dart", "outcome.dart"),
        ("swift", "outcome.swift"),
        ("scala", "outcome.scala"),
        ("java", "outcome.java"),
        ("ml", "outcome.ml"),
    ] {
        let type_name = if extension == "ml" {
            "outcome"
        } else {
            "Outcome"
        };
        let spec = ClosedSumSpec::builder(type_name)
            .add_case(ClosedSumCaseSpec::unit("Empty").unwrap())
            .build()
            .unwrap();
        assert!(sigil_stitch::lang::lang_from_extension(extension).is_some());
        let output = FileSpec::builder(filename)
            .add_closed_sum(spec)
            .build()
            .unwrap()
            .render(80)
            .unwrap_or_else(|error| panic!("{extension}: {error}"));
        assert!(!output.is_empty(), "{extension}");
    }
}

#[test]
fn built_in_closed_sum_adapters_preserve_each_case_shape() {
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
            .add_closed_sum(mixed_sum(name))
            .build()
            .unwrap()
            .render(100)
            .unwrap_or_else(|error| panic!("{filename}: {error}"));
        assert_eq!(output, expected, "{filename}");
    }
}

#[test]
fn empty_sum_support_matches_the_language_capability_matrix() {
    for (filename, name) in [
        ("empty.rs", "Empty"),
        ("Empty.swift", "Empty"),
        ("empty.ml", "empty"),
        ("Empty.kt", "Empty"),
    ] {
        let output = FileSpec::builder(filename)
            .add_closed_sum(ClosedSumSpec::builder(name).build().unwrap())
            .build()
            .unwrap()
            .render(100)
            .unwrap_or_else(|error| panic!("{filename}: {error}"));
        assert!(output.contains(name), "{filename}: {output}");
    }

    for (filename, name) in [
        ("Empty.hs", "Empty"),
        ("Empty.scala", "Empty"),
        ("Empty.java", "Empty"),
        ("empty.dart", "Empty"),
    ] {
        let file = FileSpec::builder(filename)
            .add_closed_sum(ClosedSumSpec::builder(name).build().unwrap())
            .build()
            .unwrap();
        assert!(
            file.render(100).is_err(),
            "{filename} unexpectedly accepts empty sum"
        );
    }
}

#[test]
fn swift_and_kotlin_reject_closed_sum_type_parameters() {
    let spec = ClosedSumSpec::builder("Outcome")
        .add_type_param(TypeParamSpec::new("T"))
        .add_case(ClosedSumCaseSpec::positional("Value", vec![TypeName::raw("T")]).unwrap())
        .build()
        .unwrap();
    for lang in [
        Box::new(Swift::new()) as Box<dyn CodeLang>,
        Box::new(Kotlin::new()),
    ] {
        assert!(matches!(
            spec.validate(lang.as_ref()),
            Err(SigilStitchError::UnsupportedTypeDeclarationCapabilities { capabilities, .. })
                if capabilities.contains(&sigil_stitch::lang::capability::TypeDeclarationCapability::ParametricPolymorphism)
        ));
    }
}

#[test]
fn haskell_closed_sum_preserves_explicit_constraints() {
    let spec = ClosedSumSpec::builder("Maybe")
        .add_type_param(TypeParamSpec::new("a"))
        .add_where_constraint(TypeName::primitive("a"), vec![TypeName::raw("Eq")])
        .add_case(ClosedSumCaseSpec::positional("Value", vec![TypeName::raw("a")]).unwrap())
        .build()
        .unwrap();
    let output = FileSpec::builder_with("Maybe.hs", Haskell::new())
        .add_closed_sum(spec)
        .build()
        .unwrap()
        .render(100)
        .unwrap();
    assert!(output.contains("data Eq a => Maybe a"), "{output}");
}

#[test]
fn haskell_and_ocaml_reject_explicit_visibility() {
    let spec = ClosedSumSpec::builder("Outcome")
        .visibility(Visibility::Public)
        .add_case(ClosedSumCaseSpec::unit("Empty").unwrap())
        .build()
        .unwrap();
    assert!(matches!(
        spec.validate(&Haskell::new()),
        Err(SigilStitchError::InvalidTypeDeclaration { reason, .. })
            if reason.contains("explicit visibility")
    ));

    let spec = ClosedSumSpec::builder("outcome")
        .visibility(Visibility::Public)
        .add_case(ClosedSumCaseSpec::unit("Empty").unwrap())
        .build()
        .unwrap();
    assert!(matches!(
        spec.validate(&OCaml::new()),
        Err(SigilStitchError::InvalidTypeDeclaration { reason, .. })
            if reason.contains("explicit visibility")
    ));
}

#[test]
fn closed_sum_visibility_follows_each_target_declaration_grammar() {
    let dart = ClosedSumSpec::builder("Outcome")
        .visibility(Visibility::Public)
        .add_case(ClosedSumCaseSpec::unit("Empty").unwrap())
        .build()
        .unwrap();
    assert!(matches!(
        dart.validate(&Dart::new()),
        Err(SigilStitchError::InvalidTypeDeclaration { reason, .. })
            if reason.contains("explicit visibility")
    ));

    let java = ClosedSumSpec::builder("Outcome")
        .visibility(Visibility::Private)
        .add_case(ClosedSumCaseSpec::unit("Empty").unwrap())
        .build()
        .unwrap();
    assert!(matches!(
        java.validate(&Java::new()),
        Err(SigilStitchError::InvalidTypeDeclaration { reason, .. })
            if reason.contains("visibility")
    ));
}

#[test]
fn case_annotations_do_not_request_root_attributes() {
    let spec = ClosedSumSpec::builder("Outcome")
        .add_case(
            ClosedSumCaseSpec::builder("Empty")
                .annotate(AnnotationSpec::new("Serializable"))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    spec.validate(&Dart::new()).unwrap();
}

#[test]
fn scala_rejects_constraints_that_its_closed_sum_lowerer_cannot_preserve() {
    let spec = ClosedSumSpec::builder("Outcome")
        .add_where_constraint(TypeName::raw("T"), vec![TypeName::raw("Bound")])
        .add_case(ClosedSumCaseSpec::unit("Empty").unwrap())
        .build()
        .unwrap();
    assert!(matches!(
        spec.validate(&Scala::new()),
        Err(SigilStitchError::InvalidTypeDeclaration { reason, .. })
            if reason.contains("declaration constraints")
    ));
}

#[test]
fn rust_closed_sum_rejects_non_lifetime_bounds_on_lifetime_parameters() {
    let spec = ClosedSumSpec::builder("Outcome")
        .add_type_param(TypeParamSpec::lifetime("'a").with_bound(TypeName::raw("Clone")))
        .add_case(ClosedSumCaseSpec::positional("Value", vec![TypeName::raw("'a")]).unwrap())
        .build()
        .unwrap();
    assert!(matches!(
        spec.validate(&Rust::new()),
        Err(SigilStitchError::InvalidTypeParameter { reason, .. })
            if reason.contains("lifetime parameters accept only")
    ));
}

#[test]
fn ocaml_closed_sum_rejects_invalid_type_parameter_names() {
    let spec = ClosedSumSpec::builder("outcome")
        .add_type_param(TypeParamSpec::new("T"))
        .add_case(ClosedSumCaseSpec::unit("Empty").unwrap())
        .build()
        .unwrap();
    assert!(matches!(
        spec.validate(&OCaml::new()),
        Err(SigilStitchError::InvalidTypeParameter { reason, .. })
            if reason.contains("lowercase type-variable")
    ));
}

#[test]
fn case_builder_rejects_switching_payload_forms() {
    let error = ClosedSumCaseSpec::builder("Mixed")
        .positional_payload(TypeName::raw("Payload"))
        .record_field(FieldSpec::of("field", TypeName::raw("Payload")))
        .build()
        .unwrap_err();
    assert!(matches!(
        error,
        SigilStitchError::InvalidClosedSumCasePayload { reason, .. }
            if reason.contains("cannot combine")
    ));
}

#[test]
fn malformed_case_structured_annotations_fail_intrinsic_validation() {
    let error = ClosedSumCaseSpec::builder("Value")
        .annotate(AnnotationSpec::new(""))
        .build()
        .unwrap_err();
    assert!(matches!(
        error,
        SigilStitchError::InvalidClosedSumCasePayload { reason, .. }
            if reason.contains("structured annotation")
    ));
}

#[test]
fn closed_sum_serde_and_import_pipeline_are_preserved() {
    let spec = ClosedSumSpec::builder("Outcome")
        .add_case(
            ClosedSumCaseSpec::positional(
                "Value",
                vec![TypeName::importable("com.example", "Payload")],
            )
            .unwrap(),
        )
        .build()
        .unwrap();
    let json = serde_json::to_value(&spec).unwrap();
    let restored: ClosedSumSpec = serde_json::from_value(json).unwrap();
    let file = FileSpec::builder_with("Outcome.java", sigil_stitch::lang::java::Java::new())
        .add_closed_sum(restored)
        .build()
        .unwrap();
    let wide = file.render(120).unwrap();
    let narrow = file.render(18).unwrap();
    assert!(wide.contains("import com.example.Payload;"), "{wide}");
    assert!(narrow.contains("Payload"), "{narrow}");
}

#[test]
fn project_validation_aggregates_closed_sum_failures_before_rendering() {
    let invalid = FileSpec::builder_with("Empty.hs", Haskell::new())
        .add_closed_sum(ClosedSumSpec::builder("Empty").build().unwrap())
        .build()
        .unwrap();
    let valid = FileSpec::builder_with("Outcome.rs", Rust::new())
        .add_closed_sum(
            ClosedSumSpec::builder("Outcome")
                .add_case(ClosedSumCaseSpec::unit("Ready").unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    let project = ProjectSpec::builder()
        .add_file(invalid)
        .add_file(valid)
        .build()
        .unwrap();
    let error = project.render(100).unwrap_err();
    assert!(matches!(
        error,
        SigilStitchError::ProjectSpecValidation {
            invalid_file_count: 1,
            ..
        }
    ));
}
