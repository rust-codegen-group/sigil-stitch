//! File storage and ordered project lowering contracts for closed sums.

use sigil_stitch::lang::{ClosedSumIntent, ValidatedClosedSum};
use sigil_stitch::prelude::*;
use std::sync::{Arc, Mutex};

fn sum() -> ClosedSumSpec {
    ClosedSumSpec::builder("Outcome")
        .add_case(ClosedSumCaseSpec::unit("Ready").unwrap())
        .build()
        .unwrap()
}

#[test]
fn first_class_files_round_trip_but_extension_specs_do_not_serialize() {
    let first_class = FileSpec::builder("outcome.rs")
        .add_closed_sum(sum())
        .build()
        .unwrap();
    let json = serde_json::to_value(&first_class).unwrap();
    assert!(json["members"][0].get("ClosedSum").is_some());
    let restored: FileSpec = serde_json::from_value(json.clone()).unwrap();
    assert_eq!(serde_json::to_value(&restored).unwrap(), json);
    assert!(matches!(
        restored.render(80),
        Err(SigilStitchError::MissingLang { .. })
    ));
    assert_eq!(
        restored
            .with_lang(sigil_stitch::lang::rust::Rust::new())
            .render(80)
            .unwrap(),
        first_class.render(80).unwrap()
    );
    let extension = FileSpec::builder("outcome.rs")
        .add_spec(sum())
        .build()
        .unwrap();
    assert!(serde_json::to_value(extension).is_err());
}

#[derive(Debug)]
struct TraceLang {
    label: &'static str,
    fail: bool,
    events: Arc<Mutex<Vec<String>>>,
}
impl TraceLang {
    fn event(&self, name: &str) {
        self.events
            .lock()
            .unwrap()
            .push(format!("{}:{name}", self.label));
    }
}
impl RendererLang for TraceLang {
    fn file_extension(&self) -> &str {
        "sum"
    }
    fn line_comment_prefix(&self) -> &str {
        "//"
    }
    fn rewrite_nodes(&self, _: &mut Vec<CodeNode>) {
        self.event("rewrite");
    }
    fn lower_type_name(&self, ty: &TypeName) -> Result<CodeBlock, SigilStitchError> {
        self.event("type");
        assert!(matches!(ty, TypeName::Raw(name) if name == "Payload"));
        CodeBlock::of("Payload", ())
    }
    fn render_statement_end(&self) -> Result<&str, SigilStitchError> {
        self.event("render");
        Ok(";")
    }
}
impl CodeLang for TraceLang {
    fn render_imports(&self, _: &sigil_stitch::import::ImportGroup) -> String {
        self.event("imports");
        String::new()
    }
    fn capabilities(&self) -> LanguageCapabilities<'_> {
        LanguageCapabilities::permissive().with_closed_sum(ClosedSumCapabilityProfile::new(
            &[],
            &[ClosedSumCaseForm::Unit],
            false,
        ))
    }
    fn validate_closed_sum(&self, _: ClosedSumIntent<'_>) -> Result<(), SigilStitchError> {
        self.event("validate");
        Ok(())
    }
    fn lower_closed_sum(
        &self,
        _: ValidatedClosedSum<'_>,
    ) -> Result<Vec<CodeBlock>, SigilStitchError> {
        self.event("lower");
        if self.fail {
            return Err(SigilStitchError::InvalidTypeDeclaration {
                type_name: "Outcome".into(),
                reason: "lowerer failed".into(),
            });
        }
        let mut block = CodeBlock::builder();
        block.add_statement("sum %T", TypeName::raw("Payload"));
        Ok(vec![block.build()?])
    }
}

#[test]
fn project_lowerer_failure_stops_preparation_and_preserves_destination() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut project = ProjectSpec::builder();
    for (label, fail) in [("first", false), ("failing", true), ("last", false)] {
        project = project.add_file(
            FileSpec::builder_with(
                &format!("{label}.sum"),
                TraceLang {
                    label,
                    fail,
                    events: events.clone(),
                },
            )
            .add_closed_sum(sum())
            .build()
            .unwrap(),
        );
    }
    let project = project.build().unwrap();
    let dir = std::env::temp_dir().join(format!(
        "sigil-stitch-closed-sum-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&dir).unwrap();
    for label in ["first", "failing", "last"] {
        std::fs::write(dir.join(format!("{label}.sum")), "sentinel").unwrap();
    }
    assert!(
        matches!(project.write_to(&dir, 80), Err(SigilStitchError::InvalidTypeDeclaration { reason, .. }) if reason == "lowerer failed")
    );
    let trace = events.lock().unwrap();
    assert_eq!(
        &trace[..3],
        ["first:validate", "failing:validate", "last:validate"]
    );
    let lowering = trace.iter().position(|s| s == "first:lower").unwrap();
    let rendering = trace.iter().position(|s| s == "first:render").unwrap();
    let types = trace.iter().position(|s| s == "first:type").unwrap();
    let imports = trace.iter().position(|s| s == "first:imports").unwrap();
    let failure = trace.iter().position(|s| s == "failing:lower").unwrap();
    assert!(lowering < rendering && rendering < failure, "{trace:?}");
    assert!(
        lowering < types && types < imports && imports < rendering,
        "{trace:?}"
    );
    assert_eq!(trace.last().unwrap(), "failing:lower");
    assert_eq!(
        trace
            .iter()
            .filter(|s| s.ends_with(":rewrite"))
            .collect::<Vec<_>>(),
        vec!["first:rewrite"]
    );
    for label in ["first", "failing", "last"] {
        assert_eq!(
            std::fs::read_to_string(dir.join(format!("{label}.sum"))).unwrap(),
            "sentinel"
        );
    }
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 3);
    std::fs::remove_dir_all(&dir).unwrap();
}
