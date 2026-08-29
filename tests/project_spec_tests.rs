use std::cell::{Cell, RefCell};
use std::rc::Rc;

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::error::SigilStitchError;
use sigil_stitch::import::{
    ImportAliasAssignment, ImportAliasConflictResolver, ImportAliasConflicts, ImportAliasRejection,
};
use sigil_stitch::lang::CodeLang;
use sigil_stitch::spec::emittable::Emittable;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::Visibility;
use sigil_stitch::spec::project_spec::ProjectSpec;
use sigil_stitch::type_name::TypeName;

#[derive(Debug)]
struct ProjectValidationProbe {
    label: &'static str,
    validation_events: Rc<RefCell<Vec<&'static str>>>,
    emission_count: Rc<Cell<usize>>,
    validation_errors: &'static [&'static str],
}

impl Emittable for ProjectValidationProbe {
    fn collect_validation_errors(&self, _lang: &dyn CodeLang, errors: &mut Vec<SigilStitchError>) {
        self.validation_events.borrow_mut().push(self.label);
        errors.extend(
            self.validation_errors
                .iter()
                .map(|message| SigilStitchError::Render {
                    context: format!("validating {}", self.label),
                    message: (*message).to_string(),
                }),
        );
    }

    fn emit_members(&self, _lang: &dyn CodeLang) -> Result<Vec<CodeBlock>, SigilStitchError> {
        self.emission_count.set(self.emission_count.get() + 1);
        Ok(vec![CodeBlock::of(self.label, ())?])
    }
}

fn validation_probe_file(
    filename: &str,
    label: &'static str,
    validation_events: Rc<RefCell<Vec<&'static str>>>,
    emission_count: Rc<Cell<usize>>,
    validation_errors: &'static [&'static str],
) -> FileSpec {
    FileSpec::builder(filename)
        .add_spec(ProjectValidationProbe {
            label,
            validation_events,
            emission_count,
            validation_errors,
        })
        .build()
        .unwrap()
}

// ── Empty project ───────────────────────────────────────

#[test]
fn test_empty_project_renders_empty_vec() {
    let project = ProjectSpec::builder().build().unwrap();
    let rendered = project.render(80).unwrap();
    assert!(rendered.is_empty());
}

// ── Single file ─────────────────────────────────────────

#[test]
fn test_single_file_project() {
    let project = ProjectSpec::builder()
        .add_file(
            FileSpec::builder("index.ts")
                .add_code(CodeBlock::of("console.log('hello')", ()).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let rendered = project.render(80).unwrap();
    assert_eq!(rendered.len(), 1);
    assert_eq!(rendered[0].path, "index.ts");
    assert!(rendered[0].content.contains("console.log('hello')"));
}

// ── Multi-file with imports ─────────────────────────────

#[test]
fn test_multi_file_project_with_imports() {
    // File 1: models.ts
    let f1 = FileSpec::builder("models.ts")
        .add_code(CodeBlock::of("export interface User { name: string }", ()).unwrap());

    // File 2: service.ts — imports User from models
    let user_type = TypeName::importable_type("./models", "User");
    let mut cb = CodeBlock::builder();
    cb.add_statement("const u: %T = getUser()", (user_type,));
    let f2 = FileSpec::builder("service.ts").add_code(cb.build().unwrap());

    let project = ProjectSpec::builder()
        .add_file(f1.build().unwrap())
        .add_file(f2.build().unwrap())
        .build()
        .unwrap();

    let rendered = project.render(80).unwrap();
    assert_eq!(rendered.len(), 2);
    assert_eq!(rendered[0].path, "models.ts");
    assert_eq!(rendered[1].path, "service.ts");
    // Each file resolves imports independently.
    assert!(
        rendered[1]
            .content
            .contains("import type { User } from './models'")
    );
}

// ── File ordering preserved ─────────────────────────────

#[test]
fn test_file_ordering_preserved() {
    let mut pb = ProjectSpec::builder();
    for name in ["c.ts", "a.ts", "b.ts"] {
        pb = pb.add_file(
            FileSpec::builder(name)
                .add_code(CodeBlock::of("// placeholder", ()).unwrap())
                .build()
                .unwrap(),
        );
    }
    let rendered = pb.build().unwrap().render(80).unwrap();
    let paths: Vec<&str> = rendered.iter().map(|r| r.path.as_str()).collect();
    assert_eq!(paths, vec!["c.ts", "a.ts", "b.ts"]);
}

#[test]
fn project_validation_aggregates_complete_file_errors_in_project_order() {
    let validation_events = Rc::new(RefCell::new(Vec::new()));
    let emission_count = Rc::new(Cell::new(0));
    let project = ProjectSpec::builder()
        .add_file(validation_probe_file(
            "first.ts",
            "first",
            validation_events.clone(),
            emission_count.clone(),
            &["first-a", "first-b"],
        ))
        .add_file(validation_probe_file(
            "valid.ts",
            "valid",
            validation_events.clone(),
            emission_count.clone(),
            &[],
        ))
        .add_file(validation_probe_file(
            "third.ts",
            "third",
            validation_events.clone(),
            emission_count.clone(),
            &["third-a"],
        ))
        .build()
        .unwrap();

    let error = project.validate().unwrap_err();
    let display = error.to_string();
    let mut remaining_display = display.as_str();
    for expected in [
        "ProjectSpec has 2 invalid file(s):",
        "FileSpecValidation { filename: \"first.ts\", error_count: 2",
        "message: \"first-a\"",
        "message: \"first-b\"",
        "FileSpecValidation { filename: \"third.ts\", error_count: 1",
        "message: \"third-a\"",
    ] {
        let Some((_, remaining)) = remaining_display.split_once(expected) else {
            panic!("expected {expected:?} in order within {display:?}");
        };
        remaining_display = remaining;
    }

    let SigilStitchError::ProjectSpecValidation {
        invalid_file_count,
        errors,
    } = error
    else {
        panic!("expected ProjectSpecValidation");
    };

    assert_eq!(invalid_file_count, 2);
    assert_eq!(errors.len(), 2);
    let SigilStitchError::FileSpecValidation {
        filename,
        error_count,
        errors: member_errors,
    } = &errors[0]
    else {
        panic!("expected first FileSpecValidation");
    };
    assert_eq!(filename, "first.ts");
    assert_eq!(*error_count, 2);
    assert!(member_errors[0].to_string().contains("first-a"));
    assert!(member_errors[1].to_string().contains("first-b"));

    let SigilStitchError::FileSpecValidation {
        filename,
        error_count,
        errors: member_errors,
    } = &errors[1]
    else {
        panic!("expected third FileSpecValidation");
    };
    assert_eq!(filename, "third.ts");
    assert_eq!(*error_count, 1);
    assert!(member_errors[0].to_string().contains("third-a"));
    assert_eq!(
        validation_events.borrow().as_slice(),
        &["first", "valid", "third"]
    );
    assert_eq!(emission_count.get(), 0);
}

#[test]
fn project_render_validates_every_file_before_emitting_any_file() {
    let validation_events = Rc::new(RefCell::new(Vec::new()));
    let emission_count = Rc::new(Cell::new(0));
    let project = ProjectSpec::builder()
        .add_file(validation_probe_file(
            "valid.ts",
            "valid",
            validation_events.clone(),
            emission_count.clone(),
            &[],
        ))
        .add_file(validation_probe_file(
            "invalid.ts",
            "invalid",
            validation_events.clone(),
            emission_count.clone(),
            &["invalid member"],
        ))
        .build()
        .unwrap();

    let error = project.render(80).unwrap_err();

    assert!(matches!(
        error,
        SigilStitchError::ProjectSpecValidation {
            invalid_file_count: 1,
            ..
        }
    ));
    assert_eq!(validation_events.borrow().as_slice(), &["valid", "invalid"]);
    assert_eq!(emission_count.get(), 0);
}

#[test]
fn project_write_performs_no_writes_when_validation_fails() {
    let validation_events = Rc::new(RefCell::new(Vec::new()));
    let emission_count = Rc::new(Cell::new(0));
    let project = ProjectSpec::builder()
        .add_file(validation_probe_file(
            "nested/valid.ts",
            "valid",
            validation_events.clone(),
            emission_count.clone(),
            &[],
        ))
        .add_file(validation_probe_file(
            "nested/invalid.ts",
            "invalid",
            validation_events,
            emission_count.clone(),
            &["invalid member"],
        ))
        .build()
        .unwrap();
    let output_dir = std::env::temp_dir().join(format!(
        "sigil_stitch_project_validation_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&output_dir);

    let error = project.write_to(&output_dir, 80).unwrap_err();

    assert!(matches!(
        error,
        SigilStitchError::ProjectSpecValidation {
            invalid_file_count: 1,
            ..
        }
    ));
    assert_eq!(emission_count.get(), 0);
    assert!(!output_dir.exists());
}

// ── Render error includes filename ──────────────────────

#[test]
fn test_render_error_includes_filename() {
    let result = ProjectSpec::builder()
        .add_file(
            FileSpec::builder("app.ts")
                .add_code(CodeBlock::of("const x = 1", ()).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap()
        .render(80);
    assert!(result.is_ok());
}

// ── Duplicate filename detection ────────────────────────

#[test]
fn test_duplicate_filename_rejected() {
    let result = ProjectSpec::builder()
        .add_file(
            FileSpec::builder("user.ts")
                .add_code(CodeBlock::of("// first", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_file(
            FileSpec::builder("user.ts")
                .add_code(CodeBlock::of("// second", ()).unwrap())
                .build()
                .unwrap(),
        )
        .build();

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("duplicate filename") && err.contains("user.ts"),
        "Expected duplicate filename error, got: {err}"
    );
}

#[test]
fn test_distinct_filenames_accepted() {
    let result = ProjectSpec::builder()
        .add_file(
            FileSpec::builder("a.ts")
                .add_code(CodeBlock::of("// a", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_file(
            FileSpec::builder("b.ts")
                .add_code(CodeBlock::of("// b", ()).unwrap())
                .build()
                .unwrap(),
        )
        .build();

    assert!(result.is_ok());
}

// ── write_to creates files on disk ──────────────────────

#[test]
fn test_write_to_creates_files() {
    let dir = std::env::temp_dir().join("sigil_stitch_test_write_to");
    // Clean up from any previous run.
    let _ = std::fs::remove_dir_all(&dir);

    let written = ProjectSpec::builder()
        .add_file(
            FileSpec::builder("hello.ts")
                .add_code(CodeBlock::of("export const x = 1", ()).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap()
        .write_to(&dir, 80)
        .unwrap();
    assert_eq!(written.len(), 1);
    assert_eq!(written[0], dir.join("hello.ts"));
    let content = std::fs::read_to_string(&written[0]).unwrap();
    assert!(content.contains("export const x = 1"));

    // Clean up.
    let _ = std::fs::remove_dir_all(&dir);
}

// ── write_to creates nested directories ─────────────────

#[test]
fn test_write_to_creates_nested_dirs() {
    let dir = std::env::temp_dir().join("sigil_stitch_test_nested");
    let _ = std::fs::remove_dir_all(&dir);

    let written = ProjectSpec::builder()
        .add_file(
            FileSpec::builder("src/models/user.ts")
                .add_code(CodeBlock::of("export class User {}", ()).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap()
        .write_to(&dir, 80)
        .unwrap();
    assert_eq!(written.len(), 1);
    assert_eq!(written[0], dir.join("src/models/user.ts"));
    assert!(written[0].exists());

    let _ = std::fs::remove_dir_all(&dir);
}

// ── Multi-language smoke test ───────────────────────────

#[test]
fn test_rust_project() {
    let rendered = ProjectSpec::builder()
        .add_file(
            FileSpec::builder("lib.rs")
                .add_function(
                    FunSpec::builder("greet")
                        .visibility(Visibility::Public)
                        .returns(TypeName::primitive("String"))
                        .body(CodeBlock::of("String::from(\"hello\")", ()).unwrap())
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    assert_eq!(rendered.len(), 1);
    assert_eq!(rendered[0].path, "lib.rs");
    assert!(rendered[0].content.contains("pub fn greet() -> String {"));
}

#[test]
fn test_multi_file_rust_project() {
    let f1 =
        FileSpec::builder("types.rs").add_code(CodeBlock::of("pub struct Config {}", ()).unwrap());

    let config_type = TypeName::importable("crate::types", "Config");
    let mut cb = CodeBlock::builder();
    cb.add_statement("let _cfg: %T = Config::default()", (config_type,));
    let f2 = FileSpec::builder("main.rs").add_code(cb.build().unwrap());

    let rendered = ProjectSpec::builder()
        .add_file(f1.build().unwrap())
        .add_file(f2.build().unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    assert_eq!(rendered.len(), 2);
    assert_eq!(rendered[0].path, "types.rs");
    assert_eq!(rendered[1].path, "main.rs");
    assert!(rendered[1].content.contains("use crate::types::Config;"));
}

struct RejectSecondFileResolver {
    calls: Cell<usize>,
}

impl ImportAliasConflictResolver for RejectSecondFileResolver {
    fn resolve(
        &self,
        conflicts: &ImportAliasConflicts<'_>,
    ) -> Result<Vec<ImportAliasAssignment>, ImportAliasRejection> {
        let call = self.calls.get() + 1;
        self.calls.set(call);
        if call == 2 {
            return Err(ImportAliasRejection::new("second file rejected"));
        }
        Ok(conflicts
            .conflicts()
            .iter()
            .flat_map(|conflict| conflict.claims())
            .enumerate()
            .map(|(index, claim)| {
                ImportAliasAssignment::new(claim.id(), format!("Resolved{index}"))
            })
            .collect())
    }
}

fn file_with_import_conflict(filename: &str) -> FileSpec {
    FileSpec::builder(filename)
        .add_code(
            CodeBlock::of(
                "%T %T",
                (
                    TypeName::importable_type("./models", "User"),
                    TypeName::importable_type("./other", "User"),
                ),
            )
            .unwrap(),
        )
        .build()
        .unwrap()
}

#[test]
fn project_custom_resolver_is_file_local_and_write_is_all_or_error() {
    let project = ProjectSpec::builder()
        .add_file(file_with_import_conflict("first.ts"))
        .add_file(file_with_import_conflict("second.ts"))
        .build()
        .unwrap();
    let resolver = RejectSecondFileResolver {
        calls: Cell::new(0),
    };
    let output_dir = std::env::temp_dir().join(format!(
        "sigil_stitch_alias_resolver_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&output_dir);

    let error = project
        .write_to_with_import_alias_resolver(&output_dir, 80, &resolver)
        .unwrap_err();

    assert_eq!(resolver.calls.get(), 2);
    assert!(error.to_string().contains("second file rejected"));
    assert!(!output_dir.exists());
}
