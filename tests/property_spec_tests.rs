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

fn render_property(
    spec: &PropertySpec,
    lang: &dyn sigil_stitch::lang::CodeLang,
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

fn render_type(spec: &TypeSpec, lang: &dyn sigil_stitch::lang::CodeLang) -> String {
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
    let output = render_property(
        &PropertySpec::builder("count", TypeName::primitive("number"))
            .getter(CodeBlock::of("return this._count", ()).unwrap())
            .build()
            .unwrap(),
        &ts,
        DeclarationContext::Member,
    );
    assert!(output.contains("get count(): number {"));
    assert!(output.contains("return this._count"));
}

#[test]
fn test_ts_getter_setter() {
    let ts = TypeScript::new();
    let output = render_property(
        &PropertySpec::builder("name", TypeName::primitive("string"))
            .getter(CodeBlock::of("return this._name", ()).unwrap())
            .setter("value", CodeBlock::of("this._name = value", ()).unwrap())
            .build()
            .unwrap(),
        &ts,
        DeclarationContext::Member,
    );
    assert!(output.contains("get name(): string {"));
    assert!(output.contains("return this._name"));
    assert!(output.contains("set name(value: string) {"));
    assert!(output.contains("this._name = value"));
}

#[test]
fn test_ts_property_with_visibility() {
    let ts = TypeScript::new();
    let output = render_property(
        &PropertySpec::builder("age", TypeName::primitive("number"))
            .visibility(Visibility::Private)
            .getter(CodeBlock::of("return this._age", ()).unwrap())
            .build()
            .unwrap(),
        &ts,
        DeclarationContext::Member,
    );
    assert!(output.contains("private get age(): number {"));
}

#[test]
fn test_ts_property_in_class() {
    let ts = TypeScript::new();
    let output = render_type(
        &TypeSpec::builder("User", TypeKind::Class)
            .visibility(Visibility::Public)
            .add_property(
                PropertySpec::builder("name", TypeName::primitive("string"))
                    .getter(CodeBlock::of("return this._name", ()).unwrap())
                    .setter("value", CodeBlock::of("this._name = value", ()).unwrap())
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap(),
        &ts,
    );
    assert!(output.contains("export class User {"));
    assert!(output.contains("get name(): string {"));
    assert!(output.contains("set name(value: string) {"));
}

// ── JavaScript: Accessor style (no types) ───────────────

#[test]
fn test_js_getter_setter() {
    let js = JavaScript::new();
    let output = render_property(
        &PropertySpec::builder("name", TypeName::primitive(""))
            .getter(CodeBlock::of("return this._name", ()).unwrap())
            .setter("value", CodeBlock::of("this._name = value", ()).unwrap())
            .build()
            .unwrap(),
        &js,
        DeclarationContext::Member,
    );
    assert!(output.contains("get name() {"));
    assert!(output.contains("set name(value) {"));
    // No type annotation.
    assert!(!output.contains(": string"));
}

// ── Swift: Field style ──────────────────────────────────

#[test]
fn test_swift_getter_only() {
    let swift = Swift::new();
    let output = render_property(
        &PropertySpec::builder("count", TypeName::primitive("Int"))
            .getter(CodeBlock::of("return _count", ()).unwrap())
            .build()
            .unwrap(),
        &swift,
        DeclarationContext::Member,
    );
    // Swift computed properties are always declared with `var`, even when
    // they expose only a getter.
    assert!(output.contains("var count: Int {"));
    assert!(output.contains("get {"));
    assert!(output.contains("return _count"));
}

#[test]
fn test_swift_getter_setter() {
    let swift = Swift::new();
    let output = render_property(
        &PropertySpec::builder("name", TypeName::primitive("String"))
            .getter(CodeBlock::of("return _name", ()).unwrap())
            .setter("newValue", CodeBlock::of("_name = newValue", ()).unwrap())
            .build()
            .unwrap(),
        &swift,
        DeclarationContext::Member,
    );
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
    let output = render_property(
        &PropertySpec::builder("count", TypeName::primitive("Int"))
            .visibility(Visibility::Public)
            .getter(CodeBlock::of("return _count", ()).unwrap())
            .build()
            .unwrap(),
        &swift,
        DeclarationContext::Member,
    );
    assert!(output.contains("public var count: Int {"));
}

#[test]
fn test_swift_property_in_class() {
    let swift = Swift::new();
    let output = render_type(
        &TypeSpec::builder("Counter", TypeKind::Class)
            .add_property(
                PropertySpec::builder("count", TypeName::primitive("Int"))
                    .getter(CodeBlock::of("return _count", ()).unwrap())
                    .setter("newValue", CodeBlock::of("_count = newValue", ()).unwrap())
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap(),
        &swift,
    );
    assert!(output.contains("class Counter {"));
    assert!(output.contains("var count: Int {"));
    assert!(output.contains("get {"));
    assert!(output.contains("set(newValue) {"));
}

// ── Kotlin: Field style ─────────────────────────────────

#[test]
fn test_kotlin_getter_only() {
    let kt = Kotlin::new();
    let output = render_property(
        &PropertySpec::builder("count", TypeName::primitive("Int"))
            .getter(CodeBlock::of("return _count", ()).unwrap())
            .build()
            .unwrap(),
        &kt,
        DeclarationContext::Member,
    );
    // Getter-only → readonly → "val" keyword.
    assert!(output.contains("val count: Int\n"));
    assert!(output.contains("get() {"));
    assert!(output.contains("return _count"));
}

#[test]
fn test_kotlin_getter_setter() {
    let kt = Kotlin::new();
    let output = render_property(
        &PropertySpec::builder("name", TypeName::primitive("String"))
            .getter(CodeBlock::of("return field", ()).unwrap())
            .setter("value", CodeBlock::of("field = value", ()).unwrap())
            .build()
            .unwrap(),
        &kt,
        DeclarationContext::Member,
    );
    // Getter+setter → mutable → "var" keyword.
    assert!(output.contains("var name: String\n"));
    assert!(output.contains("get() {"));
    assert!(output.contains("set(value) {"));
}

#[test]
fn test_kotlin_property_in_class() {
    let kt = Kotlin::new();
    let output = render_type(
        &TypeSpec::builder("Person", TypeKind::Class)
            .add_property(
                PropertySpec::builder("name", TypeName::primitive("String"))
                    .getter(CodeBlock::of("return field", ()).unwrap())
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap(),
        &kt,
    );
    assert!(output.contains("class Person {"));
    assert!(output.contains("val name: String\n"));
    assert!(output.contains("get() {"));
}

// ── Doc comments ────────────────────────────────────────

#[test]
fn test_property_with_doc() {
    let ts = TypeScript::new();
    let output = render_property(
        &PropertySpec::builder("count", TypeName::primitive("number"))
            .doc("The current count.")
            .getter(CodeBlock::of("return this._count", ()).unwrap())
            .build()
            .unwrap(),
        &ts,
        DeclarationContext::Member,
    );
    assert!(output.contains("* The current count."));
    assert!(output.contains("get count(): number {"));
}

// ── Static property ─────────────────────────────────────

#[test]
fn test_ts_static_property() {
    let ts = TypeScript::new();
    let output = render_property(
        &PropertySpec::builder("instance", TypeName::primitive("App"))
            .is_static()
            .getter(CodeBlock::of("return App._instance", ()).unwrap())
            .build()
            .unwrap(),
        &ts,
        DeclarationContext::Member,
    );
    assert!(output.contains("static get instance(): App {"));
}
