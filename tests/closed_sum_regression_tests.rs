//! Regressions at the declaration-validation and complete-lowering boundary.

use sigil_stitch::lang::{dart::Dart, java::Java, ocaml::OCaml, swift::Swift};
use sigil_stitch::prelude::*;
#[allow(
    deprecated,
    reason = "exercise released generic and callable compatibility inputs"
)]
use sigil_stitch::spec::where_spec::TypeParamSpec;

#[test]
fn java_record_cases_reject_forbidden_component_names() {
    for name in [
        "clone",
        "finalize",
        "getClass",
        "hashCode",
        "notify",
        "notifyAll",
        "toString",
        "wait",
    ] {
        let sum = ClosedSumSpec::builder("Outcome")
            .add_case(
                ClosedSumCaseSpec::record(
                    "Value",
                    vec![FieldSpec::of(name, TypeName::primitive("int"))],
                )
                .unwrap(),
            )
            .build()
            .unwrap();
        assert!(
            matches!(sum.validate(&Java::new()), Err(SigilStitchError::InvalidField { field_name, context: FieldContext::ClosedSumRecordPayload, .. }) if field_name == name),
            "{name}"
        );
    }
}

#[test]
#[allow(
    deprecated,
    reason = "exercise released generic and callable compatibility inputs"
)]
fn ocaml_rejects_type_parameters_with_the_same_lowered_name() {
    let sum = ClosedSumSpec::builder("outcome")
        .add_type_param(TypeParamSpec::new("a"))
        .add_type_param(TypeParamSpec::new("'a"))
        .add_case(ClosedSumCaseSpec::unit("Ok").unwrap())
        .build()
        .unwrap();
    assert!(
        matches!(sum.validate(&OCaml::new()), Err(SigilStitchError::InvalidTypeParameter { parameter_name, .. }) if parameter_name == "'a")
    );
}

#[test]
#[allow(
    deprecated,
    reason = "exercise released generic and callable compatibility inputs"
)]
fn ordinary_declaration_capabilities_precede_type_specific_capabilities() {
    let spec = TypeSpec::builder("Outcome", TypeKind::Enum)
        .add_type_param(TypeParamSpec::new("T"))
        .add_field(FieldSpec::of("value", TypeName::raw("Int")))
        .build()
        .unwrap();
    assert!(
        matches!(spec.validate(&Swift::new()), Err(SigilStitchError::UnsupportedTypeDeclarationCapabilities { capabilities, .. }) if capabilities == vec![TypeDeclarationCapability::ParametricPolymorphism])
    );
    let error = FileSpec::builder_with("Outcome.swift", Swift::new())
        .add_type(spec)
        .build()
        .unwrap()
        .validate()
        .unwrap_err();
    let SigilStitchError::FileSpecValidation { errors, .. } = error else {
        panic!("{error}")
    };
    assert!(
        matches!(errors.as_slice(), [SigilStitchError::UnsupportedTypeDeclarationCapabilities { capabilities: declaration, .. }, SigilStitchError::UnsupportedTypeCapabilities { capabilities: type_specific, .. }] if declaration == &[TypeDeclarationCapability::ParametricPolymorphism] && type_specific == &[TypeCapability::RecordFields])
    );
}

#[derive(Debug)]
struct RecordOnly;
impl RendererLang for RecordOnly {
    fn file_extension(&self) -> &str {
        "sum"
    }
    fn line_comment_prefix(&self) -> &str {
        "//"
    }
}
impl CodeLang for RecordOnly {
    fn capabilities(&self) -> LanguageCapabilities<'_> {
        LanguageCapabilities::permissive().with_closed_sum(ClosedSumCapabilityProfile::new(
            &[],
            &[ClosedSumCaseForm::RecordPayload],
            false,
        ))
    }
}

#[test]
fn case_form_errors_precede_record_target_errors() {
    let sum = ClosedSumSpec::builder("Outcome")
        .add_case(
            ClosedSumCaseSpec::record(
                "Record",
                vec![FieldSpec::of("value", TypeName::raw("Payload"))],
            )
            .unwrap(),
        )
        .add_case(ClosedSumCaseSpec::unit("Unit").unwrap())
        .build()
        .unwrap();
    assert!(matches!(
        sum.validate(&RecordOnly),
        Err(SigilStitchError::UnsupportedClosedSumCaseForm {
            form: ClosedSumCaseForm::Unit,
            ..
        })
    ));
    let error = FileSpec::builder_with("outcome.sum", RecordOnly)
        .add_closed_sum(sum)
        .build()
        .unwrap()
        .validate()
        .unwrap_err();
    let SigilStitchError::FileSpecValidation { errors, .. } = error else {
        panic!("{error}")
    };
    assert!(matches!(
        errors.as_slice(),
        [
            SigilStitchError::UnsupportedClosedSumCaseForm { .. },
            SigilStitchError::UnsupportedFieldContext { .. }
        ]
    ));
}

#[test]
fn dart_case_annotations_are_lowered_without_root_attributes() {
    let sum = ClosedSumSpec::builder("Outcome")
        .add_case(
            ClosedSumCaseSpec::builder("Ok")
                .annotate(AnnotationSpec::new("deprecated"))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    let source = FileSpec::builder_with("outcome.dart", Dart::new())
        .add_closed_sum(sum)
        .build()
        .unwrap()
        .render(80)
        .unwrap();
    assert!(
        source.contains("@deprecated\nfinal class OutcomeOk"),
        "{source}"
    );
}

#[derive(Debug)]
struct NoFeatures;
impl RendererLang for NoFeatures {
    fn file_extension(&self) -> &str {
        "minimal"
    }
    fn line_comment_prefix(&self) -> &str {
        "//"
    }
}
impl CodeLang for NoFeatures {
    fn capabilities(&self) -> LanguageCapabilities<'_> {
        const TYPES: &[TypeKindCapabilityProfile<'_>] =
            &[TypeKindCapabilityProfile::new(TypeKind::Class, &[], &[])];
        LanguageCapabilities::strict().with_types(TYPES)
    }
}

#[test]
#[allow(
    deprecated,
    reason = "exercise released generic and callable compatibility inputs"
)]
fn declaration_capability_diagnostics_preserve_feature_order_and_empty_groups() {
    use sigil_stitch::spec::where_spec::TypeParamKind;
    let declaration = TypeSpec::builder("Container", TypeKind::Class)
        .add_type_param(
            TypeParamSpec::new("F")
                .with_kind(TypeParamKind::Constructor1)
                .with_bound(TypeName::raw("Bound")),
        )
        .annotate(AnnotationSpec::new("Marker"));
    let spec = declaration.build().unwrap();
    let expected = vec![
        TypeDeclarationCapability::ParametricPolymorphism,
        TypeDeclarationCapability::BoundedPolymorphism,
        TypeDeclarationCapability::HigherKindedPolymorphism,
        TypeDeclarationCapability::Attributes,
    ];
    assert!(
        matches!(spec.validate(&NoFeatures), Err(SigilStitchError::UnsupportedTypeDeclarationCapabilities { capabilities, .. }) if capabilities == expected)
    );
    let error = FileSpec::builder_with("container.minimal", NoFeatures)
        .add_type(spec)
        .build()
        .unwrap()
        .validate()
        .unwrap_err();
    assert!(
        matches!(error, SigilStitchError::FileSpecValidation { errors, .. } if matches!(errors.as_slice(), [SigilStitchError::UnsupportedTypeDeclarationCapabilities { capabilities, .. }] if capabilities == &expected))
    );
    let type_only = TypeSpec::builder("Container", TypeKind::Class)
        .add_field(FieldSpec::of("value", TypeName::raw("Value")))
        .build()
        .unwrap();
    assert!(
        matches!(type_only.validate(&NoFeatures), Err(SigilStitchError::UnsupportedTypeCapabilities { capabilities, .. }) if capabilities == vec![TypeCapability::RecordFields])
    );
    let unsupported = TypeSpec::builder("Container", TypeKind::Enum)
        .add_type_param(TypeParamSpec::new("T"))
        .build()
        .unwrap();
    assert!(matches!(
        unsupported.validate(&NoFeatures),
        Err(SigilStitchError::UnsupportedTypeKind { .. })
    ));
    assert!(
        LanguageCapabilities::permissive().supports_type_declaration_capability(
            TypeKind::Class,
            TypeDeclarationCapability::Attributes
        )
    );
    assert!(
        !NoFeatures
            .capabilities()
            .supports_type_declaration_capability(
                TypeKind::Enum,
                TypeDeclarationCapability::Attributes
            )
    );
}
