use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::error::SigilStitchError;
use sigil_stitch::lang::CodeLang;
use sigil_stitch::spec::emittable::Emittable;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::type_name::TypeName;

#[test]
fn test_empty_file() {
    let file = FileSpec::builder("empty.ts").build().unwrap();
    let output = file.render(80).unwrap();
    assert!(output.is_empty() || output.trim().is_empty());
}

#[test]
fn test_simple_file_with_import() {
    let user = TypeName::importable_type("./models", "User");

    let mut b = CodeBlock::builder();
    b.add_statement("const u: %T = getUser()", (user,));
    let block = b.build().unwrap();

    let file = FileSpec::builder("user.ts")
        .add_code(block)
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    assert!(output.contains("import type { User } from './models'"));
    assert!(output.contains("const u: User = getUser();"));
}

#[test]
fn test_conflicting_imports() {
    let user1 = TypeName::importable_type("./models", "User");
    let user2 = TypeName::importable_type("./other", "User");

    let mut b = CodeBlock::builder();
    b.add_statement("const u1: %T = get1()", (user1,));
    b.add_statement("const u2: %T = get2()", (user2,));
    let block = b.build().unwrap();

    let file = FileSpec::builder("user.ts")
        .add_code(block)
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    assert!(output.contains("const u1: User = get1();"));
    assert!(output.contains("const u2: OtherUser = get2();"));
    assert!(output.contains("User as OtherUser"));
}

#[test]
fn test_raw_content_no_import_tracking() {
    let file = FileSpec::builder("raw.ts")
        .add_raw("// This is raw content\nexport const VERSION = '1.0.0';\n")
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    assert!(output.contains("// This is raw content"));
    assert!(output.contains("export const VERSION = '1.0.0';"));
    assert!(!output.contains("import"));
}

#[test]
fn test_mixed_code_and_raw() {
    let user = TypeName::importable_type("./models", "User");

    let mut b = CodeBlock::builder();
    b.add_statement("const u: %T = getUser()", (user,));
    let block = b.build().unwrap();

    let file = FileSpec::builder("mixed.ts")
        .add_raw("// Generated file, do not edit.\n")
        .add_code(block)
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    assert!(output.contains("import type { User }"));
    assert!(output.contains("// Generated file"));
    assert!(output.contains("const u: User = getUser();"));
}

#[test]
fn test_file_with_header() {
    let mut header_builder = CodeBlock::builder();
    header_builder.add("// License: MIT", ());
    let header = header_builder.build().unwrap();

    let mut b = CodeBlock::builder();
    b.add_statement("const x = 1", ());
    let block = b.build().unwrap();

    let file = FileSpec::builder("test.ts")
        .header(header)
        .add_code(block)
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    assert!(output.starts_with("// License: MIT"));
    assert!(output.contains("const x = 1;"));
}

#[test]
fn test_dedup_same_import() {
    let user1 = TypeName::importable_type("./models", "User");
    let user2 = TypeName::importable_type("./models", "User");

    let mut b = CodeBlock::builder();
    b.add_statement("const u1: %T = get1()", (user1,));
    b.add_statement("const u2: %T = get2()", (user2,));
    let block = b.build().unwrap();

    let file = FileSpec::builder("user.ts")
        .add_code(block)
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    let import_count = output.matches("import type { User }").count();
    assert_eq!(import_count, 1);
}

#[test]
fn test_build_empty_filename_errors() {
    let result = FileSpec::builder("").build();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("'name' must not be empty")
    );
}

#[test]
fn test_aliased_type_in_codeblock() {
    let user = TypeName::importable("./models", "User").with_alias("UserModel");

    let mut b = CodeBlock::builder();
    b.add_statement("const u: %T = getUser()", (user,));
    let block = b.build().unwrap();

    let file = FileSpec::builder("user.ts")
        .add_code(block)
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    assert!(
        output.contains("User as UserModel"),
        "Expected aliased import, got:\n{output}"
    );
    assert!(
        output.contains("const u: UserModel = getUser();"),
        "Expected alias in code, got:\n{output}"
    );
}

#[test]
fn test_aliased_type_with_auto_alias_conflict() {
    let user1 = TypeName::importable_type("./models", "User").with_alias("ModelUser");
    let user2 = TypeName::importable_type("./other", "User");

    let mut b = CodeBlock::builder();
    b.add_statement("const u1: %T = get1()", (user1,));
    b.add_statement("const u2: %T = get2()", (user2,));
    let block = b.build().unwrap();

    let file = FileSpec::builder("user.ts")
        .add_code(block)
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    assert!(
        output.contains("const u1: ModelUser = get1();"),
        "Expected preferred alias, got:\n{output}"
    );
    assert!(
        output.contains("const u2: OtherUser = get2();"),
        "Expected auto-alias for second, got:\n{output}"
    );
}

#[test]
fn test_serde_round_trip_render_returns_error_without_lang() {
    let file = FileSpec::builder("test.ts")
        .add_code(CodeBlock::of("const x = 1", ()).unwrap())
        .build()
        .unwrap();

    let json = serde_json::to_string(&file).unwrap();
    let deserialized: FileSpec = serde_json::from_str(&json).unwrap();

    let err = deserialized.render(80).unwrap_err();
    assert!(err.to_string().contains("no language"));
}

#[test]
fn test_serde_round_trip_with_lang() {
    use sigil_stitch::lang::typescript::TypeScript;

    let mut b = CodeBlock::builder();
    b.add_statement("const x = 1", ());
    let file = FileSpec::builder("test.ts")
        .add_code(b.build().unwrap())
        .build()
        .unwrap();

    let json = serde_json::to_string(&file).unwrap();
    let deserialized: FileSpec = serde_json::from_str(&json).unwrap();

    let output = deserialized
        .with_lang(TypeScript::new())
        .render(80)
        .unwrap();
    assert!(
        output.contains("const x = 1;"),
        "Expected 'const x = 1;' in output:\n{output}"
    );
}

#[test]
fn test_custom_emittable_via_add_spec() {
    #[derive(Debug)]
    struct CommentSpec(&'static str);

    impl Emittable for CommentSpec {
        fn emit_members(&self, lang: &dyn CodeLang) -> Result<Vec<CodeBlock>, SigilStitchError> {
            let mut cb = CodeBlock::builder();
            let p = lang.line_comment_prefix();
            let s = lang.line_comment_suffix();
            cb.add(&format!("{p} {}{s}", self.0), ());
            Ok(vec![cb.build()?])
        }
    }

    let mut code_cb = CodeBlock::builder();
    code_cb.add_statement("const x = 1", ());

    let file = FileSpec::builder("test.ts")
        .add_code(code_cb.build().unwrap())
        .add_spec(CommentSpec("AUTO-GENERATED"))
        .add_function(FunSpec::builder("foo").build().unwrap())
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    assert!(output.contains("const x = 1;"), "code member: {output}");
    assert!(
        output.contains("// AUTO-GENERATED"),
        "spec member: {output}"
    );
    assert!(output.contains("function foo()"), "fun member: {output}");
}

#[test]
fn test_spec_with_imports() {
    #[derive(Debug)]
    struct TypedConstSpec(TypeName);

    impl Emittable for TypedConstSpec {
        fn emit_members(&self, _lang: &dyn CodeLang) -> Result<Vec<CodeBlock>, SigilStitchError> {
            let mut cb = CodeBlock::builder();
            cb.add_statement("const current: %T = null", (self.0.clone(),));
            Ok(vec![cb.build()?])
        }
    }

    let file = FileSpec::builder("test.ts")
        .add_spec(TypedConstSpec(TypeName::importable_type(
            "./models", "User",
        )))
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    assert!(
        output.contains("import type { User }"),
        "import should be collected from Spec member: {output}"
    );
    assert!(
        output.contains("const current: User = null;"),
        "body: {output}"
    );
}

#[test]
fn test_spec_error_propagation() {
    #[derive(Debug)]
    struct FailingSpec;

    impl Emittable for FailingSpec {
        fn emit_members(&self, _lang: &dyn CodeLang) -> Result<Vec<CodeBlock>, SigilStitchError> {
            Err(SigilStitchError::Render {
                context: "FailingSpec".into(),
                message: "intentional test error".into(),
            })
        }
    }

    let file = FileSpec::builder("test.ts")
        .add_spec(FailingSpec)
        .build()
        .unwrap();

    let err = file.render(80).unwrap_err();
    assert!(err.to_string().contains("intentional test error"), "{err}");
}

#[test]
fn test_serde_rejects_spec_variant() {
    #[derive(Debug)]
    struct Dummy;

    impl Emittable for Dummy {
        fn emit_members(&self, _lang: &dyn CodeLang) -> Result<Vec<CodeBlock>, SigilStitchError> {
            Ok(vec![])
        }
    }

    let file = FileSpec::builder("test.ts")
        .add_spec(Dummy)
        .build()
        .unwrap();

    let err = serde_json::to_string(&file).unwrap_err();
    assert!(err.to_string().contains("cannot be serialized"), "{err}");
}
