use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::code_renderer::CodeRenderer;
use sigil_stitch::import::ImportGroup;
use sigil_stitch::lang::rust_lang::RustLang;
use sigil_stitch::lang::typescript::TypeScript;
use sigil_stitch::spec::emittable::Emittable;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::fun_spec::{FunSpec, TypeParamSpec};
use sigil_stitch::spec::modifiers::{TypeKind, Visibility};
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

fn render_blocks_ts(blocks: &[CodeBlock]) -> String {
    let lang = TypeScript::new();
    let imports = ImportGroup::new();
    let mut output = String::new();
    for (i, block) in blocks.iter().enumerate() {
        if i > 0 {
            output.push('\n');
        }
        let mut renderer = CodeRenderer::new(&lang, &imports, 80);
        output.push_str(&renderer.render(block).unwrap());
    }
    output
}

fn render_blocks_rs(blocks: &[CodeBlock]) -> String {
    let lang = RustLang::new();
    let imports = ImportGroup::new();
    let mut output = String::new();
    for (i, block) in blocks.iter().enumerate() {
        if i > 0 {
            output.push('\n');
        }
        let mut renderer = CodeRenderer::new(&lang, &imports, 80);
        output.push_str(&renderer.render(block).unwrap());
    }
    output
}

#[test]
fn test_ts_class() {
    let body = CodeBlock::of("return this.name", ()).unwrap();
    let ts = TypeSpec::builder("UserService", TypeKind::Class)
        .visibility(Visibility::Public)
        .add_field(
            FieldSpec::builder("name", TypeName::primitive("string"))
                .visibility(Visibility::Private)
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("getName")
                .returns(TypeName::primitive("string"))
                .body(body)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let blocks = ts.emit(&TypeScript::new()).unwrap();
    let output = render_blocks_ts(&blocks);
    assert!(output.contains("export class UserService {"));
    assert!(output.contains("private name: string;"));
    assert!(output.contains("getName(): string {"));
    assert!(output.contains("return this.name"));
}

#[test]
fn test_ts_interface() {
    let ts = TypeSpec::builder("Repository", TypeKind::Interface)
        .visibility(Visibility::Public)
        .add_method(
            FunSpec::builder("findById")
                .add_param(ParameterSpec::new("id", TypeName::primitive("string")).unwrap())
                .returns(TypeName::generic(
                    TypeName::primitive("Promise"),
                    vec![TypeName::primitive("Entity")],
                ))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let blocks = ts.emit(&TypeScript::new()).unwrap();
    let output = render_blocks_ts(&blocks);
    assert!(output.contains("export interface Repository {"));
    assert!(output.contains("findById(id: string): Promise<Entity>;"));
}

#[test]
fn test_rust_struct_with_impl() {
    let body = CodeBlock::of("Self { name: name.to_string() }", ()).unwrap();
    let ts = TypeSpec::builder("Config", TypeKind::Struct)
        .visibility(Visibility::Public)
        .add_field(
            FieldSpec::builder("name", TypeName::primitive("String"))
                .visibility(Visibility::Public)
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("new")
                .visibility(Visibility::Public)
                .add_param(ParameterSpec::new("name", TypeName::primitive("&str")).unwrap())
                .returns(TypeName::primitive("Self"))
                .body(body)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let blocks = ts.emit(&RustLang::new()).unwrap();
    let output = render_blocks_rs(&blocks);
    assert!(output.contains("pub struct Config {"));
    assert!(output.contains("pub name: String,"));
    assert!(output.contains("impl Config {"));
    assert!(output.contains("pub fn new(name: &str) -> Self {"));
}

#[test]
fn test_ts_class_extends_implements() {
    let ts = TypeSpec::builder("AdminService", TypeKind::Class)
        .visibility(Visibility::Public)
        .extends(TypeName::primitive("BaseService"))
        .implements(TypeName::primitive("Serializable"))
        .build()
        .unwrap();

    let blocks = ts.emit(&TypeScript::new()).unwrap();
    let output = render_blocks_ts(&blocks);
    assert!(
        output.contains("export class AdminService extends BaseService implements Serializable {")
    );
}

#[test]
fn test_build_empty_name_errors() {
    let result = TypeSpec::builder("", TypeKind::Class).build();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("'name' must not be empty")
    );
}

#[test]
fn test_build_duplicate_field_name_errors() {
    let result = TypeSpec::builder("MyClass", TypeKind::Class)
        .add_field(
            FieldSpec::builder("name", TypeName::primitive("string"))
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("age", TypeName::primitive("number"))
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("name", TypeName::primitive("string"))
                .build()
                .unwrap(),
        )
        .build();
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("duplicate field name"));
    assert!(err_msg.contains("name"));
    assert!(err_msg.contains("MyClass"));
}

#[test]
fn test_build_no_duplicate_fields_ok() {
    let result = TypeSpec::builder("MyClass", TypeKind::Class)
        .add_field(
            FieldSpec::builder("name", TypeName::primitive("string"))
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("age", TypeName::primitive("number"))
                .build()
                .unwrap(),
        )
        .build();
    assert!(result.is_ok());
}

#[test]
fn test_type_alias_rust() {
    let spec = TypeSpec::builder("Meters", TypeKind::TypeAlias)
        .extends(TypeName::primitive("f64"))
        .build()
        .unwrap();
    let blocks = spec.emit(&RustLang::new()).unwrap();
    let output = render_blocks_rs(&blocks);
    assert_eq!(output.trim(), "type Meters = f64;");
}

#[test]
fn test_type_alias_rust_pub() {
    let spec = TypeSpec::builder("Meters", TypeKind::TypeAlias)
        .visibility(Visibility::Public)
        .extends(TypeName::primitive("f64"))
        .build()
        .unwrap();
    let blocks = spec.emit(&RustLang::new()).unwrap();
    let output = render_blocks_rs(&blocks);
    assert_eq!(output.trim(), "pub type Meters = f64;");
}

#[test]
fn test_type_alias_ts() {
    let spec = TypeSpec::builder("UserId", TypeKind::TypeAlias)
        .visibility(Visibility::Public)
        .extends(TypeName::primitive("string"))
        .build()
        .unwrap();
    let blocks = spec.emit(&TypeScript::new()).unwrap();
    let output = render_blocks_ts(&blocks);
    assert_eq!(output.trim(), "export type UserId = string;");
}

#[test]
fn test_type_alias_cpp() {
    use sigil_stitch::lang::cpp_lang::CppLang;
    let spec = TypeSpec::builder("Meters", TypeKind::TypeAlias)
        .extends(TypeName::primitive("double"))
        .build()
        .unwrap();
    let lang = CppLang::new();
    let imports = ImportGroup::new();
    let blocks = spec.emit(&lang).unwrap();
    let mut renderer = CodeRenderer::new(&lang, &imports, 80);
    let output = renderer.render(&blocks[0]).unwrap();
    assert_eq!(output.trim(), "using Meters = double;");
}

#[test]
fn test_type_alias_c() {
    use sigil_stitch::lang::c_lang::CLang;
    let spec = TypeSpec::builder("Meters", TypeKind::TypeAlias)
        .extends(TypeName::primitive("double"))
        .build()
        .unwrap();
    let lang = CLang::new();
    let imports = ImportGroup::new();
    let blocks = spec.emit(&lang).unwrap();
    let mut renderer = CodeRenderer::new(&lang, &imports, 80);
    let output = renderer.render(&blocks[0]).unwrap();
    assert_eq!(output.trim(), "typedef double Meters;");
}

#[test]
fn test_type_alias_go() {
    use sigil_stitch::lang::go_lang::GoLang;
    let spec = TypeSpec::builder("Meters", TypeKind::TypeAlias)
        .extends(TypeName::primitive("float64"))
        .build()
        .unwrap();
    let lang = GoLang::new();
    let imports = ImportGroup::new();
    let blocks = spec.emit(&lang).unwrap();
    let mut renderer = CodeRenderer::new(&lang, &imports, 80);
    let output = renderer.render(&blocks[0]).unwrap();
    assert_eq!(output.trim(), "type Meters = float64");
}

#[test]
fn test_type_alias_python() {
    use sigil_stitch::lang::python::Python;
    let spec = TypeSpec::builder("UserId", TypeKind::TypeAlias)
        .extends(TypeName::primitive("str"))
        .build()
        .unwrap();
    let lang = Python::new();
    let imports = ImportGroup::new();
    let blocks = spec.emit(&lang).unwrap();
    let mut renderer = CodeRenderer::new(&lang, &imports, 80);
    let output = renderer.render(&blocks[0]).unwrap();
    assert_eq!(output.trim(), "type UserId = str");
}

#[test]
fn test_type_alias_kotlin() {
    use sigil_stitch::lang::kotlin::Kotlin;
    let spec = TypeSpec::builder("Name", TypeKind::TypeAlias)
        .visibility(Visibility::Public)
        .extends(TypeName::primitive("String"))
        .build()
        .unwrap();
    let lang = Kotlin::new();
    let imports = ImportGroup::new();
    let blocks = spec.emit(&lang).unwrap();
    let mut renderer = CodeRenderer::new(&lang, &imports, 80);
    let output = renderer.render(&blocks[0]).unwrap();
    assert_eq!(output.trim(), "typealias Name = String");
}

#[test]
fn test_newtype_rust() {
    let spec = TypeSpec::builder("Meters", TypeKind::Newtype)
        .visibility(Visibility::Public)
        .extends(TypeName::primitive("f64"))
        .build()
        .unwrap();
    let blocks = spec.emit(&RustLang::new()).unwrap();
    let output = render_blocks_rs(&blocks);
    assert_eq!(output.trim(), "pub struct Meters(f64);");
}

#[test]
fn test_newtype_go() {
    use sigil_stitch::lang::go_lang::GoLang;
    let spec = TypeSpec::builder("Meters", TypeKind::Newtype)
        .extends(TypeName::primitive("float64"))
        .build()
        .unwrap();
    let lang = GoLang::new();
    let imports = ImportGroup::new();
    let blocks = spec.emit(&lang).unwrap();
    let mut renderer = CodeRenderer::new(&lang, &imports, 80);
    let output = renderer.render(&blocks[0]).unwrap();
    assert_eq!(output.trim(), "type Meters float64");
}

#[test]
fn test_newtype_kotlin() {
    use sigil_stitch::lang::kotlin::Kotlin;
    let spec = TypeSpec::builder("Meters", TypeKind::Newtype)
        .visibility(Visibility::Public)
        .extends(TypeName::primitive("Double"))
        .build()
        .unwrap();
    let lang = Kotlin::new();
    let imports = ImportGroup::new();
    let blocks = spec.emit(&lang).unwrap();
    let mut renderer = CodeRenderer::new(&lang, &imports, 80);
    let output = renderer.render(&blocks[0]).unwrap();
    assert_eq!(output.trim(), "value class Meters(val value: Double)");
}

#[test]
fn test_newtype_python() {
    use sigil_stitch::lang::python::Python;
    let spec = TypeSpec::builder("UserId", TypeKind::Newtype)
        .extends(TypeName::primitive("str"))
        .build()
        .unwrap();
    let lang = Python::new();
    let imports = ImportGroup::new();
    let blocks = spec.emit(&lang).unwrap();
    let mut renderer = CodeRenderer::new(&lang, &imports, 80);
    let output = renderer.render(&blocks[0]).unwrap();
    assert_eq!(output.trim(), "UserId = NewType(\"UserId\", str)");
}

#[test]
fn test_type_alias_validation_no_super_type() {
    let result = TypeSpec::builder("Foo", TypeKind::TypeAlias).build();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("expected exactly 1 super_type")
    );
}

#[test]
fn test_type_alias_validation_has_fields() {
    let result = TypeSpec::builder("Foo", TypeKind::TypeAlias)
        .extends(TypeName::primitive("string"))
        .add_field(
            FieldSpec::builder("x", TypeName::primitive("number"))
                .build()
                .unwrap(),
        )
        .build();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("must not have fields")
    );
}

#[test]
fn test_where_clause_rust_struct() {
    let body = CodeBlock::of("Self { value }", ()).unwrap();
    let type_spec = TypeSpec::builder("Container", TypeKind::Struct)
        .visibility(Visibility::Public)
        .add_type_param(TypeParamSpec::new("T"))
        .add_where_constraint(
            TypeName::primitive("T"),
            vec![TypeName::primitive("Clone"), TypeName::primitive("Send")],
        )
        .add_field(
            FieldSpec::builder("value", TypeName::primitive("T"))
                .visibility(Visibility::Public)
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("new")
                .visibility(Visibility::Public)
                .add_param(ParameterSpec::new("value", TypeName::primitive("T")).unwrap())
                .returns(TypeName::primitive("Self"))
                .body(body)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    let blocks = type_spec.emit(&RustLang::new()).unwrap();
    let output = render_blocks_rs(&blocks);
    assert!(
        output.contains("pub struct Container<T>"),
        "header: {output}"
    );
    assert!(
        output.contains("where\n    T: Clone + Send,"),
        "where on struct: {output}"
    );
    assert!(output.contains("impl<T> Container<T>"), "impl: {output}");
    assert!(
        output.contains("impl<T> Container<T>\nwhere\n    T: Clone + Send,"),
        "where on impl: {output}"
    );
}

#[test]
fn test_emittable_delegates_to_emit() {
    let ts = TypeSpec::builder("Greeter", TypeKind::Class)
        .build()
        .unwrap();
    let lang = TypeScript::new();
    let blocks = ts.emit_members(&lang).unwrap();
    assert!(!blocks.is_empty());
}

#[test]
fn test_emittable_returns_multiple_blocks_for_rust() {
    let ts = TypeSpec::builder("Greeter", TypeKind::Struct)
        .add_method(FunSpec::builder("hello").build().unwrap())
        .build()
        .unwrap();
    let lang = RustLang::new();
    let blocks = ts.emit_members(&lang).unwrap();
    assert!(
        blocks.len() >= 2,
        "Rust struct+impl should produce ≥2 blocks, got {}",
        blocks.len()
    );
}

// ── Embedded types ──────────────────────────────────────

fn render_blocks_go(blocks: &[CodeBlock]) -> String {
    use sigil_stitch::lang::go_lang::GoLang;
    let lang = GoLang::new();
    let imports = ImportGroup::new();
    let mut output = String::new();
    for (i, block) in blocks.iter().enumerate() {
        if i > 0 {
            output.push('\n');
        }
        let mut renderer = CodeRenderer::new(&lang, &imports, 80);
        output.push_str(&renderer.render(block).unwrap());
    }
    output
}

#[test]
fn test_embedded_go_struct_emit() {
    use sigil_stitch::lang::go_lang::GoLang;
    let spec = TypeSpec::builder("UserAdmin", TypeKind::Struct)
        .add_embedded(TypeName::primitive("User"))
        .add_embedded(TypeName::primitive("Admin"))
        .add_field(
            FieldSpec::builder("Role", TypeName::primitive("string"))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    let blocks = spec.emit(&GoLang::new()).unwrap();
    let output = render_blocks_go(&blocks);
    assert!(output.contains("User\n"), "embedded User: {output}");
    assert!(output.contains("Admin\n"), "embedded Admin: {output}");
    assert!(output.contains("Role string"), "field Role: {output}");
    let user_pos = output.find("User").unwrap();
    let role_pos = output.find("Role").unwrap();
    assert!(
        user_pos < role_pos,
        "embedded types should come before fields"
    );
}

#[test]
fn test_embedded_ts_interface_emit() {
    let spec = TypeSpec::builder("AdminUser", TypeKind::Interface)
        .add_embedded(TypeName::primitive("BaseUser"))
        .add_embedded(TypeName::primitive("AdminRole"))
        .add_field(
            FieldSpec::builder("permissions", TypeName::primitive("string[]"))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    let blocks = spec.emit(&TypeScript::new()).unwrap();
    let output = render_blocks_ts(&blocks);
    assert!(output.contains("BaseUser;"), "embedded BaseUser: {output}");
    assert!(
        output.contains("AdminRole;"),
        "embedded AdminRole: {output}"
    );
    assert!(
        output.contains("permissions: string[];"),
        "field permissions: {output}"
    );
}

#[test]
fn test_embedded_rust_struct_emit() {
    let spec = TypeSpec::builder("Combined", TypeKind::Struct)
        .add_embedded(TypeName::primitive("Base"))
        .add_field(
            FieldSpec::builder("extra", TypeName::primitive("String"))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    let blocks = spec.emit(&RustLang::new()).unwrap();
    let output = render_blocks_rs(&blocks);
    assert!(output.contains("Base,"), "embedded Base: {output}");
    assert!(output.contains("extra: String,"), "field extra: {output}");
}

#[test]
fn test_embedded_only_no_fields() {
    use sigil_stitch::lang::go_lang::GoLang;
    let spec = TypeSpec::builder("ReadCloser", TypeKind::Interface)
        .add_embedded(TypeName::primitive("Reader"))
        .add_embedded(TypeName::primitive("Closer"))
        .build()
        .unwrap();
    let blocks = spec.emit(&GoLang::new()).unwrap();
    let output = render_blocks_go(&blocks);
    assert!(output.contains("Reader\n"), "Reader embedded: {output}");
    assert!(output.contains("Closer\n"), "Closer embedded: {output}");
}

#[test]
fn test_embedded_with_methods_after() {
    let spec = TypeSpec::builder("Controller", TypeKind::Class)
        .add_embedded(TypeName::primitive("BaseHandler"))
        .add_field(
            FieldSpec::builder("name", TypeName::primitive("string"))
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("handle")
                .returns(TypeName::primitive("void"))
                .body(CodeBlock::of("// handle", ()).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    let blocks = spec.emit(&TypeScript::new()).unwrap();
    let output = render_blocks_ts(&blocks);
    assert!(output.contains("BaseHandler;"), "embedded: {output}");
    assert!(output.contains("name: string;"), "field: {output}");
    assert!(output.contains("handle(): void {"), "method: {output}");
    let embedded_pos = output.find("BaseHandler").unwrap();
    let field_pos = output.find("name:").unwrap();
    let method_pos = output.find("handle()").unwrap();
    assert!(embedded_pos < field_pos, "embedded before field");
    assert!(field_pos < method_pos, "field before method");
}

#[test]
fn test_embedded_import_tracking() {
    use sigil_stitch::lang::go_lang::GoLang;
    let io_reader = TypeName::importable("io", "Reader");
    let spec = TypeSpec::builder("MyReader", TypeKind::Struct)
        .add_embedded(io_reader)
        .build()
        .unwrap();

    let file = sigil_stitch::spec::file_spec::FileSpec::builder_with("reader.go", GoLang::new())
        .header(CodeBlock::of("package main", ()).unwrap())
        .add_type(spec)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();
    assert!(
        output.contains("import"),
        "should have import statement: {output}"
    );
    assert!(
        output.contains("\"io\""),
        "should import io package: {output}"
    );
}

#[test]
fn test_enum_constructor_with_valueless_variant_errors() {
    use sigil_stitch::spec::enum_variant_spec::EnumVariantSpec;

    let result = TypeSpec::builder("Status", TypeKind::Enum)
        .add_primary_constructor_param(
            ParameterSpec::builder(
                "value",
                sigil_stitch::type_name::TypeName::primitive("String"),
            )
            .is_property()
            .build()
            .unwrap(),
        )
        .add_variant(EnumVariantSpec::new("ACTIVE").unwrap())
        .build();

    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("some variants lack values"),
        "should error when enum has constructor but variants lack values"
    );
}

#[test]
fn test_enum_no_values_no_constructor_ok() {
    use sigil_stitch::spec::enum_variant_spec::EnumVariantSpec;

    let result = TypeSpec::builder("Color", TypeKind::Enum)
        .add_variant(EnumVariantSpec::new("RED").unwrap())
        .add_variant(EnumVariantSpec::new("GREEN").unwrap())
        .build();

    assert!(result.is_ok());
}

#[test]
fn test_enum_valued_variants_without_constructor_ok() {
    use sigil_stitch::spec::enum_variant_spec::EnumVariantSpec;

    let result = TypeSpec::builder("Direction", TypeKind::Enum)
        .add_variant(
            EnumVariantSpec::builder("UP")
                .value(CodeBlock::of("'UP'", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_variant(
            EnumVariantSpec::builder("DOWN")
                .value(CodeBlock::of("'DOWN'", ()).unwrap())
                .build()
                .unwrap(),
        )
        .build();

    assert!(
        result.is_ok(),
        "assignment-style valued enums should not require a constructor"
    );
}
