//! Complete declaration metadata, intrinsic diagnostics, and target rejection.

use sigil_stitch::lang::{ClosedSumIntent, ValidatedClosedSum};
use sigil_stitch::prelude::*;
use sigil_stitch::spec::where_spec::TypeParamSpec;

fn unit(name: &str) -> ClosedSumSpec {
    ClosedSumSpec::builder(name)
        .add_case(ClosedSumCaseSpec::unit("Ready").unwrap())
        .build()
        .unwrap()
}

fn errors(sum: ClosedSumSpec) -> Vec<SigilStitchError> {
    let result = FileSpec::builder("outcome.rs")
        .add_closed_sum(sum)
        .build()
        .unwrap()
        .validate()
        .unwrap_err();
    let SigilStitchError::FileSpecValidation { errors, .. } = result else {
        panic!("{result}")
    };
    errors
}

#[test]
fn deserialized_intrinsic_errors_remain_ordered_and_complete() {
    let mut value = serde_json::to_value(unit("Outcome")).unwrap();
    value["name"] = "".into();
    for flag in [
        "is_static",
        "is_abstract",
        "is_readonly",
        "is_async",
        "is_override",
        "is_constructor",
    ] {
        value["modifiers"][flag] = true.into();
    }
    let sum: ClosedSumSpec = serde_json::from_value(value).unwrap();
    let issues = errors(sum);
    assert!(matches!(
        &issues[0],
        SigilStitchError::EmptyName {
            builder: "ClosedSumSpec"
        }
    ));
    assert!(
        matches!(&issues[1], SigilStitchError::InvalidTypeModifiers { modifiers, .. } if modifiers == &["static", "abstract", "readonly", "async", "override", "constructor"])
    );

    let sum = ClosedSumSpec::builder("Outcome")
        .annotation(CodeBlock::of("", ()).unwrap())
        .annotate(AnnotationSpec::new(""))
        .annotate(AnnotationSpec::importable(TypeName::raw("")))
        .add_type_param(TypeParamSpec::new(""))
        .add_type_param(TypeParamSpec::new("T").with_bound(TypeName::raw("")))
        .add_type_param(TypeParamSpec::new("T"))
        .add_type_param(TypeParamSpec::new("T"))
        .add_where_constraint(TypeName::raw(""), vec![])
        .add_where_constraint(TypeName::raw("T"), vec![TypeName::raw("")])
        .add_case(ClosedSumCaseSpec::unit("Ready").unwrap())
        .add_case(ClosedSumCaseSpec::unit("Ready").unwrap())
        .add_case(ClosedSumCaseSpec::unit("Ready").unwrap())
        .build()
        .unwrap();
    let issues = errors(sum);
    assert_eq!(issues.iter().filter(|e| matches!(e, SigilStitchError::InvalidTypeDeclaration { reason, .. } if reason.contains("annotation"))).count(), 3);
    assert_eq!(
        issues
            .iter()
            .filter(|e| matches!(e, SigilStitchError::DuplicateTypeParameterName { .. }))
            .count(),
        1
    );
    assert_eq!(
        issues
            .iter()
            .filter(|e| matches!(e, SigilStitchError::DuplicateClosedSumCaseName { .. }))
            .count(),
        1
    );
    assert_eq!(
        issues
            .iter()
            .filter(|e| matches!(e, SigilStitchError::InvalidTypeParameter { .. }))
            .count(),
        5
    );
}

#[test]
fn case_builders_and_deserialized_payloads_reject_invalid_shapes() {
    assert!(matches!(
        ClosedSumSpec::builder("").build(),
        Err(SigilStitchError::EmptyName { .. })
    ));
    assert!(matches!(
        ClosedSumCaseSpec::unit(""),
        Err(SigilStitchError::EmptyName { .. })
    ));
    assert!(matches!(
        ClosedSumCaseSpec::positional("Value", vec![]),
        Err(SigilStitchError::InvalidClosedSumCasePayload {
            form: ClosedSumCaseForm::PositionalPayload,
            ..
        })
    ));
    assert!(matches!(
        ClosedSumCaseSpec::record("Value", vec![]),
        Err(SigilStitchError::InvalidClosedSumCasePayload {
            form: ClosedSumCaseForm::RecordPayload,
            ..
        })
    ));
    for reverse in [false, true] {
        let builder = ClosedSumCaseSpec::builder("Value");
        let field = FieldSpec::of("value", TypeName::raw("Value"));
        let builder = if reverse {
            builder
                .record_field(field)
                .positional_payload(TypeName::raw("Value"))
        } else {
            builder
                .positional_payload(TypeName::raw("Value"))
                .record_field(field)
        };
        assert!(
            matches!(builder.build(), Err(SigilStitchError::InvalidClosedSumCasePayload { reason, .. }) if reason.contains("cannot combine"))
        );
    }
    for annotation in [
        AnnotationSpec::new(""),
        AnnotationSpec::importable(TypeName::raw("")),
    ] {
        assert!(
            matches!(ClosedSumCaseSpec::builder("Value").annotate(annotation).build(), Err(SigilStitchError::InvalidClosedSumCasePayload { reason, .. }) if reason.contains("empty name"))
        );
    }
    assert!(
        matches!(ClosedSumCaseSpec::builder("Value").annotation(CodeBlock::of("", ()).unwrap()).build(), Err(SigilStitchError::InvalidClosedSumCasePayload { reason, .. }) if reason.contains("opaque annotation"))
    );
    let mut value = serde_json::to_value(unit("Outcome")).unwrap();
    value["cases"][0]["name"] = "".into();
    value["cases"][0]["data"] = serde_json::json!({"Record": []});
    let issues = errors(serde_json::from_value(value).unwrap());
    assert!(matches!(
        issues[0],
        SigilStitchError::EmptyName {
            builder: "ClosedSumCaseSpec"
        }
    ));
    assert!(matches!(
        issues[1],
        SigilStitchError::InvalidClosedSumCasePayload {
            form: ClosedSumCaseForm::RecordPayload,
            ..
        }
    ));
    let mut field = serde_json::to_value(FieldSpec::of("value", TypeName::raw("Value"))).unwrap();
    field["field_type"] = serde_json::json!({"Raw": ""});
    let mut value = serde_json::to_value(unit("Outcome")).unwrap();
    value["cases"][0]["data"] = serde_json::json!({"Record": [field]});
    assert!(errors(serde_json::from_value(value).unwrap()).iter().any(|e| matches!(e, SigilStitchError::InvalidClosedSumCasePayload { reason, .. } if reason.contains("empty type"))));
}

#[test]
fn root_names_visibility_and_case_metadata_are_target_local() {
    for extension in ["rs", "swift", "hs", "ml", "scala", "java", "kt", "dart"] {
        let lang = sigil_stitch::lang::lang_from_extension(extension).unwrap();
        assert!(
            matches!(unit("bad-name").validate(lang.as_ref()), Err(SigilStitchError::InvalidTypeDeclaration { reason, .. }) if reason.contains("identifier")),
            "{extension}"
        );
        let name = if extension == "ml" {
            "outcome"
        } else {
            "Outcome"
        };
        let sum = ClosedSumSpec::builder(name)
            .visibility(Visibility::Protected)
            .add_case(ClosedSumCaseSpec::unit("Ready").unwrap())
            .build()
            .unwrap();
        assert!(
            matches!(sum.validate(lang.as_ref()), Err(SigilStitchError::InvalidTypeDeclaration { reason, .. }) if reason.contains("visibility")),
            "{extension}"
        );
        for opaque in [false, true] {
            let case = ClosedSumCaseSpec::builder("Ready").doc("Case documentation");
            let case = if opaque {
                case.annotation(CodeBlock::of("OpaqueMarker", ()).unwrap())
            } else {
                case.annotate(AnnotationSpec::new("StructuredMarker"))
            };
            let sum = ClosedSumSpec::builder(name)
                .doc("Root documentation")
                .add_case(case.build().unwrap())
                .build()
                .unwrap();
            if matches!(extension, "hs" | "ml") {
                assert!(
                    matches!(sum.validate(lang.as_ref()), Err(SigilStitchError::InvalidClosedSumCase { reason, .. }) if reason.contains("annotations"))
                );
            } else {
                let source = FileSpec::builder(&format!("outcome.{extension}"))
                    .add_closed_sum(sum)
                    .build()
                    .unwrap()
                    .render(80)
                    .unwrap();
                assert!(
                    source.contains("Root documentation") && source.contains("Case documentation"),
                    "{source}"
                );
                assert!(
                    source.contains(if opaque {
                        "OpaqueMarker"
                    } else {
                        "StructuredMarker"
                    }),
                    "{source}"
                );
            }
        }
    }
}

#[derive(Debug)]
struct InspectIntent;
impl RendererLang for InspectIntent {
    fn file_extension(&self) -> &str {
        "inspect"
    }
    fn line_comment_prefix(&self) -> &str {
        "//"
    }
}
impl CodeLang for InspectIntent {
    fn capabilities(&self) -> LanguageCapabilities<'_> {
        let declaration = &[
            TypeDeclarationCapability::ParametricPolymorphism,
            TypeDeclarationCapability::BoundedPolymorphism,
            TypeDeclarationCapability::Attributes,
        ];
        let profile = ClosedSumCapabilityProfile::new(
            declaration,
            &[ClosedSumCaseForm::RecordPayload],
            false,
        )
        .with_record_fields(FieldCapabilityProfile::new(
            FieldContext::ClosedSumRecordPayload,
            &[FieldCapability::ExplicitType],
        ));
        LanguageCapabilities::permissive().with_closed_sum(profile)
    }
    fn validate_closed_sum(&self, intent: ClosedSumIntent<'_>) -> Result<(), SigilStitchError> {
        assert_eq!(intent.name(), "Outcome");
        assert!(!intent.is_empty());
        assert_eq!(intent.modifiers().visibility, Visibility::Public);
        assert_eq!(intent.doc(), &["Root"]);
        assert_eq!(intent.type_params()[0].name(), "T");
        assert_eq!(intent.where_constraints().len(), 1);
        assert_eq!(intent.annotations().len(), 1);
        assert_eq!(intent.annotation_specs().len(), 1);
        Ok(())
    }
    fn lower_closed_sum(
        &self,
        sum: ValidatedClosedSum<'_>,
    ) -> Result<Vec<CodeBlock>, SigilStitchError> {
        assert_eq!(sum.name(), "Outcome");
        assert_eq!(sum.modifiers().visibility, Visibility::Public);
        assert_eq!(sum.visibility(), Visibility::Public);
        assert_eq!(sum.doc(), &["Root"]);
        assert_eq!(sum.type_params().len(), 1);
        assert_eq!(sum.where_constraints().len(), 1);
        assert_eq!(sum.annotations().len(), 1);
        assert_eq!(sum.annotation_specs().len(), 1);
        let case = sum.cases().next().unwrap();
        assert_eq!(case.form(), ClosedSumCaseForm::RecordPayload);
        assert_eq!(case.doc(), &["Case"]);
        assert_eq!(case.annotations().len(), 1);
        assert_eq!(case.annotation_specs().len(), 1);
        assert_eq!(case.record_payload().unwrap().fields()[0].name(), "value");
        Ok(vec![CodeBlock::of("%L", case.name())?])
    }
}

#[test]
fn validated_views_preserve_complete_semantic_metadata() {
    let case = ClosedSumCaseSpec::builder("Value")
        .doc("Case")
        .annotation(CodeBlock::of("OpaqueCase", ()).unwrap())
        .annotate(AnnotationSpec::new("CaseMarker"))
        .record_field(FieldSpec::of("value", TypeName::raw("T")))
        .build()
        .unwrap();
    let sum = ClosedSumSpec::builder("Outcome")
        .visibility(Visibility::Public)
        .doc("Root")
        .add_type_param(TypeParamSpec::new("T"))
        .add_where_constraint(TypeName::raw("T"), vec![TypeName::raw("Bound")])
        .annotation(CodeBlock::of("OpaqueRoot", ()).unwrap())
        .annotate(AnnotationSpec::new("RootMarker"))
        .add_case(case)
        .build()
        .unwrap();
    assert_eq!(sum.intent().cases()[0].name(), "Value");
    assert_eq!(
        sum.emit(&InspectIntent).unwrap()[0]
            .render_standalone(&InspectIntent, 80)
            .unwrap(),
        "Value"
    );
    let capabilities = InspectIntent.capabilities();
    assert!(capabilities.supports_closed_sum_form(ClosedSumCaseForm::RecordPayload));
    assert!(!capabilities.supports_closed_sum_form(ClosedSumCaseForm::Unit));
    assert!(capabilities.supports_closed_sum_capability(TypeDeclarationCapability::Attributes));
    assert!(
        !capabilities
            .supports_closed_sum_capability(TypeDeclarationCapability::HigherKindedPolymorphism)
    );
    let profile = capabilities.closed_sum_profile().unwrap();
    assert_eq!(profile.case_forms(), &[ClosedSumCaseForm::RecordPayload]);
    assert_eq!(profile.declaration_capabilities().len(), 3);
    assert!(!profile.supports_empty_sum());
    assert!(!LanguageCapabilities::permissive().supports_closed_sum_form(ClosedSumCaseForm::Unit));
    assert!(
        !LanguageCapabilities::permissive()
            .supports_closed_sum_capability(TypeDeclarationCapability::Attributes)
    );
}

#[test]
fn rust_lifetime_constraints_reject_invalid_subjects_and_preserve_valid_bounds() {
    use sigil_stitch::lang::rust::Rust;
    for parameter in [
        TypeParamSpec::lifetime("'9bad"),
        TypeParamSpec::new("bad-name"),
        TypeParamSpec::new("T").with_context_bound(TypeName::raw("Show")),
    ] {
        let sum = ClosedSumSpec::builder("Outcome")
            .add_type_param(parameter)
            .add_case(ClosedSumCaseSpec::unit("Ready").unwrap())
            .build()
            .unwrap();
        assert!(matches!(
            sum.validate(&Rust::new()),
            Err(SigilStitchError::InvalidTypeParameter { .. })
        ));
    }
    for (subject, bound) in [
        (
            TypeName::generic(TypeName::raw("'a"), vec![TypeName::raw("T")]),
            TypeName::raw("'static"),
        ),
        (TypeName::raw("'b"), TypeName::raw("'static")),
        (TypeName::raw("'a"), TypeName::raw("Clone")),
    ] {
        let sum = ClosedSumSpec::builder("Outcome")
            .add_type_param(TypeParamSpec::lifetime("'a"))
            .add_where_constraint(subject, vec![bound])
            .add_case(
                ClosedSumCaseSpec::positional(
                    "Value",
                    vec![TypeName::reference_with_lifetime(
                        TypeName::raw("str"),
                        "'a",
                    )],
                )
                .unwrap(),
            )
            .build()
            .unwrap();
        assert!(matches!(
            sum.validate(&Rust::new()),
            Err(SigilStitchError::InvalidTypeParameter { .. })
        ));
    }
    let sum = ClosedSumSpec::builder("Outcome")
        .add_type_param(
            TypeParamSpec::new("T")
                .with_bound(TypeName::raw("Clone"))
                .with_bound(TypeName::raw("Send")),
        )
        .add_type_param(TypeParamSpec::lifetime("'a").with_bound(TypeName::raw("'static")))
        .add_where_constraint(
            TypeName::raw("T"),
            vec![TypeName::raw("Sync"), TypeName::raw("Sized")],
        )
        .add_where_constraint(TypeName::raw("'a"), vec![TypeName::raw("'static")])
        .add_case(
            ClosedSumCaseSpec::positional(
                "Value",
                vec![
                    TypeName::reference_with_lifetime(TypeName::raw("T"), "'a"),
                    TypeName::raw("T"),
                ],
            )
            .unwrap(),
        )
        .build()
        .unwrap();
    let source = FileSpec::builder("outcome.rs")
        .add_closed_sum(sum)
        .build()
        .unwrap()
        .render(100)
        .unwrap();
    assert!(
        source.contains(
            "enum Outcome<'a: 'static, T: Clone + Send> where T: Sync + Sized, 'a: 'static"
        ),
        "{source}"
    );
}

#[test]
fn algebraic_parameters_contexts_and_compound_payloads_keep_local_grammar() {
    use sigil_stitch::lang::{haskell::Haskell, ocaml::OCaml};
    for (extension, parameter) in [("hs", "A"), ("ml", "A")] {
        let lang = sigil_stitch::lang::lang_from_extension(extension).unwrap();
        let name = if extension == "ml" {
            "outcome"
        } else {
            "Outcome"
        };
        let sum = ClosedSumSpec::builder(name)
            .add_type_param(TypeParamSpec::new(parameter))
            .add_case(ClosedSumCaseSpec::unit("Ready").unwrap())
            .build()
            .unwrap();
        assert!(matches!(
            sum.validate(lang.as_ref()),
            Err(SigilStitchError::InvalidTypeParameter { .. })
        ));
    }
    let invalid_constraint = ClosedSumSpec::builder("Outcome")
        .add_type_param(TypeParamSpec::new("a"))
        .add_where_constraint(TypeName::raw("b"), vec![TypeName::raw("Show")])
        .add_case(ClosedSumCaseSpec::unit("Ready").unwrap())
        .build()
        .unwrap();
    assert!(matches!(
        invalid_constraint.validate(&Haskell::new()),
        Err(SigilStitchError::InvalidTypeParameter { .. })
    ));
    let haskell = ClosedSumSpec::builder("Outcome")
        .add_type_param(
            TypeParamSpec::new("a")
                .with_bound(TypeName::raw("Show"))
                .with_context_bound(TypeName::raw("Eq")),
        )
        .add_where_constraint(TypeName::primitive("a"), vec![TypeName::raw("Ord")])
        .add_case(
            ClosedSumCaseSpec::positional(
                "Value",
                vec![TypeName::generic(
                    TypeName::raw("Maybe"),
                    vec![TypeName::raw("a")],
                )],
            )
            .unwrap(),
        )
        .build()
        .unwrap();
    let source = FileSpec::builder("Outcome.hs")
        .add_closed_sum(haskell)
        .build()
        .unwrap()
        .render(100)
        .unwrap();
    assert!(
        source.contains("data (Show a, Eq a, Ord a) => Outcome a"),
        "{source}"
    );
    assert!(source.contains("Value (Maybe a)"), "{source}");
    let ocaml = ClosedSumSpec::builder("outcome")
        .doc("Root documentation")
        .add_type_param(TypeParamSpec::new("a"))
        .add_type_param(TypeParamSpec::new("'b"))
        .add_case(
            ClosedSumCaseSpec::builder("Value")
                .doc("Case documentation")
                .positional_payload(TypeName::tuple(vec![
                    TypeName::raw("'a"),
                    TypeName::raw("'b"),
                ]))
                .positional_payload(TypeName::raw("int"))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    ocaml.validate(&OCaml::new()).unwrap();
    let source = FileSpec::builder("outcome.ml")
        .add_closed_sum(ocaml)
        .build()
        .unwrap()
        .render(100)
        .unwrap();
    assert!(source.contains("type ('a, 'b) outcome"), "{source}");
    assert!(
        source.contains("Case documentation") && source.contains("Root documentation"),
        "{source}"
    );
    assert!(source.contains("Value of ('a * 'b) * int"), "{source}");
}

#[test]
fn root_annotations_and_multiple_positional_values_are_not_dropped() {
    for extension in ["rs", "swift", "scala", "java", "kt"] {
        let sum = ClosedSumSpec::builder("Outcome")
            .doc("Root")
            .annotation(CodeBlock::of("OpaqueRoot", ()).unwrap())
            .annotate(AnnotationSpec::new("RootMarker"))
            .add_case(
                ClosedSumCaseSpec::positional(
                    "Value",
                    vec![TypeName::raw("First"), TypeName::raw("Second")],
                )
                .unwrap(),
            )
            .build()
            .unwrap();
        let source = FileSpec::builder(&format!("outcome.{extension}"))
            .add_closed_sum(sum)
            .build()
            .unwrap()
            .render(100)
            .unwrap();
        for fragment in ["OpaqueRoot", "RootMarker", "First", "Second"] {
            assert!(source.contains(fragment), "{extension}: {source}");
        }
    }
}
