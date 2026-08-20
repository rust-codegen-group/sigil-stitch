use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::error::SigilStitchError;
use sigil_stitch::lang::{CodeLang, RendererLang};
use sigil_stitch::spec::emittable::Emittable;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::TypeKind;
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::spec::where_spec::TypeParamSpec;
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

#[derive(Debug, Clone, Copy)]
enum FailingHook {
    Newtype,
    Context,
    Suffix,
}

#[derive(Debug)]
struct FailingHookLang(FailingHook);

impl RendererLang for FailingHookLang {
    fn file_extension(&self) -> &str {
        "fail"
    }

    fn line_comment_prefix(&self) -> &str {
        "//"
    }
}

impl CodeLang for FailingHookLang {
    fn emit_newtype_decl(
        &self,
        _visibility: &str,
        name: &str,
        _type_params: &[TypeParamSpec],
        inner: &TypeName,
    ) -> Result<CodeBlock, SigilStitchError> {
        if matches!(self.0, FailingHook::Newtype) {
            return CodeBlock::of("%T %T", inner.clone());
        }
        CodeBlock::of(&format!("struct {name}(%T);"), inner.clone())
    }

    fn emit_type_context(
        &self,
        _type_params: &[TypeParamSpec],
    ) -> Result<Option<CodeBlock>, SigilStitchError> {
        if matches!(self.0, FailingHook::Context) {
            return Err(SigilStitchError::Render {
                context: "emit_type_context".into(),
                message: "intentional hook error".into(),
            });
        }
        Ok(None)
    }

    fn emit_type_close_suffix(
        &self,
        _kind: TypeKind,
        _impl_types: &[TypeName],
    ) -> Result<Option<CodeBlock>, SigilStitchError> {
        if matches!(self.0, FailingHook::Suffix) {
            return Err(SigilStitchError::Render {
                context: "emit_type_close_suffix".into(),
                message: "intentional hook error".into(),
            });
        }
        Ok(None)
    }

    fn function_syntax(&self) -> sigil_stitch::lang::config::FunctionSyntaxConfig<'_> {
        sigil_stitch::lang::config::FunctionSyntaxConfig {
            function_signature_style: sigil_stitch::spec::fun_spec::FunctionSignatureStyle::Split,
            ..Default::default()
        }
    }
}

#[test]
fn test_structured_hook_errors_propagate_from_file_render() {
    let newtype = TypeSpec::builder("Wrapped", TypeKind::Newtype)
        .extends(TypeName::primitive("String"))
        .build()
        .unwrap();
    let newtype_error =
        FileSpec::builder_with("wrapped.fail", FailingHookLang(FailingHook::Newtype))
            .add_type(newtype)
            .build()
            .unwrap()
            .render(80)
            .unwrap_err();
    assert!(
        newtype_error
            .to_string()
            .contains("expects 2 args but got 1"),
        "{newtype_error}"
    );

    let function = FunSpec::builder("display")
        .add_type_param(TypeParamSpec::new("T"))
        .add_param(ParameterSpec::new("value", TypeName::primitive("T")).unwrap())
        .returns(TypeName::primitive("String"))
        .build()
        .unwrap();
    let context_error =
        FileSpec::builder_with("display.fail", FailingHookLang(FailingHook::Context))
            .add_function(function)
            .build()
            .unwrap()
            .render(80)
            .unwrap_err();
    assert!(
        context_error.to_string().contains("emit_type_context"),
        "{context_error}"
    );

    let type_spec = TypeSpec::builder("Record", TypeKind::Struct)
        .build()
        .unwrap();
    let suffix_error = FileSpec::builder_with("record.fail", FailingHookLang(FailingHook::Suffix))
        .add_type(type_spec)
        .build()
        .unwrap()
        .render(80)
        .unwrap_err();
    assert!(
        suffix_error.to_string().contains("emit_type_close_suffix"),
        "{suffix_error}"
    );
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

#[test]
fn validate_reports_unsupported_type_before_render() {
    let file = FileSpec::builder_with("user.bash", sigil_stitch::lang::bash::Bash::new())
        .add_type(TypeSpec::builder("User", TypeKind::Class).build().unwrap())
        .build()
        .unwrap();

    for error in [file.validate().unwrap_err(), file.render(80).unwrap_err()] {
        let SigilStitchError::FileSpecValidation {
            filename,
            error_count,
            errors,
        } = error
        else {
            panic!("expected FileSpecValidation, got {error:?}");
        };
        assert_eq!(filename, "user.bash");
        assert_eq!(error_count, 1);
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            &errors[0],
            SigilStitchError::UnsupportedTypeKind { type_name, .. } if type_name == "User"
        ));
    }
}

#[test]
fn validate_aggregates_multiple_unsupported_types() {
    let file = FileSpec::builder_with("user.bash", sigil_stitch::lang::bash::Bash::new())
        .add_type(TypeSpec::builder("User", TypeKind::Class).build().unwrap())
        .add_type(
            TypeSpec::builder("Account", TypeKind::Class)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let error = file.validate().unwrap_err();
    let SigilStitchError::FileSpecValidation {
        filename,
        error_count,
        errors,
    } = error
    else {
        panic!("expected FileSpecValidation, got {error:?}");
    };

    assert_eq!(filename, "user.bash");
    assert_eq!(error_count, 2);
    assert_eq!(errors.len(), 2);

    let mut names: Vec<_> = errors
        .iter()
        .map(|error| match error {
            SigilStitchError::UnsupportedTypeKind { type_name, .. } => type_name.as_str(),
            other => panic!("expected UnsupportedTypeKind, got {other:?}"),
        })
        .collect();
    names.sort_unstable();
    assert_eq!(names, ["Account", "User"]);

    let render_error = file.render(80).unwrap_err();
    assert!(matches!(
        render_error,
        SigilStitchError::FileSpecValidation { .. }
    ));
}

#[test]
fn validate_missing_lang_stays_direct() {
    let file = FileSpec::builder("empty.ts").build().unwrap();
    let json = serde_json::to_string(&file).unwrap();
    let deserialized: FileSpec = serde_json::from_str(&json).unwrap();

    let error = deserialized.validate().unwrap_err();
    assert!(matches!(
        error,
        SigilStitchError::MissingLang { ref filename } if filename == "empty.ts"
    ));
}

#[test]
fn legacy_adapter_defaults_to_permissive_capabilities() {
    let file = FileSpec::builder_with("wrapped.fail", FailingHookLang(FailingHook::Newtype))
        .add_type(
            TypeSpec::builder("Wrapper", TypeKind::Newtype)
                .extends(TypeName::primitive("String"))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    assert!(file.validate().is_ok());
}
