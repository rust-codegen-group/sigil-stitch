use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::code_renderer::CodeRenderer;
use sigil_stitch::import::ImportGroup;
use sigil_stitch::lang::CodeLang;
use sigil_stitch::lang::capability::TypeCapability;
use sigil_stitch::lang::rust::Rust;
use sigil_stitch::lang::typescript::TypeScript;
use sigil_stitch::spec::emittable::Emittable;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::{TypeKind, Visibility};
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::spec::where_spec::TypeParamSpec;
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
    let lang = Rust::new();
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

fn render_newtype_file(lang: impl CodeLang, filename: &str, inner: TypeName) -> String {
    let newtype = TypeSpec::builder("Wrapper", TypeKind::Newtype)
        .extends(inner)
        .build()
        .unwrap();
    FileSpec::builder_with(filename, lang)
        .add_type(newtype)
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

fn render_generic_newtype_file(
    lang: impl CodeLang,
    filename: &str,
    type_param: TypeParamSpec,
    inner: TypeName,
) -> String {
    let newtype = TypeSpec::builder("Wrapper", TypeKind::Newtype)
        .add_type_param(type_param)
        .extends(inner)
        .build()
        .unwrap();
    FileSpec::builder_with(filename, lang)
        .add_type(newtype)
        .build()
        .unwrap()
        .render(80)
        .unwrap()
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

    let blocks = ts.emit(&Rust::new()).unwrap();
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
    let blocks = spec.emit(&Rust::new()).unwrap();
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
    let blocks = spec.emit(&Rust::new()).unwrap();
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
    use sigil_stitch::lang::cpp::Cpp;
    let spec = TypeSpec::builder("Meters", TypeKind::TypeAlias)
        .extends(TypeName::primitive("double"))
        .build()
        .unwrap();
    let lang = Cpp::new();
    let imports = ImportGroup::new();
    let blocks = spec.emit(&lang).unwrap();
    let mut renderer = CodeRenderer::new(&lang, &imports, 80);
    let output = renderer.render(&blocks[0]).unwrap();
    assert_eq!(output.trim(), "using Meters = double;");
}

#[test]
fn test_type_alias_c() {
    use sigil_stitch::lang::c::C;
    let spec = TypeSpec::builder("Meters", TypeKind::TypeAlias)
        .extends(TypeName::primitive("double"))
        .build()
        .unwrap();
    let lang = C::new();
    let imports = ImportGroup::new();
    let blocks = spec.emit(&lang).unwrap();
    let mut renderer = CodeRenderer::new(&lang, &imports, 80);
    let output = renderer.render(&blocks[0]).unwrap();
    assert_eq!(output.trim(), "typedef double Meters;");
}

#[test]
fn test_type_alias_go() {
    use sigil_stitch::lang::go::Go;
    let spec = TypeSpec::builder("Meters", TypeKind::TypeAlias)
        .extends(TypeName::primitive("float64"))
        .build()
        .unwrap();
    let lang = Go::new();
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
    let blocks = spec.emit(&Rust::new()).unwrap();
    let output = render_blocks_rs(&blocks);
    assert_eq!(output.trim(), "pub struct Meters(f64);");
}

#[test]
fn test_newtype_go() {
    use sigil_stitch::lang::go::Go;
    let spec = TypeSpec::builder("Meters", TypeKind::Newtype)
        .extends(TypeName::primitive("float64"))
        .build()
        .unwrap();
    let lang = Go::new();
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
fn test_newtype_imports_survive_every_structured_hook() {
    use sigil_stitch::lang::c::C;
    use sigil_stitch::lang::go::Go;
    use sigil_stitch::lang::haskell::Haskell;
    use sigil_stitch::lang::kotlin::Kotlin;
    use sigil_stitch::lang::php::Php;
    use sigil_stitch::lang::python::Python;
    use sigil_stitch::lang::scala::Scala;

    let rust = render_newtype_file(
        Rust::new(),
        "wrapper.rs",
        TypeName::optional(TypeName::importable("crate::models", "External")),
    );
    assert!(rust.contains("use crate::models::External;"), "{rust}");
    assert!(rust.contains("struct Wrapper(Option<External>);"), "{rust}");

    let c = render_newtype_file(
        C::new(),
        "wrapper.c",
        TypeName::pointer(TypeName::importable("external.h", "External")),
    );
    assert!(c.contains("#include <external.h>"), "{c}");
    assert!(c.contains("typedef External* Wrapper;"), "{c}");

    let go = render_newtype_file(
        Go::new(),
        "wrapper.go",
        TypeName::optional(TypeName::importable(
            "example.com/project/external",
            "External",
        )),
    );
    assert!(go.contains("\"example.com/project/external\""), "{go}");
    assert!(go.contains("type Wrapper *external.External"), "{go}");

    let haskell = render_newtype_file(
        Haskell::new(),
        "Wrapper.hs",
        TypeName::optional(TypeName::importable("External.Types", "External")),
    );
    assert!(
        haskell.contains("import External.Types (External)"),
        "{haskell}"
    );
    assert!(
        haskell.contains("newtype Wrapper = Wrapper (Maybe External)"),
        "{haskell}"
    );

    let kotlin = render_newtype_file(
        Kotlin::new(),
        "Wrapper.kt",
        TypeName::optional(TypeName::importable("example.models", "External")),
    );
    assert!(
        kotlin.contains("import example.models.External"),
        "{kotlin}"
    );
    assert!(
        kotlin.contains("value class Wrapper(val value: External?)"),
        "{kotlin}"
    );

    let php = render_newtype_file(
        Php::new(),
        "Wrapper.php",
        TypeName::optional(TypeName::importable("Example\\Models", "External")),
    );
    assert!(php.contains("use Example\\Models\\External;"), "{php}");
    assert!(php.contains("private ?External $value"), "{php}");

    let python = render_newtype_file(
        Python::new(),
        "wrapper.py",
        TypeName::optional(TypeName::importable("example.models", "External")),
    );
    assert!(
        python.contains("from example.models import External"),
        "{python}"
    );
    assert!(
        python.contains("Wrapper = NewType(\"Wrapper\", External | None)"),
        "{python}"
    );

    let scala = render_newtype_file(
        Scala::new(),
        "Wrapper.scala",
        TypeName::optional(TypeName::importable("example.models", "External")),
    );
    assert!(scala.contains("import example.models.External"), "{scala}");
    assert!(
        scala.contains("class Wrapper(val value: Option[External])"),
        "{scala}"
    );
}

#[test]
fn test_newtype_type_params_are_owned_by_each_language() {
    use sigil_stitch::lang::c::C;
    use sigil_stitch::lang::go::Go;
    use sigil_stitch::lang::haskell::Haskell;
    use sigil_stitch::lang::kotlin::Kotlin;
    use sigil_stitch::lang::php::Php;
    use sigil_stitch::lang::python::Python;
    use sigil_stitch::lang::scala::Scala;

    let rust = render_generic_newtype_file(
        Rust::new(),
        "wrapper.rs",
        TypeParamSpec::new("T").with_bound(TypeName::importable("crate::traits", "Bound")),
        TypeName::primitive("T"),
    );
    assert!(rust.contains("use crate::traits::Bound;"), "{rust}");
    assert!(rust.contains("struct Wrapper<T: Bound>(T);"), "{rust}");

    let go = render_generic_newtype_file(
        Go::new(),
        "wrapper.go",
        TypeParamSpec::new("T").with_bound(TypeName::importable(
            "example.com/project/constraints",
            "Bound",
        )),
        TypeName::primitive("T"),
    );
    assert!(go.contains("\"example.com/project/constraints\""), "{go}");
    assert!(go.contains("type Wrapper[T constraints.Bound] T"), "{go}");

    let haskell = render_generic_newtype_file(
        Haskell::new(),
        "Wrapper.hs",
        TypeParamSpec::new("a").with_bound(TypeName::importable("Constraints", "Bound")),
        TypeName::primitive("a"),
    );
    assert!(haskell.contains("import Constraints (Bound)"), "{haskell}");
    assert!(
        haskell.contains("newtype Bound a => Wrapper a = Wrapper a"),
        "{haskell}"
    );

    let kotlin = render_generic_newtype_file(
        Kotlin::new(),
        "Wrapper.kt",
        TypeParamSpec::new("T").with_bound(TypeName::importable("example.constraints", "Bound")),
        TypeName::primitive("T"),
    );
    assert!(
        kotlin.contains("import example.constraints.Bound"),
        "{kotlin}"
    );
    assert!(
        kotlin.contains("value class Wrapper<T : Bound>(val value: T)"),
        "{kotlin}"
    );

    let scala = render_generic_newtype_file(
        Scala::new(),
        "Wrapper.scala",
        TypeParamSpec::new("T").with_bound(TypeName::importable("example.constraints", "Bound")),
        TypeName::primitive("T"),
    );
    assert!(
        scala.contains("import example.constraints.Bound"),
        "{scala}"
    );
    assert!(
        scala.contains("class Wrapper[T <: Bound](val value: T)"),
        "{scala}"
    );

    let c = render_generic_newtype_file(
        C::new(),
        "wrapper.c",
        TypeParamSpec::new("T").with_bound(TypeName::importable("bound.h", "Bound")),
        TypeName::primitive("int"),
    );
    assert!(!c.contains("bound.h"), "{c}");
    assert!(c.contains("typedef int Wrapper;"), "{c}");

    let php = render_generic_newtype_file(
        Php::new(),
        "Wrapper.php",
        TypeParamSpec::new("T").with_bound(TypeName::importable("Example\\Constraints", "Bound")),
        TypeName::primitive("int"),
    );
    assert!(!php.contains("Constraints"), "{php}");
    assert!(php.contains("class Wrapper"), "{php}");

    let python = render_generic_newtype_file(
        Python::new(),
        "wrapper.py",
        TypeParamSpec::new("T").with_bound(TypeName::importable("example.constraints", "Bound")),
        TypeName::primitive("int"),
    );
    assert!(!python.contains("example.constraints"), "{python}");
    assert!(
        python.contains("Wrapper = NewType(\"Wrapper\", int)"),
        "{python}"
    );
}

#[test]
fn test_newtype_hook_uses_resolved_preferred_alias() {
    use sigil_stitch::lang::php::Php;

    let output = render_newtype_file(
        Php::new(),
        "Wrapper.php",
        TypeName::optional(
            TypeName::importable("Example\\Models", "External").with_alias("ModelExternal"),
        ),
    );

    assert!(
        output.contains("use Example\\Models\\External as ModelExternal;"),
        "{output}"
    );
    assert!(output.contains("private ?ModelExternal $value"), "{output}");
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
    let blocks = type_spec.emit(&Rust::new()).unwrap();
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
        .add_method(
            FunSpec::builder("hello")
                .body(CodeBlock::of("()", ()).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    let lang = Rust::new();
    let blocks = ts.emit_members(&lang).unwrap();
    assert!(
        blocks.len() >= 2,
        "Rust struct+impl should produce ≥2 blocks, got {}",
        blocks.len()
    );
}

// ── Embedded types ──────────────────────────────────────

fn render_blocks_go(blocks: &[CodeBlock]) -> String {
    use sigil_stitch::lang::go::Go;
    let lang = Go::new();
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
    use sigil_stitch::lang::go::Go;
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
    let blocks = spec.emit(&Go::new()).unwrap();
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
    let blocks = spec.emit(&Rust::new()).unwrap();
    let output = render_blocks_rs(&blocks);
    assert!(output.contains("Base,"), "embedded Base: {output}");
    assert!(output.contains("extra: String,"), "field extra: {output}");
}

#[test]
fn test_embedded_only_no_fields() {
    use sigil_stitch::lang::go::Go;
    let spec = TypeSpec::builder("ReadCloser", TypeKind::Interface)
        .add_embedded(TypeName::primitive("Reader"))
        .add_embedded(TypeName::primitive("Closer"))
        .build()
        .unwrap();
    let blocks = spec.emit(&Go::new()).unwrap();
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
    use sigil_stitch::lang::go::Go;
    let io_reader = TypeName::importable("io", "Reader");
    let spec = TypeSpec::builder("MyReader", TypeKind::Struct)
        .add_embedded(io_reader)
        .build()
        .unwrap();

    let file = sigil_stitch::spec::file_spec::FileSpec::builder_with("reader.go", Go::new())
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

#[test]
fn unsupported_builtin_type_kinds_fail_closed() {
    let cases: &[(&dyn CodeLang, &str)] = &[
        (&sigil_stitch::lang::bash::Bash::new(), "user.bash"),
        (&sigil_stitch::lang::zsh::Zsh::new(), "user.zsh"),
        (&sigil_stitch::lang::lua::Lua::new(), "user.lua"),
    ];

    for (lang, filename) in cases {
        let spec = TypeSpec::builder("User", TypeKind::Class).build().unwrap();
        let error = spec.emit(*lang).unwrap_err();
        assert!(
            matches!(
                error,
                sigil_stitch::error::SigilStitchError::UnsupportedTypeKind {
                    kind: TypeKind::Class,
                    ..
                }
            ),
            "{filename}: {error}"
        );
    }
}

#[test]
fn go_enum_fails_closed() {
    let go = sigil_stitch::lang::go::Go::new();
    let spec = TypeSpec::builder("Color", TypeKind::Enum).build().unwrap();
    let error = spec.emit(&go).unwrap_err();
    assert!(
        matches!(
            error,
            sigil_stitch::error::SigilStitchError::UnsupportedTypeKind {
                kind: TypeKind::Enum,
                ..
            }
        ),
        "{error}"
    );
}

#[test]
fn unsupported_spec_capability_fails_closed() {
    let go = sigil_stitch::lang::go::Go::new();
    let spec = TypeSpec::builder("Server", TypeKind::Struct)
        .add_method(
            FunSpec::builder("Start")
                .body(CodeBlock::of("return", ()).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    let error = spec.emit(&go).unwrap_err();
    assert!(
        matches!(
            error,
            sigil_stitch::error::SigilStitchError::UnsupportedTypeCapabilities {
                ref capabilities,
                ..
            } if capabilities.contains(&TypeCapability::Methods)
        ),
        "{error}"
    );
}
