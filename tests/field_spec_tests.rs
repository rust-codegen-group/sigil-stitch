use sigil_stitch::code_renderer::CodeRenderer;
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
fn test_rust_optional_field_wraps_with_option() {
    let field = optional_field(TypeName::primitive("String"));
    let out = emit_for(&Rust::new(), &field, DeclarationContext::Member);
    assert_eq!(out.trim(), "name: Option<String>,");
}

#[test]
fn test_go_optional_field_prefixes_type_with_pointer() {
    use sigil_stitch::lang::go::Go;
    let field = optional_field(TypeName::primitive("string"));
    let out = emit_for(&Go::new(), &field, DeclarationContext::Member);
    assert_eq!(out.trim(), "name *string");
}

#[test]
fn test_python_optional_field_unions_with_none() {
    use sigil_stitch::lang::python::Python;
    let field = optional_field(TypeName::primitive("str"));
    let out = emit_for(&Python::new(), &field, DeclarationContext::Member);
    assert_eq!(out.trim(), "name: str | None");
}

#[test]
fn test_java_optional_field_wraps_with_optional() {
    use sigil_stitch::lang::java::Java;
    let field = optional_field(TypeName::primitive("String"));
    let out = emit_for(&Java::new(), &field, DeclarationContext::Member);
    assert_eq!(out.trim(), "Optional<String> name;");
}

#[test]
fn test_kotlin_optional_field_suffixes_type() {
    use sigil_stitch::lang::kotlin::Kotlin;
    let field = optional_field(TypeName::primitive("String"));
    let out = emit_for(&Kotlin::new(), &field, DeclarationContext::Member);
    assert!(
        out.contains("name: String?"),
        "expected type suffix '?', got {out:?}"
    );
}

#[test]
fn test_swift_optional_field_suffixes_type() {
    use sigil_stitch::lang::swift::Swift;
    let field = optional_field(TypeName::primitive("String"));
    let out = emit_for(&Swift::new(), &field, DeclarationContext::Member);
    assert!(
        out.contains("name: String?"),
        "expected type suffix '?', got {out:?}"
    );
}

#[test]
fn test_dart_optional_field_suffixes_type() {
    use sigil_stitch::lang::dart::Dart;
    let field = optional_field(TypeName::primitive("String"));
    let out = emit_for(&Dart::new(), &field, DeclarationContext::Member);
    assert_eq!(out.trim(), "String? name;");
}

#[test]
fn test_c_optional_field_prefixes_name_with_pointer() {
    use sigil_stitch::lang::c::C;
    let field = optional_field(TypeName::primitive("int"));
    let out = emit_for(&C::new(), &field, DeclarationContext::Member);
    assert_eq!(out.trim(), "int *name;");
}

#[test]
fn test_cpp_optional_field_wraps_with_std_optional() {
    use sigil_stitch::lang::cpp::Cpp;
    let field = optional_field(TypeName::primitive("int"));
    let out = emit_for(&Cpp::new(), &field, DeclarationContext::Member);
    assert_eq!(out.trim(), "std::optional<int> name;");
}

#[test]
fn test_javascript_optional_field_is_ignored() {
    use sigil_stitch::lang::javascript::JavaScript;
    let field = optional_field(TypeName::primitive("any"));
    let out = emit_for(&JavaScript::new(), &field, DeclarationContext::Member);
    assert!(out.contains("name"), "expected name in output, got {out:?}");
    assert!(
        !out.contains("?"),
        "JS output must not contain '?': {out:?}"
    );
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
