use sigil_stitch::code_renderer::CodeRenderer;
use sigil_stitch::error::SigilStitchError;
use sigil_stitch::import::ImportGroup;
use sigil_stitch::lang::CodeLang;
use sigil_stitch::lang::rust::Rust;
use sigil_stitch::lang::typescript::TypeScript;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::modifiers::{DeclarationContext, Visibility};
use sigil_stitch::type_name::TypeName;

fn emit_field_ts(spec: &FieldSpec, ctx: DeclarationContext) -> String {
    let lang = TypeScript::new();
    let block = spec.emit(&lang, ctx).unwrap();
    let imports = ImportGroup::new();
    let mut renderer = CodeRenderer::new(&lang, &imports, 80);
    renderer.render(&block).unwrap()
}

fn emit_field_rs(spec: &FieldSpec, ctx: DeclarationContext) -> String {
    let lang = Rust::new();
    let block = spec.emit(&lang, ctx).unwrap();
    let imports = ImportGroup::new();
    let mut renderer = CodeRenderer::new(&lang, &imports, 80);
    renderer.render(&block).unwrap()
}

fn emit_for(lang: &dyn CodeLang, spec: &FieldSpec, ctx: DeclarationContext) -> String {
    let block = spec.emit(lang, ctx).unwrap();
    let imports = ImportGroup::new();
    let mut renderer = CodeRenderer::new(lang, &imports, 80);
    renderer.render(&block).unwrap()
}

fn optional_field(type_name: TypeName) -> FieldSpec {
    FieldSpec::builder("name", type_name)
        .is_optional()
        .build()
        .unwrap()
}

#[test]
fn test_ts_field_basic() {
    let field = FieldSpec::builder("name", TypeName::primitive("string"))
        .build()
        .unwrap();
    let output = emit_field_ts(&field, DeclarationContext::Member);
    assert_eq!(output.trim(), "name: string;");
}

#[test]
fn test_ts_field_with_visibility() {
    let field = FieldSpec::builder("name", TypeName::primitive("string"))
        .visibility(Visibility::Private)
        .build()
        .unwrap();
    let output = emit_field_ts(&field, DeclarationContext::Member);
    assert_eq!(output.trim(), "private name: string;");
}

#[test]
fn test_rust_field_basic() {
    let field = FieldSpec::builder("name", TypeName::primitive("String"))
        .visibility(Visibility::Public)
        .build()
        .unwrap();
    let output = emit_field_rs(&field, DeclarationContext::Member);
    assert_eq!(output.trim(), "pub name: String,");
}

#[test]
fn test_ts_field_readonly_static() {
    let field = FieldSpec::builder("MAX", TypeName::primitive("number"))
        .is_static()
        .is_readonly()
        .build()
        .unwrap();
    let output = emit_field_ts(&field, DeclarationContext::Member);
    assert_eq!(output.trim(), "static readonly MAX: number;");
}

#[test]
fn test_build_empty_name_errors() {
    let result = FieldSpec::builder("", TypeName::primitive("string")).build();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("'name' must not be empty")
    );
}

#[test]
fn test_ts_optional_field_uses_name_suffix() {
    let field = optional_field(TypeName::primitive("string"));
    let out = emit_for(&TypeScript::new(), &field, DeclarationContext::Member);
    assert_eq!(out.trim(), "name?: string;");
}

#[test]
fn optional_presence_is_rejected_when_only_nullable_values_are_supported() {
    let languages: Vec<Box<dyn CodeLang>> = vec![
        Box::new(Rust::new()),
        Box::new(sigil_stitch::lang::c::C::new()),
        Box::new(sigil_stitch::lang::cpp::Cpp::new()),
        Box::new(sigil_stitch::lang::csharp::CSharp::new()),
        Box::new(sigil_stitch::lang::dart::Dart::new()),
        Box::new(sigil_stitch::lang::go::Go::new()),
        Box::new(sigil_stitch::lang::haskell::Haskell::new()),
        Box::new(sigil_stitch::lang::java::Java::new()),
        Box::new(sigil_stitch::lang::javascript::JavaScript::new()),
        Box::new(sigil_stitch::lang::kotlin::Kotlin::new()),
        Box::new(sigil_stitch::lang::ocaml::OCaml::new()),
        Box::new(sigil_stitch::lang::php::Php::new()),
        Box::new(sigil_stitch::lang::python::Python::new()),
        Box::new(sigil_stitch::lang::scala::Scala::new()),
        Box::new(sigil_stitch::lang::swift::Swift::new()),
    ];
    let field = optional_field(TypeName::primitive("Value"));

    for lang in languages {
        assert!(matches!(
            field.emit(lang.as_ref(), DeclarationContext::Member),
            Err(SigilStitchError::UnsupportedFieldCapabilities { capabilities, .. })
                if capabilities.contains(
                    &sigil_stitch::lang::capability::FieldCapability::OptionalPresence
                )
        ));
    }
}

#[test]
fn optional_value_types_do_not_make_fields_omissible() {
    let cases: Vec<(Box<dyn CodeLang>, TypeName, &str)> = vec![
        (
            Box::new(Rust::new()),
            TypeName::primitive("String"),
            "name: Option<String>,",
        ),
        (
            Box::new(sigil_stitch::lang::c::C::new()),
            TypeName::primitive("int"),
            "int* name;",
        ),
        (
            Box::new(sigil_stitch::lang::cpp::Cpp::new()),
            TypeName::primitive("int"),
            "std::optional<int> name;",
        ),
        (
            Box::new(sigil_stitch::lang::go::Go::new()),
            TypeName::primitive("string"),
            "name *string",
        ),
        (
            Box::new(sigil_stitch::lang::java::Java::new()),
            TypeName::primitive("String"),
            "Optional<String> name;",
        ),
        (
            Box::new(sigil_stitch::lang::python::Python::new()),
            TypeName::primitive("str"),
            "name: str | None",
        ),
        (
            Box::new(sigil_stitch::lang::swift::Swift::new()),
            TypeName::primitive("String"),
            "var name: String?",
        ),
    ];

    for (lang, inner, expected) in cases {
        let field = FieldSpec::of("name", TypeName::optional(inner));
        let out = emit_for(lang.as_ref(), &field, DeclarationContext::Member);
        assert_eq!(out.trim(), expected, "{}", lang.file_extension());
    }
}

#[test]
fn test_ts_reserved_word_field_not_escaped() {
    let field = FieldSpec::builder("type", TypeName::primitive("string"))
        .is_readonly()
        .build()
        .unwrap();
    let out = emit_field_ts(&field, DeclarationContext::Member);
    assert_eq!(out.trim(), "readonly type: string;");
}

#[test]
fn test_go_reserved_word_field_is_escaped() {
    use sigil_stitch::lang::go::Go;
    let field = FieldSpec::builder("type", TypeName::primitive("string"))
        .build()
        .unwrap();
    let out = emit_for(&Go::new(), &field, DeclarationContext::Member);
    assert_eq!(out.trim(), "type_ string");
}
