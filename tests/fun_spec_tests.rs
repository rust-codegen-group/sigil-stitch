use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::code_renderer::CodeRenderer;
use sigil_stitch::import::ImportGroup;
use sigil_stitch::lang::rust_lang::RustLang;
use sigil_stitch::lang::typescript::TypeScript;
use sigil_stitch::spec::emittable::Emittable;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::{DeclarationContext, Visibility};
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::where_spec::{TypeParamKind, TypeParamSpec};
use sigil_stitch::type_name::TypeName;

fn emit_fun_ts(spec: &FunSpec, ctx: DeclarationContext) -> String {
    let lang = TypeScript::new();
    let block = spec.emit(&lang, ctx).unwrap();
    let imports = ImportGroup::new();
    let mut renderer = CodeRenderer::new(&lang, &imports, 80);
    renderer.render(&block).unwrap()
}

fn emit_fun_rs(spec: &FunSpec, ctx: DeclarationContext) -> String {
    let lang = RustLang::new();
    let block = spec.emit(&lang, ctx).unwrap();
    let imports = ImportGroup::new();
    let mut renderer = CodeRenderer::new(&lang, &imports, 80);
    renderer.render(&block).unwrap()
}

#[test]
fn test_ts_simple_function() {
    let body = CodeBlock::of("console.log(name)", ()).unwrap();
    let fun = FunSpec::builder("greet")
        .add_param(ParameterSpec::new("name", TypeName::primitive("string")).unwrap())
        .returns(TypeName::primitive("void"))
        .body(body)
        .build()
        .unwrap();
    let output = emit_fun_ts(&fun, DeclarationContext::TopLevel);
    assert!(output.contains("function greet(name: string): void {"));
    assert!(output.contains("console.log(name)"));
    assert!(output.contains("}"));
}

#[test]
fn test_ts_async_method() {
    let body = CodeBlock::of("return db.find(id)", ()).unwrap();
    let fun = FunSpec::builder("getUser")
        .add_param(ParameterSpec::new("id", TypeName::primitive("string")).unwrap())
        .returns(TypeName::generic(
            TypeName::primitive("Promise"),
            vec![TypeName::primitive("User")],
        ))
        .is_async()
        .visibility(Visibility::Public)
        .body(body)
        .build()
        .unwrap();
    let output = emit_fun_ts(&fun, DeclarationContext::Member);
    assert!(output.contains("public async getUser(id: string): Promise<User> {"));
}

#[test]
fn test_ts_abstract_method() {
    let fun = FunSpec::builder("validate")
        .is_abstract()
        .returns(TypeName::primitive("boolean"))
        .build()
        .unwrap();
    let output = emit_fun_ts(&fun, DeclarationContext::Member);
    assert!(output.contains("abstract validate(): boolean;"));
}

#[test]
fn test_rust_simple_function() {
    let body = CodeBlock::of("a + b", ()).unwrap();
    let fun = FunSpec::builder("add")
        .visibility(Visibility::Public)
        .add_param(ParameterSpec::new("a", TypeName::primitive("i32")).unwrap())
        .add_param(ParameterSpec::new("b", TypeName::primitive("i32")).unwrap())
        .returns(TypeName::primitive("i32"))
        .body(body)
        .build()
        .unwrap();
    let output = emit_fun_rs(&fun, DeclarationContext::TopLevel);
    assert!(output.contains("pub fn add(a: i32, b: i32) -> i32 {"));
    assert!(output.contains("a + b"));
}

#[test]
fn test_fun_with_type_params() {
    let tp = TypeParamSpec::new("T").with_bound(TypeName::primitive("Serializable"));
    let body = CodeBlock::of("return JSON.stringify(value)", ()).unwrap();
    let fun = FunSpec::builder("serialize")
        .add_type_param(tp)
        .add_param(ParameterSpec::new("value", TypeName::primitive("T")).unwrap())
        .returns(TypeName::primitive("string"))
        .body(body)
        .build()
        .unwrap();
    let output = emit_fun_ts(&fun, DeclarationContext::TopLevel);
    assert!(output.contains("function serialize<T extends Serializable>(value: T): string {"));
}

#[test]
fn test_build_empty_name_errors() {
    let result = FunSpec::builder("").build();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("'name' must not be empty")
    );
}

#[test]
fn test_where_clause_rust_function() {
    let fun = FunSpec::builder("process")
        .add_type_param(TypeParamSpec::new("T"))
        .add_type_param(TypeParamSpec::new("U"))
        .add_where_constraint(
            TypeName::primitive("T"),
            vec![TypeName::primitive("Clone"), TypeName::primitive("Send")],
        )
        .add_where_constraint(TypeName::primitive("U"), vec![TypeName::primitive("Debug")])
        .add_param(ParameterSpec::new("a", TypeName::primitive("T")).unwrap())
        .add_param(ParameterSpec::new("b", TypeName::primitive("U")).unwrap())
        .body(CodeBlock::of("todo!()", ()).unwrap())
        .build()
        .unwrap();
    let output = emit_fun_rs(&fun, DeclarationContext::TopLevel);
    assert!(
        output.contains("fn process<T, U>(a: T, b: U)"),
        "sig: {output}"
    );
    assert!(
        output.contains("where\n    T: Clone + Send,\n    U: Debug,"),
        "where: {output}"
    );
    assert!(
        output.contains("U: Debug,\n{"),
        "block_open on new line after where: {output}"
    );
}

#[test]
fn test_where_clause_ts_inline_ignored() {
    let fun = FunSpec::builder("process")
        .add_type_param(TypeParamSpec::new("T"))
        .add_where_constraint(
            TypeName::primitive("T"),
            vec![TypeName::primitive("Serializable")],
        )
        .body(CodeBlock::of("return value", ()).unwrap())
        .build()
        .unwrap();
    let output = emit_fun_ts(&fun, DeclarationContext::TopLevel);
    assert!(
        !output.contains("where"),
        "TS should not emit where: {output}"
    );
    assert!(output.contains("function process<T>("), "sig: {output}");
}

#[test]
fn test_where_bound_convenience() {
    let fun = FunSpec::builder("example")
        .add_type_param(TypeParamSpec::new("T"))
        .where_bound("T", TypeName::primitive("Clone"))
        .where_bound("T", TypeName::primitive("Send"))
        .body(CodeBlock::of("todo!()", ()).unwrap())
        .build()
        .unwrap();
    let output = emit_fun_rs(&fun, DeclarationContext::TopLevel);
    assert!(
        output.contains("where\n    T: Clone + Send,"),
        "where: {output}"
    );
}

#[test]
fn test_type_param_kind_none_unchanged() {
    let fun = FunSpec::builder("foo")
        .add_type_param(TypeParamSpec::new("T"))
        .body(CodeBlock::of("return null", ()).unwrap())
        .build()
        .unwrap();
    let output = emit_fun_ts(&fun, DeclarationContext::TopLevel);
    assert!(output.contains("function foo<T>()"), "unchanged: {output}");
}

#[test]
fn test_type_param_with_kind_default_no_output() {
    let fun = FunSpec::builder("apply")
        .add_type_param(TypeParamSpec::new("F").with_kind(TypeParamKind::Constructor1))
        .body(CodeBlock::of("todo!()", ()).unwrap())
        .build()
        .unwrap();
    let output = emit_fun_rs(&fun, DeclarationContext::TopLevel);
    assert!(
        output.contains("fn apply<F>()"),
        "default renders no kind suffix: {output}"
    );
}

#[test]
fn test_lifetime_params_before_type_params() {
    let fun = FunSpec::builder("longest")
        .add_type_param(TypeParamSpec::new("T"))
        .add_type_param(TypeParamSpec::lifetime("'a"))
        .add_param(
            ParameterSpec::new(
                "x",
                TypeName::reference_with_lifetime(TypeName::primitive("str"), "'a"),
            )
            .unwrap(),
        )
        .add_param(
            ParameterSpec::new(
                "y",
                TypeName::reference_with_lifetime(TypeName::primitive("str"), "'a"),
            )
            .unwrap(),
        )
        .returns(TypeName::reference_with_lifetime(
            TypeName::primitive("str"),
            "'a",
        ))
        .body(CodeBlock::of("x", ()).unwrap())
        .build()
        .unwrap();
    let output = emit_fun_rs(&fun, DeclarationContext::TopLevel);
    assert!(
        output.contains("fn longest<'a, T>("),
        "lifetime first: {output}"
    );
    assert!(
        output.contains("x: &'a str"),
        "lifetime ref param: {output}"
    );
    assert!(
        output.contains("-> &'a str"),
        "lifetime ref return: {output}"
    );
}

#[test]
fn test_emittable_delegates_to_emit() {
    let f = FunSpec::builder("greet").build().unwrap();
    let lang = TypeScript::new();
    let blocks = f.emit_members(&lang).unwrap();
    assert_eq!(blocks.len(), 1);
}

#[test]
fn test_emittable_uses_top_level_context() {
    let f = FunSpec::builder("greet")
        .visibility(Visibility::Public)
        .build()
        .unwrap();
    let lang = TypeScript::new();
    let blocks = f.emit_members(&lang).unwrap();
    let imports = ImportGroup::new();
    let mut renderer = CodeRenderer::new(&lang, &imports, 80);
    let output = renderer.render(&blocks[0]).unwrap();
    assert!(
        output.contains("export"),
        "TopLevel public function should have 'export': {output}"
    );
}
