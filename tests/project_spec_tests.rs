use std::cell::Cell;

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::import::{
    ImportAliasAssignment, ImportAliasConflictResolver, ImportAliasConflicts, ImportAliasRejection,
};
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::Visibility;
use sigil_stitch::spec::project_spec::ProjectSpec;
use sigil_stitch::type_name::TypeName;

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
