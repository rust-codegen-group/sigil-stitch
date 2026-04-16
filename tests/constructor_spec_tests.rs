use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::cpp_lang::CppLang;
use sigil_stitch::lang::dart::DartLang;
use sigil_stitch::lang::java_lang::JavaLang;
use sigil_stitch::lang::javascript::JavaScript;
use sigil_stitch::lang::kotlin::Kotlin;
use sigil_stitch::lang::python::Python;
use sigil_stitch::lang::rust_lang::RustLang;
use sigil_stitch::lang::swift::Swift;
use sigil_stitch::lang::typescript::TypeScript;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::{DeclarationContext, TypeKind, Visibility};
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

// ── Helpers ──────────────────────────────────────────────

fn render_fun<L: sigil_stitch::lang::CodeLang>(
    spec: &FunSpec<L>,
    lang: &L,
    ctx: DeclarationContext,
) -> String {
    let block = spec.emit(lang, ctx).unwrap();
    let imports = sigil_stitch::import::ImportGroup::new();
    let mut renderer = sigil_stitch::code_renderer::CodeRenderer::new(lang, &imports, 80);
    renderer.render(&block).unwrap()
}

fn render_type<L: sigil_stitch::lang::CodeLang>(spec: &TypeSpec<L>, lang: &L) -> String {
    let blocks = spec.emit(lang).unwrap();
    let imports = sigil_stitch::import::ImportGroup::new();
    let mut output = String::new();
    for (i, block) in blocks.iter().enumerate() {
        if i > 0 {
            output.push('\n');
        }
        let mut renderer = sigil_stitch::code_renderer::CodeRenderer::new(lang, &imports, 80);
        output.push_str(&renderer.render(block).unwrap());
    }
    output
}

// ── TypeScript ───────────────────────────────────────────

#[test]
fn test_ts_constructor() {
    let ts = TypeScript::new();
    let mut fb = FunSpec::<TypeScript>::builder("constructor");
    fb.is_constructor();
    fb.add_param(ParameterSpec::new("name", TypeName::primitive("string")).unwrap());
    fb.body(CodeBlock::of("this.name = name", ()).unwrap());
    let output = render_fun(&fb.build().unwrap(), &ts, DeclarationContext::Member);
    assert!(output.contains("constructor(name: string) {"));
    assert!(output.contains("this.name = name"));
    // Must NOT have "function" keyword.
    assert!(!output.contains("function"));
}

#[test]
fn test_ts_constructor_in_class() {
    let ts = TypeScript::new();
    let mut tb = TypeSpec::<TypeScript>::builder("User", TypeKind::Class);
    tb.visibility(Visibility::Public);
    let mut ctor = FunSpec::builder("constructor");
    ctor.is_constructor();
    ctor.add_param(ParameterSpec::new("name", TypeName::primitive("string")).unwrap());
    ctor.body(CodeBlock::of("this.name = name", ()).unwrap());
    tb.add_method(ctor.build().unwrap());
    let output = render_type(&tb.build().unwrap(), &ts);
    assert!(output.contains("export class User {"));
    assert!(output.contains("constructor(name: string) {"));
}

// ── JavaScript ───────────────────────────────────────────

#[test]
fn test_js_constructor() {
    let js = JavaScript::new();
    let mut fb = FunSpec::<JavaScript>::builder("constructor");
    fb.is_constructor();
    fb.add_param(ParameterSpec::new("name", TypeName::primitive("")).unwrap());
    fb.body(CodeBlock::of("this.name = name", ()).unwrap());
    let output = render_fun(&fb.build().unwrap(), &js, DeclarationContext::Member);
    assert!(output.contains("constructor(name) {"));
    assert!(!output.contains("function"));
}

// ── Java ─────────────────────────────────────────────────

#[test]
fn test_java_constructor() {
    let java = JavaLang::new();
    let mut fb = FunSpec::<JavaLang>::builder("UserService");
    fb.is_constructor();
    fb.visibility(Visibility::Public);
    fb.add_param(ParameterSpec::new("repo", TypeName::primitive("UserRepository")).unwrap());
    fb.body(CodeBlock::of("this.repo = repo;", ()).unwrap());
    let output = render_fun(&fb.build().unwrap(), &java, DeclarationContext::Member);
    assert!(output.contains("public UserService(UserRepository repo) {"));
    // No return type.
    assert!(!output.contains("void"));
}

// ── C++ ──────────────────────────────────────────────────

#[test]
fn test_cpp_constructor() {
    let cpp = CppLang::new();
    let mut fb = FunSpec::<CppLang>::builder("Counter");
    fb.is_constructor();
    fb.body(CodeBlock::of("count_ = 0;", ()).unwrap());
    let output = render_fun(&fb.build().unwrap(), &cpp, DeclarationContext::Member);
    assert!(output.contains("Counter() {"));
    // No return type prefix.
    assert!(!output.contains("void"));
}

// ── Dart ─────────────────────────────────────────────────

#[test]
fn test_dart_constructor() {
    let dart = DartLang::new();
    let mut fb = FunSpec::<DartLang>::builder("Task");
    fb.is_constructor();
    fb.add_param(ParameterSpec::new("title", TypeName::primitive("String")).unwrap());
    fb.body(CodeBlock::of("this.title = title;", ()).unwrap());
    let output = render_fun(&fb.build().unwrap(), &dart, DeclarationContext::Member);
    assert!(output.contains("Task(String title) {"));
}

// ── Swift ────────────────────────────────────────────────

#[test]
fn test_swift_constructor() {
    let swift = Swift::new();
    let mut fb = FunSpec::<Swift>::builder("init");
    fb.is_constructor();
    fb.add_param(ParameterSpec::new("name", TypeName::primitive("String")).unwrap());
    fb.body(CodeBlock::of("self.name = name", ()).unwrap());
    let output = render_fun(&fb.build().unwrap(), &swift, DeclarationContext::Member);
    // Must be `init(name: String)` NOT `func init(name: String)`.
    assert!(output.contains("init(name: String) {"));
    assert!(!output.contains("func init"));
}

#[test]
fn test_swift_constructor_in_class() {
    let swift = Swift::new();
    let mut tb = TypeSpec::<Swift>::builder("Person", TypeKind::Class);
    let mut ctor = FunSpec::builder("init");
    ctor.is_constructor();
    ctor.add_param(ParameterSpec::new("name", TypeName::primitive("String")).unwrap());
    ctor.body(CodeBlock::of("self.name = name", ()).unwrap());
    tb.add_method(ctor.build().unwrap());
    let output = render_type(&tb.build().unwrap(), &swift);
    assert!(output.contains("class Person {"));
    assert!(output.contains("init(name: String) {"));
    assert!(!output.contains("func init"));
}

// ── Kotlin ───────────────────────────────────────────────

#[test]
fn test_kotlin_secondary_constructor() {
    let kt = Kotlin::new();
    let mut fb = FunSpec::<Kotlin>::builder("constructor");
    fb.is_constructor();
    fb.add_param(ParameterSpec::new("name", TypeName::primitive("String")).unwrap());
    fb.body(CodeBlock::of("this.name = name", ()).unwrap());
    let output = render_fun(&fb.build().unwrap(), &kt, DeclarationContext::Member);
    // Must be `constructor(name: String)` NOT `fun constructor(name: String)`.
    assert!(output.contains("constructor(name: String)"));
    assert!(!output.contains("fun constructor"));
}

#[test]
fn test_kotlin_constructor_in_class() {
    let kt = Kotlin::new();
    let mut tb = TypeSpec::<Kotlin>::builder("Person", TypeKind::Class);
    let mut ctor = FunSpec::builder("constructor");
    ctor.is_constructor();
    ctor.add_param(ParameterSpec::new("name", TypeName::primitive("String")).unwrap());
    ctor.body(CodeBlock::of("this.name = name", ()).unwrap());
    tb.add_method(ctor.build().unwrap());
    let output = render_type(&tb.build().unwrap(), &kt);
    assert!(output.contains("class Person {"));
    assert!(output.contains("constructor(name: String)"));
    assert!(!output.contains("fun constructor"));
}

// ── Python ───────────────────────────────────────────────

#[test]
fn test_python_constructor() {
    let py = Python::new();
    let mut fb = FunSpec::<Python>::builder("__init__");
    fb.is_constructor();
    fb.add_param(ParameterSpec::new("self", TypeName::primitive("")).unwrap());
    fb.add_param(ParameterSpec::new("name", TypeName::primitive("str")).unwrap());
    fb.body(CodeBlock::of("self.name = name", ()).unwrap());
    let output = render_fun(&fb.build().unwrap(), &py, DeclarationContext::Member);
    // Must keep "def" keyword: `def __init__(self, name: str):`
    assert!(output.contains("def __init__(self, name: str):"));
}

// ── Rust ─────────────────────────────────────────────────

#[test]
fn test_rust_constructor() {
    let rs = RustLang::new();
    let mut fb = FunSpec::<RustLang>::builder("new");
    fb.is_constructor();
    fb.visibility(Visibility::Public);
    fb.add_param(ParameterSpec::new("name", TypeName::primitive("&str")).unwrap());
    fb.returns(TypeName::primitive("Self"));
    fb.body(CodeBlock::of("Self { name: name.to_string() }", ()).unwrap());
    let output = render_fun(&fb.build().unwrap(), &rs, DeclarationContext::TopLevel);
    // Must keep "fn" keyword: `pub fn new(name: &str) -> Self {`
    assert!(output.contains("pub fn new(name: &str) -> Self {"));
}

// ── Super delegation ─────────────────────────────────────

#[test]
fn test_js_constructor_with_super() {
    let js = JavaScript::new();
    let mut fb = FunSpec::<JavaScript>::builder("constructor");
    fb.is_constructor();
    fb.add_param(ParameterSpec::new("name", TypeName::primitive("")).unwrap());
    fb.add_param(ParameterSpec::new("breed", TypeName::primitive("")).unwrap());
    fb.body(CodeBlock::of("super(name);\nthis.breed = breed;", ()).unwrap());
    let output = render_fun(&fb.build().unwrap(), &js, DeclarationContext::Member);
    assert!(output.contains("constructor(name, breed) {"));
    assert!(output.contains("super(name);"));
    assert!(output.contains("this.breed = breed;"));
}

#[test]
fn test_java_constructor_with_super() {
    let java = JavaLang::new();
    let mut fb = FunSpec::<JavaLang>::builder("Dog");
    fb.is_constructor();
    fb.visibility(Visibility::Public);
    fb.add_param(ParameterSpec::new("name", TypeName::primitive("String")).unwrap());
    fb.body(CodeBlock::of("super(name);", ()).unwrap());
    let output = render_fun(&fb.build().unwrap(), &java, DeclarationContext::Member);
    assert!(output.contains("public Dog(String name) {"));
    assert!(output.contains("super(name);"));
}

// ── Backward compatibility ───────────────────────────────

#[test]
fn test_backward_compat_ts_constructor_without_flag() {
    // Existing pattern: FunSpec with name "constructor" and no is_constructor flag
    // should still work because TS function_keyword(Member) already returns "".
    let ts = TypeScript::new();
    let mut fb = FunSpec::<TypeScript>::builder("constructor");
    fb.add_param(ParameterSpec::new("name", TypeName::primitive("string")).unwrap());
    fb.body(CodeBlock::of("this.name = name", ()).unwrap());
    let output = render_fun(&fb.build().unwrap(), &ts, DeclarationContext::Member);
    assert!(output.contains("constructor(name: string) {"));
}

#[test]
fn test_backward_compat_java_constructor_without_flag() {
    // Existing pattern: FunSpec with class name and no is_constructor flag.
    let java = JavaLang::new();
    let mut fb = FunSpec::<JavaLang>::builder("UserService");
    fb.visibility(Visibility::Public);
    fb.add_param(ParameterSpec::new("repo", TypeName::primitive("UserRepository")).unwrap());
    fb.body(CodeBlock::of("this.repo = repo;", ()).unwrap());
    let output = render_fun(&fb.build().unwrap(), &java, DeclarationContext::Member);
    assert!(output.contains("public UserService(UserRepository repo) {"));
}

// ── Super/this delegation ───────────────────────────────

#[test]
fn test_ts_constructor_with_super_delegation() {
    let ts = TypeScript::new();
    let mut fb = FunSpec::<TypeScript>::builder("constructor");
    fb.is_constructor();
    fb.add_param(ParameterSpec::new("name", TypeName::primitive("string")).unwrap());
    fb.add_param(ParameterSpec::new("age", TypeName::primitive("number")).unwrap());
    fb.delegation(CodeBlock::of("super(name)", ()).unwrap());
    fb.body(CodeBlock::of("this.age = age", ()).unwrap());
    let output = render_fun(&fb.build().unwrap(), &ts, DeclarationContext::Member);
    assert!(output.contains("constructor(name: string, age: number) {"));
    assert!(output.contains("super(name);"));
    assert!(output.contains("this.age = age"));
    // super should appear before the body
    let super_pos = output.find("super(name);").unwrap();
    let body_pos = output.find("this.age = age").unwrap();
    assert!(super_pos < body_pos);
}

#[test]
fn test_ts_constructor_with_this_delegation() {
    let ts = TypeScript::new();
    let mut fb = FunSpec::<TypeScript>::builder("constructor");
    fb.is_constructor();
    fb.add_param(ParameterSpec::new("name", TypeName::primitive("string")).unwrap());
    fb.delegation(CodeBlock::of("this(name, 0)", ()).unwrap());
    fb.body(CodeBlock::of("// additional init", ()).unwrap());
    let output = render_fun(&fb.build().unwrap(), &ts, DeclarationContext::Member);
    assert!(output.contains("this(name, 0);"));
}

#[test]
fn test_java_constructor_with_super_delegation() {
    let java = JavaLang::new();
    let mut fb = FunSpec::<JavaLang>::builder("Dog");
    fb.is_constructor();
    fb.visibility(Visibility::Public);
    fb.add_param(ParameterSpec::new("name", TypeName::primitive("String")).unwrap());
    fb.add_param(ParameterSpec::new("breed", TypeName::primitive("String")).unwrap());
    fb.delegation(CodeBlock::of("super(name)", ()).unwrap());
    fb.body(CodeBlock::of("this.breed = breed;", ()).unwrap());
    let output = render_fun(&fb.build().unwrap(), &java, DeclarationContext::Member);
    assert!(output.contains("public Dog(String name, String breed) {"));
    assert!(output.contains("super(name);"));
    assert!(output.contains("this.breed = breed;"));
    let super_pos = output.find("super(name);").unwrap();
    let body_pos = output.find("this.breed = breed;").unwrap();
    assert!(super_pos < body_pos);
}

#[test]
fn test_java_constructor_with_this_delegation() {
    let java = JavaLang::new();
    let mut fb = FunSpec::<JavaLang>::builder("Config");
    fb.is_constructor();
    fb.visibility(Visibility::Public);
    fb.add_param(ParameterSpec::new("name", TypeName::primitive("String")).unwrap());
    fb.delegation(CodeBlock::of("this(name, \"default\")", ()).unwrap());
    fb.body(CodeBlock::of("// chained", ()).unwrap());
    let output = render_fun(&fb.build().unwrap(), &java, DeclarationContext::Member);
    assert!(output.contains("this(name, \"default\");"));
}

#[test]
fn test_kotlin_constructor_with_super_delegation_signature_style() {
    let kt = Kotlin::new();
    let mut fb = FunSpec::<Kotlin>::builder("constructor");
    fb.is_constructor();
    fb.add_param(ParameterSpec::new("name", TypeName::primitive("String")).unwrap());
    fb.delegation(CodeBlock::of("super(name)", ()).unwrap());
    fb.body(CodeBlock::of("this.name = name", ()).unwrap());
    let output = render_fun(&fb.build().unwrap(), &kt, DeclarationContext::Member);
    // Kotlin places delegation in the signature, not the body
    assert!(output.contains("constructor(name: String) : super(name) {"));
    assert!(output.contains("this.name = name"));
    // The delegation should NOT appear as a statement in the body
    let body_start = output.find('{').unwrap();
    let body_content = &output[body_start..];
    assert!(
        !body_content.contains("super(name)\n    this"),
        "super() should be in signature, not body"
    );
}

#[test]
fn test_kotlin_constructor_with_this_delegation_signature_style() {
    let kt = Kotlin::new();
    let mut fb = FunSpec::<Kotlin>::builder("constructor");
    fb.is_constructor();
    fb.add_param(ParameterSpec::new("name", TypeName::primitive("String")).unwrap());
    fb.delegation(CodeBlock::of("this(name, 0)", ()).unwrap());
    fb.body(CodeBlock::of("// chained", ()).unwrap());
    let output = render_fun(&fb.build().unwrap(), &kt, DeclarationContext::Member);
    assert!(output.contains("constructor(name: String) : this(name, 0) {"));
}

#[test]
fn test_swift_constructor_with_super_delegation() {
    let swift = Swift::new();
    let mut fb = FunSpec::<Swift>::builder("init");
    fb.is_constructor();
    fb.add_param(ParameterSpec::new("name", TypeName::primitive("String")).unwrap());
    fb.delegation(CodeBlock::of("super.init()", ()).unwrap());
    fb.body(CodeBlock::of("self.name = name", ()).unwrap());
    let output = render_fun(&fb.build().unwrap(), &swift, DeclarationContext::Member);
    assert!(output.contains("init(name: String) {"));
    assert!(output.contains("super.init()"));
    assert!(output.contains("self.name = name"));
}

#[test]
fn test_dart_constructor_with_super_delegation() {
    let dart = DartLang::new();
    let mut fb = FunSpec::<DartLang>::builder("Dog");
    fb.is_constructor();
    fb.add_param(ParameterSpec::new("name", TypeName::primitive("String")).unwrap());
    fb.delegation(CodeBlock::of("super(name)", ()).unwrap());
    fb.body(CodeBlock::of("print('Dog created');", ()).unwrap());
    let output = render_fun(&fb.build().unwrap(), &dart, DeclarationContext::Member);
    assert!(output.contains("Dog(String name) {"));
    assert!(output.contains("super(name);"));
    assert!(output.contains("print('Dog created');"));
}

#[test]
fn test_python_constructor_with_super_delegation() {
    let py = Python::new();
    let mut fb = FunSpec::<Python>::builder("__init__");
    fb.is_constructor();
    fb.add_param(ParameterSpec::new("self", TypeName::primitive("")).unwrap());
    fb.add_param(ParameterSpec::new("name", TypeName::primitive("str")).unwrap());
    fb.delegation(CodeBlock::of("super().__init__(name)", ()).unwrap());
    fb.body(CodeBlock::of("self.extra = True", ()).unwrap());
    let output = render_fun(&fb.build().unwrap(), &py, DeclarationContext::Member);
    assert!(output.contains("def __init__(self, name: str):"));
    assert!(output.contains("super().__init__(name)"));
    assert!(output.contains("self.extra = True"));
}

#[test]
fn test_cpp_constructor_with_super_delegation() {
    let cpp = CppLang::new();
    let mut fb = FunSpec::<CppLang>::builder("Dog");
    fb.is_constructor();
    fb.add_param(ParameterSpec::new("name", TypeName::primitive("std::string")).unwrap());
    fb.delegation(CodeBlock::of("Animal(name)", ()).unwrap());
    fb.body(CodeBlock::of("// extra init", ()).unwrap());
    let output = render_fun(&fb.build().unwrap(), &cpp, DeclarationContext::Member);
    // C++ uses body-style delegation (in this simplified model)
    assert!(output.contains("Dog(std::string name) {"));
    assert!(output.contains("Animal(name);"));
}

// ── Delegation in class context ─────────────────────────

#[test]
fn test_ts_class_with_super_constructor_delegation() {
    let ts = TypeScript::new();
    let mut tb = TypeSpec::<TypeScript>::builder("Dog", TypeKind::Class);
    tb.visibility(Visibility::Public);
    tb.extends(TypeName::primitive("Animal"));

    let mut ctor = FunSpec::builder("constructor");
    ctor.is_constructor();
    ctor.add_param(ParameterSpec::new("name", TypeName::primitive("string")).unwrap());
    ctor.add_param(ParameterSpec::new("breed", TypeName::primitive("string")).unwrap());
    ctor.delegation(CodeBlock::of("super(name)", ()).unwrap());
    ctor.body(CodeBlock::of("this.breed = breed", ()).unwrap());
    tb.add_method(ctor.build().unwrap());

    let output = render_type(&tb.build().unwrap(), &ts);
    assert!(output.contains("export class Dog extends Animal {"));
    assert!(output.contains("constructor(name: string, breed: string) {"));
    assert!(output.contains("super(name);"));
    assert!(output.contains("this.breed = breed"));
}

#[test]
fn test_kotlin_class_with_super_constructor_delegation() {
    let kt = Kotlin::new();
    let mut tb = TypeSpec::<Kotlin>::builder("Dog", TypeKind::Class);
    tb.extends(TypeName::primitive("Animal"));

    let mut ctor = FunSpec::builder("constructor");
    ctor.is_constructor();
    ctor.add_param(ParameterSpec::new("name", TypeName::primitive("String")).unwrap());
    ctor.delegation(CodeBlock::of("super(name)", ()).unwrap());
    ctor.body(CodeBlock::of("this.name = name", ()).unwrap());
    tb.add_method(ctor.build().unwrap());

    let output = render_type(&tb.build().unwrap(), &kt);
    assert!(output.contains("class Dog : Animal {"));
    assert!(output.contains("constructor(name: String) : super(name) {"));
}

// ── Primary constructor (Kotlin) ────────────────────────

#[test]
fn test_kotlin_primary_constructor() {
    let kt = Kotlin::new();
    let mut tb = TypeSpec::<Kotlin>::builder("Person", TypeKind::Class);
    tb.add_primary_constructor_param(
        ParameterSpec::new("val name", TypeName::primitive("String")).unwrap(),
    );
    tb.add_primary_constructor_param(
        ParameterSpec::new("val age", TypeName::primitive("Int")).unwrap(),
    );

    let output = render_type(&tb.build().unwrap(), &kt);
    assert!(output.contains("class Person(val name: String, val age: Int) {"));
}

#[test]
fn test_kotlin_primary_constructor_with_super() {
    let kt = Kotlin::new();
    let mut tb = TypeSpec::<Kotlin>::builder("Student", TypeKind::Class);
    tb.add_primary_constructor_param(
        ParameterSpec::new("val name", TypeName::primitive("String")).unwrap(),
    );
    tb.add_primary_constructor_param(
        ParameterSpec::new("val grade", TypeName::primitive("Int")).unwrap(),
    );
    tb.extends(TypeName::primitive("Person"));

    let output = render_type(&tb.build().unwrap(), &kt);
    assert!(output.contains("class Student(val name: String, val grade: Int) : Person {"));
}

#[test]
fn test_kotlin_data_class_primary_constructor() {
    let kt = Kotlin::new();
    let mut tb = TypeSpec::<Kotlin>::builder("Point", TypeKind::Struct);
    tb.add_primary_constructor_param(
        ParameterSpec::new("val x", TypeName::primitive("Int")).unwrap(),
    );
    tb.add_primary_constructor_param(
        ParameterSpec::new("val y", TypeName::primitive("Int")).unwrap(),
    );

    let output = render_type(&tb.build().unwrap(), &kt);
    assert!(output.contains("data class Point(val x: Int, val y: Int) {"));
}

#[test]
fn test_kotlin_primary_constructor_with_secondary() {
    let kt = Kotlin::new();
    let mut tb = TypeSpec::<Kotlin>::builder("Config", TypeKind::Class);
    tb.add_primary_constructor_param(
        ParameterSpec::new("val name", TypeName::primitive("String")).unwrap(),
    );
    tb.add_primary_constructor_param(
        ParameterSpec::new("val value", TypeName::primitive("Int")).unwrap(),
    );

    // Secondary constructor delegates to primary
    let mut ctor = FunSpec::builder("constructor");
    ctor.is_constructor();
    ctor.add_param(ParameterSpec::new("name", TypeName::primitive("String")).unwrap());
    ctor.delegation(CodeBlock::of("this(name, 0)", ()).unwrap());
    ctor.body(CodeBlock::of("// secondary", ()).unwrap());
    tb.add_method(ctor.build().unwrap());

    let output = render_type(&tb.build().unwrap(), &kt);
    assert!(output.contains("class Config(val name: String, val value: Int) {"));
    assert!(output.contains("constructor(name: String) : this(name, 0) {"));
}

// ── Primary constructor ignored for non-supporting languages ──

#[test]
fn test_ts_ignores_primary_constructor() {
    let ts = TypeScript::new();
    let mut tb = TypeSpec::<TypeScript>::builder("Foo", TypeKind::Class);
    tb.add_primary_constructor_param(
        ParameterSpec::new("x", TypeName::primitive("number")).unwrap(),
    );

    let output = render_type(&tb.build().unwrap(), &ts);
    // TypeScript doesn't support primary constructors — params should be ignored
    assert!(output.contains("class Foo {"));
    assert!(!output.contains("(x: number)"));
}
