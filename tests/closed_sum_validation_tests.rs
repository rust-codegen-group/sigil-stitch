//! Intrinsic, capability, and complete-lowerer failure contracts.

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::error::SigilStitchError;
use sigil_stitch::lang::capability::{
    ClosedSumCapabilityProfile, ClosedSumCaseForm, LanguageCapabilities,
};
use sigil_stitch::lang::{CodeLang, RendererLang, ValidatedClosedSum};
use sigil_stitch::spec::closed_sum_case_spec::ClosedSumCaseSpec;
use sigil_stitch::spec::closed_sum_spec::ClosedSumSpec;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::type_name::TypeName;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Clone, Copy)]
enum Lowering {
    Valid,
    Failure,
    EmptyVector,
    EmptyBlock,
}

#[derive(Debug, Clone)]
struct Adapter {
    profile: Option<ClosedSumCapabilityProfile<'static>>,
    lowering: Lowering,
    calls: Arc<AtomicUsize>,
}

const UNIT_PROFILE: ClosedSumCapabilityProfile<'static> =
    ClosedSumCapabilityProfile::new(&[], &[ClosedSumCaseForm::Unit], false);

impl Adapter {
    fn new(profile: Option<ClosedSumCapabilityProfile<'static>>, lowering: Lowering) -> Self {
        Self {
            profile,
            lowering,
            calls: Arc::new(AtomicUsize::new(0)),
        }
    }
}

impl RendererLang for Adapter {
    fn file_extension(&self) -> &str {
        "sum"
    }

    fn line_comment_prefix(&self) -> &str {
        "//"
    }
}

impl CodeLang for Adapter {
    fn capabilities(&self) -> LanguageCapabilities<'_> {
        match self.profile {
            Some(profile) => LanguageCapabilities::permissive().with_closed_sum(profile),
            None => LanguageCapabilities::permissive(),
        }
    }

    fn lower_closed_sum(
        &self,
        _: ValidatedClosedSum<'_>,
    ) -> Result<Vec<CodeBlock>, SigilStitchError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        match self.lowering {
            Lowering::Valid => Ok(vec![CodeBlock::of("sum", ())?]),
            Lowering::Failure => Err(SigilStitchError::InvalidTypeDeclaration {
                type_name: "adapter failure".to_string(),
                reason: "lowering failed".to_string(),
            }),
            Lowering::EmptyVector => Ok(vec![]),
            Lowering::EmptyBlock => Ok(vec![CodeBlock::of("sum", ())?, CodeBlock::of("", ())?]),
        }
    }
}

fn unit_sum() -> ClosedSumSpec {
    ClosedSumSpec::builder("Outcome")
        .add_case(ClosedSumCaseSpec::unit("Ready").unwrap())
        .build()
        .unwrap()
}

fn file_errors(sum: ClosedSumSpec, adapter: Adapter, extension: bool) -> Vec<SigilStitchError> {
    let file = FileSpec::builder_with("outcome.sum", adapter);
    let file = if extension {
        file.add_spec(sum)
    } else {
        file.add_closed_sum(sum)
    };
    let error = file.build().unwrap().validate().unwrap_err();
    let SigilStitchError::FileSpecValidation { errors, .. } = error else {
        panic!("expected complete file diagnostics: {error}");
    };
    errors
}

#[test]
fn malformed_deserialized_cases_validate_before_absent_profile() {
    for data in [
        serde_json::json!({ "Positional": [] }),
        serde_json::json!({ "Record": [] }),
        serde_json::json!({ "Positional": [{ "Primitive": "" }] }),
    ] {
        let mut json = serde_json::to_value(unit_sum()).unwrap();
        json["cases"][0]["data"] = data;
        let sum: ClosedSumSpec = serde_json::from_value(json).unwrap();
        let adapter = Adapter::new(None, Lowering::Valid);
        let errors = file_errors(sum.clone(), adapter.clone(), false);
        assert!(matches!(
            errors.as_slice(),
            [
                SigilStitchError::InvalidClosedSumCasePayload { .. },
                SigilStitchError::UnsupportedClosedSum { .. }
            ]
        ));
        assert_eq!(
            sum.validate(&adapter).unwrap_err().to_string(),
            errors[0].to_string()
        );
        assert_eq!(adapter.calls.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn duplicate_cases_follow_all_intrinsic_errors_and_are_reported_once() {
    let sum = ClosedSumSpec::builder("Outcome")
        .add_case(ClosedSumCaseSpec::unit("A").unwrap())
        .add_case(ClosedSumCaseSpec::unit("A").unwrap())
        .add_case(ClosedSumCaseSpec::unit("A").unwrap())
        .add_case(ClosedSumCaseSpec::unit("B").unwrap())
        .add_case(ClosedSumCaseSpec::unit("B").unwrap())
        .build()
        .unwrap();
    let mut json = serde_json::to_value(sum).unwrap();
    json["cases"][4]["data"] = serde_json::json!({ "Positional": [] });
    let sum: ClosedSumSpec = serde_json::from_value(json).unwrap();
    let errors = file_errors(sum, Adapter::new(None, Lowering::Valid), false);
    assert!(matches!(
        errors.as_slice(),
        [
            SigilStitchError::InvalidClosedSumCasePayload { case_name, .. },
            SigilStitchError::DuplicateClosedSumCaseName { case_name: a, .. },
            SigilStitchError::DuplicateClosedSumCaseName { case_name: b, .. },
            SigilStitchError::UnsupportedClosedSum { .. }
        ] if case_name == "B" && a == "A" && b == "B"
    ));
}

#[test]
fn duplicate_record_fields_survive_deserialization_without_ordinary_variant_errors() {
    let mut json = serde_json::to_value(unit_sum()).unwrap();
    let field = serde_json::to_value(FieldSpec::of("value", TypeName::raw("Payload"))).unwrap();
    json["cases"][0]["data"] = serde_json::json!({ "Record": [field, field, field] });
    let sum: ClosedSumSpec = serde_json::from_value(json).unwrap();
    let errors = file_errors(sum, Adapter::new(None, Lowering::Valid), false);
    assert!(matches!(
        errors.as_slice(),
        [
            SigilStitchError::DuplicateClosedSumRecordFieldName { case_name, field_name },
            SigilStitchError::UnsupportedClosedSum { .. }
        ] if case_name == "Ready" && field_name == "value"
    ));
}

#[test]
fn unsupported_record_form_retains_intrinsic_errors_but_skips_target_field_validation() {
    let mut json = serde_json::to_value(unit_sum()).unwrap();
    let field = serde_json::to_value(FieldSpec::of("value", TypeName::raw(""))).unwrap();
    json["cases"][0]["data"] = serde_json::json!({ "Record": [field] });
    let sum: ClosedSumSpec = serde_json::from_value(json).unwrap();
    let adapter = Adapter::new(Some(UNIT_PROFILE), Lowering::Valid);
    let errors = file_errors(sum, adapter.clone(), false);
    assert!(matches!(
        errors.as_slice(),
        [
            SigilStitchError::InvalidClosedSumCasePayload { .. },
            SigilStitchError::UnsupportedClosedSumCaseForm {
                form: ClosedSumCaseForm::RecordPayload,
                ..
            }
        ]
    ));
    assert_eq!(adapter.calls.load(Ordering::SeqCst), 0);
}

#[test]
fn concrete_and_extension_routes_preserve_the_same_ordered_diagnostics() {
    let mut json = serde_json::to_value(unit_sum()).unwrap();
    json["name"] = serde_json::json!("");
    json["modifiers"]["is_abstract"] = serde_json::json!(true);
    json["cases"][0]["name"] = serde_json::json!("");
    let sum: ClosedSumSpec = serde_json::from_value(json).unwrap();
    let adapter = Adapter::new(None, Lowering::Valid);
    let concrete = file_errors(sum.clone(), adapter.clone(), false);
    let extension = file_errors(sum.clone(), adapter.clone(), true);
    assert_eq!(
        concrete.iter().map(ToString::to_string).collect::<Vec<_>>(),
        extension
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
    );
    assert!(matches!(
        concrete.as_slice(),
        [
            SigilStitchError::EmptyName { builder: "ClosedSumSpec" },
            SigilStitchError::InvalidTypeModifiers { modifiers, .. },
            SigilStitchError::EmptyName { builder: "ClosedSumCaseSpec" },
            SigilStitchError::UnsupportedClosedSum { .. }
        ] if modifiers == &["abstract"]
    ));
    assert_eq!(
        sum.emit(&adapter).unwrap_err().to_string(),
        concrete[0].to_string()
    );
    assert_eq!(adapter.calls.load(Ordering::SeqCst), 0);
}

#[test]
fn complete_lowerer_failures_are_preserved_and_empty_outputs_fail_closed() {
    for lowering in [
        Lowering::Failure,
        Lowering::EmptyVector,
        Lowering::EmptyBlock,
    ] {
        let adapter = Adapter::new(Some(UNIT_PROFILE), lowering);
        let error = unit_sum().emit(&adapter).unwrap_err();
        match lowering {
            Lowering::Failure => assert!(matches!(
                error,
                SigilStitchError::InvalidTypeDeclaration { type_name, reason }
                    if type_name == "adapter failure" && reason == "lowering failed"
            )),
            Lowering::EmptyVector => assert!(matches!(
                error,
                SigilStitchError::EmptyClosedSumLowering {
                    block_index: None,
                    ..
                }
            )),
            Lowering::EmptyBlock => assert!(matches!(
                error,
                SigilStitchError::EmptyClosedSumLowering {
                    block_index: Some(1),
                    ..
                }
            )),
            Lowering::Valid => unreachable!(),
        }
        assert_eq!(adapter.calls.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn complete_lowerer_is_called_once_per_concrete_or_extension_emission() {
    for extension in [false, true] {
        let adapter = Adapter::new(Some(UNIT_PROFILE), Lowering::Valid);
        let file = FileSpec::builder_with("outcome.sum", adapter.clone());
        let file = if extension {
            file.add_spec(unit_sum())
        } else {
            file.add_closed_sum(unit_sum())
        };
        assert_eq!(file.build().unwrap().render(80).unwrap().trim(), "sum");
        assert_eq!(adapter.calls.load(Ordering::SeqCst), 1);
    }
}
