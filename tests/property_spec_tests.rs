use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::javascript::JavaScript;
use sigil_stitch::lang::kotlin::Kotlin;
use sigil_stitch::lang::swift::Swift;
use sigil_stitch::lang::typescript::TypeScript;
use sigil_stitch::spec::modifiers::{DeclarationContext, TypeKind, Visibility};
use sigil_stitch::spec::property_spec::PropertySpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

// ── Helpers ──────────────────────────────────────────────

fn render_property<L: sigil_stitch::lang::CodeLang>(
    spec: &PropertySpec<L>,
    lang: &L,
    ctx: DeclarationContext,
) -> String {
    let blocks = spec.emit(lang, ctx).unwrap();
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

// ── TypeScript: Accessor style ──────────────────────────

#[test]
fn test_ts_getter_only() {
    let ts = TypeScript::new();
    let mut pb = PropertySpec::<TypeScript>::builder("count", TypeName::primitive("number"));
    pb.getter(CodeBlock::of("return this._count", ()).unwrap());
    let output = render_property(&pb.build().unwrap(), &ts, DeclarationContext::Member);
    assert!(output.contains("get count(): number {"));
    assert!(output.contains("return this._count"));
}

#[test]
fn test_ts_getter_setter() {
    let ts = TypeScript::new();
    let mut pb = PropertySpec::<TypeScript>::builder("name", TypeName::primitive("string"));
    pb.getter(CodeBlock::of("return this._name", ()).unwrap());
    pb.setter("value", CodeBlock::of("this._name = value", ()).unwrap());
    let output = render_property(&pb.build().unwrap(), &ts, DeclarationContext::Member);
    assert!(output.contains("get name(): string {"));
    assert!(output.contains("return this._name"));
    assert!(output.contains("set name(value: string) {"));
    assert!(output.contains("this._name = value"));
}

#[test]
fn test_ts_property_with_visibility() {
    let ts = TypeScript::new();
    let mut pb = PropertySpec::<TypeScript>::builder("age", TypeName::primitive("number"));
    pb.visibility(Visibility::Private);
    pb.getter(CodeBlock::of("return this._age", ()).unwrap());
    let output = render_property(&pb.build().unwrap(), &ts, DeclarationContext::Member);
    assert!(output.contains("private get age(): number {"));
}

#[test]
fn test_ts_property_in_class() {
    let ts = TypeScript::new();
    let mut tb = TypeSpec::<TypeScript>::builder("User", TypeKind::Class);
    tb.visibility(Visibility::Public);

    let mut pb = PropertySpec::builder("name", TypeName::primitive("string"));
    pb.getter(CodeBlock::of("return this._name", ()).unwrap());
    pb.setter("value", CodeBlock::of("this._name = value", ()).unwrap());
    tb.add_property(pb.build().unwrap());

    let output = render_type(&tb.build().unwrap(), &ts);
    assert!(output.contains("export class User {"));
    assert!(output.contains("get name(): string {"));
    assert!(output.contains("set name(value: string) {"));
}

// ── JavaScript: Accessor style (no types) ───────────────

#[test]
fn test_js_getter_setter() {
    let js = JavaScript::new();
    let mut pb = PropertySpec::<JavaScript>::builder("name", TypeName::primitive(""));
    pb.getter(CodeBlock::of("return this._name", ()).unwrap());
    pb.setter("value", CodeBlock::of("this._name = value", ()).unwrap());
    let output = render_property(&pb.build().unwrap(), &js, DeclarationContext::Member);
    assert!(output.contains("get name() {"));
    assert!(output.contains("set name(value) {"));
    // No type annotation.
    assert!(!output.contains(": string"));
}

// ── Swift: Field style ──────────────────────────────────

#[test]
fn test_swift_getter_only() {
    let swift = Swift::new();
    let mut pb = PropertySpec::<Swift>::builder("count", TypeName::primitive("Int"));
    pb.getter(CodeBlock::of("return _count", ()).unwrap());
    let output = render_property(&pb.build().unwrap(), &swift, DeclarationContext::Member);
    // Getter-only → readonly → "let" keyword.
    assert!(output.contains("let count: Int {"));
    assert!(output.contains("get {"));
    assert!(output.contains("return _count"));
}

#[test]
fn test_swift_getter_setter() {
    let swift = Swift::new();
    let mut pb = PropertySpec::<Swift>::builder("name", TypeName::primitive("String"));
    pb.getter(CodeBlock::of("return _name", ()).unwrap());
    pb.setter("newValue", CodeBlock::of("_name = newValue", ()).unwrap());
    let output = render_property(&pb.build().unwrap(), &swift, DeclarationContext::Member);
    // Getter+setter → mutable → "var" keyword.
    assert!(output.contains("var name: String {"));
    assert!(output.contains("get {"));
    assert!(output.contains("return _name"));
    assert!(output.contains("set(newValue) {"));
    assert!(output.contains("_name = newValue"));
}

#[test]
fn test_swift_property_with_visibility() {
    let swift = Swift::new();
    let mut pb = PropertySpec::<Swift>::builder("count", TypeName::primitive("Int"));
    pb.visibility(Visibility::Public);
    pb.getter(CodeBlock::of("return _count", ()).unwrap());
    let output = render_property(&pb.build().unwrap(), &swift, DeclarationContext::Member);
    assert!(output.contains("public let count: Int {"));
}

#[test]
fn test_swift_property_in_class() {
    let swift = Swift::new();
    let mut tb = TypeSpec::<Swift>::builder("Counter", TypeKind::Class);
    let mut pb = PropertySpec::builder("count", TypeName::primitive("Int"));
    pb.getter(CodeBlock::of("return _count", ()).unwrap());
    pb.setter("newValue", CodeBlock::of("_count = newValue", ()).unwrap());
    tb.add_property(pb.build().unwrap());
    let output = render_type(&tb.build().unwrap(), &swift);
    assert!(output.contains("class Counter {"));
    assert!(output.contains("var count: Int {"));
    assert!(output.contains("get {"));
    assert!(output.contains("set(newValue) {"));
}

// ── Kotlin: Field style ─────────────────────────────────

#[test]
fn test_kotlin_getter_only() {
    let kt = Kotlin::new();
    let mut pb = PropertySpec::<Kotlin>::builder("count", TypeName::primitive("Int"));
    pb.getter(CodeBlock::of("return _count", ()).unwrap());
    let output = render_property(&pb.build().unwrap(), &kt, DeclarationContext::Member);
    // Getter-only → readonly → "val" keyword.
    assert!(output.contains("val count: Int {"));
    assert!(output.contains("get() {"));
    assert!(output.contains("return _count"));
}

#[test]
fn test_kotlin_getter_setter() {
    let kt = Kotlin::new();
    let mut pb = PropertySpec::<Kotlin>::builder("name", TypeName::primitive("String"));
    pb.getter(CodeBlock::of("return field", ()).unwrap());
    pb.setter("value", CodeBlock::of("field = value", ()).unwrap());
    let output = render_property(&pb.build().unwrap(), &kt, DeclarationContext::Member);
    // Getter+setter → mutable → "var" keyword.
    assert!(output.contains("var name: String {"));
    assert!(output.contains("get() {"));
    assert!(output.contains("set(value) {"));
}

#[test]
fn test_kotlin_property_in_class() {
    let kt = Kotlin::new();
    let mut tb = TypeSpec::<Kotlin>::builder("Person", TypeKind::Class);
    let mut pb = PropertySpec::builder("name", TypeName::primitive("String"));
    pb.getter(CodeBlock::of("return field", ()).unwrap());
    tb.add_property(pb.build().unwrap());
    let output = render_type(&tb.build().unwrap(), &kt);
    assert!(output.contains("class Person {"));
    assert!(output.contains("val name: String {"));
    assert!(output.contains("get() {"));
}

// ── Doc comments ────────────────────────────────────────

#[test]
fn test_property_with_doc() {
    let ts = TypeScript::new();
    let mut pb = PropertySpec::<TypeScript>::builder("count", TypeName::primitive("number"));
    pb.doc("The current count.");
    pb.getter(CodeBlock::of("return this._count", ()).unwrap());
    let output = render_property(&pb.build().unwrap(), &ts, DeclarationContext::Member);
    assert!(output.contains("* The current count."));
    assert!(output.contains("get count(): number {"));
}

// ── Static property ─────────────────────────────────────

#[test]
fn test_ts_static_property() {
    let ts = TypeScript::new();
    let mut pb = PropertySpec::<TypeScript>::builder("instance", TypeName::primitive("App"));
    pb.is_static();
    pb.getter(CodeBlock::of("return App._instance", ()).unwrap());
    let output = render_property(&pb.build().unwrap(), &ts, DeclarationContext::Member);
    assert!(output.contains("static get instance(): App {"));
}
