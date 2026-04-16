use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::c_lang::CLang;
use sigil_stitch::lang::cpp_lang::CppLang;
use sigil_stitch::lang::java_lang::JavaLang;
use sigil_stitch::lang::kotlin::Kotlin;
use sigil_stitch::lang::python::Python;
use sigil_stitch::lang::rust_lang::RustLang;
use sigil_stitch::lang::typescript::TypeScript;
use sigil_stitch::spec::annotation_spec::AnnotationSpec;
use sigil_stitch::spec::enum_variant_spec::EnumVariantSpec;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::{DeclarationContext, TypeKind, Visibility};
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

fn render_field<L: sigil_stitch::lang::CodeLang>(
    spec: &FieldSpec<L>,
    lang: &L,
    ctx: DeclarationContext,
) -> String {
    let block = spec.emit(lang, ctx).unwrap();
    let imports = sigil_stitch::import::ImportGroup::new();
    let mut renderer = sigil_stitch::code_renderer::CodeRenderer::new(lang, &imports, 80);
    renderer.render(&block).unwrap()
}

// ── TypeScript ───────────────────────────────────────────

#[test]
fn test_ts_simple_annotation_on_fun() {
    let ts = TypeScript::new();
    let mut fb = FunSpec::<TypeScript>::builder("handleRequest");
    fb.annotate(AnnotationSpec::new("deprecated"));
    fb.returns(TypeName::primitive("void"));
    fb.body(CodeBlock::of("// todo", ()).unwrap());
    let output = render_fun(&fb.build().unwrap(), &ts, DeclarationContext::TopLevel);
    assert!(output.contains("@deprecated\n"));
    assert!(output.contains("function handleRequest(): void {"));
}

#[test]
fn test_ts_annotation_with_args() {
    let ts = TypeScript::new();
    let mut fb = FunSpec::<TypeScript>::builder("myMethod");
    fb.annotate(AnnotationSpec::new("Component").arg("selector: 'app-root'"));
    fb.body(CodeBlock::of("// body", ()).unwrap());
    let output = render_fun(&fb.build().unwrap(), &ts, DeclarationContext::Member);
    assert!(output.contains("@Component(selector: 'app-root')\n"));
}

#[test]
fn test_ts_annotation_on_class() {
    let ts = TypeScript::new();
    let mut tb = TypeSpec::<TypeScript>::builder("AppComponent", TypeKind::Class);
    tb.annotate(AnnotationSpec::new("Component").arg("{ selector: 'app-root' }"));
    tb.visibility(Visibility::Public);
    let output = render_type(&tb.build().unwrap(), &ts);
    assert!(output.contains("@Component({ selector: 'app-root' })\n"));
    assert!(output.contains("export class AppComponent {"));
}

#[test]
fn test_ts_annotation_on_field() {
    let ts = TypeScript::new();
    let mut fb = FieldSpec::builder("name", TypeName::<TypeScript>::primitive("string"));
    fb.annotate(AnnotationSpec::new("Required"));
    let output = render_field(&fb.build().unwrap(), &ts, DeclarationContext::Member);
    assert!(output.contains("@Required\n"));
    assert!(output.contains("name: string;"));
}

// ── Rust ─────────────────────────────────────────────────

#[test]
fn test_rust_derive_annotation() {
    let rs = RustLang::new();
    let mut tb = TypeSpec::<RustLang>::builder("Config", TypeKind::Struct);
    tb.annotate(AnnotationSpec::new("derive").arg("Debug, Clone, Serialize"));
    tb.visibility(Visibility::Public);
    let output = render_type(&tb.build().unwrap(), &rs);
    assert!(output.contains("#[derive(Debug, Clone, Serialize)]"));
    assert!(output.contains("pub struct Config {"));
}

#[test]
fn test_rust_allow_annotation() {
    let rs = RustLang::new();
    let mut fb = FunSpec::<RustLang>::builder("old_func");
    fb.annotate(AnnotationSpec::new("allow").arg("dead_code"));
    fb.body(CodeBlock::of("// noop", ()).unwrap());
    let output = render_fun(&fb.build().unwrap(), &rs, DeclarationContext::TopLevel);
    assert!(output.contains("#[allow(dead_code)]"));
    assert!(output.contains("fn old_func()"));
}

#[test]
fn test_rust_no_args_annotation() {
    let rs = RustLang::new();
    let mut fb = FunSpec::<RustLang>::builder("my_test");
    fb.annotate(AnnotationSpec::new("test"));
    fb.body(CodeBlock::of("assert!(true)", ()).unwrap());
    let output = render_fun(&fb.build().unwrap(), &rs, DeclarationContext::TopLevel);
    assert!(output.contains("#[test]"));
}

#[test]
fn test_rust_annotation_on_field() {
    let rs = RustLang::new();
    let mut fb = FieldSpec::builder("name", TypeName::<RustLang>::primitive("String"));
    fb.annotate(AnnotationSpec::new("serde").arg("rename = \"user_name\""));
    fb.visibility(Visibility::Public);
    let output = render_field(&fb.build().unwrap(), &rs, DeclarationContext::Member);
    assert!(output.contains("#[serde(rename = \"user_name\")]"));
    assert!(output.contains("pub name: String,"));
}

#[test]
fn test_rust_annotation_on_enum_variant() {
    let rs = RustLang::new();
    let mut tb = TypeSpec::<RustLang>::builder("Color", TypeKind::Enum);
    let mut v = EnumVariantSpec::builder("Default");
    v.annotate(AnnotationSpec::new("default"));
    tb.add_variant(v.build().unwrap());
    tb.add_variant(EnumVariantSpec::new("Custom").unwrap());
    let output = render_type(&tb.build().unwrap(), &rs);
    assert!(output.contains("#[default]"));
    assert!(output.contains("Default,"));
    assert!(output.contains("Custom,"));
}

// ── C++ ──────────────────────────────────────────────────

#[test]
fn test_cpp_nodiscard_annotation() {
    let cpp = CppLang::new();
    let mut fb = FunSpec::<CppLang>::builder("compute");
    fb.annotate(AnnotationSpec::new("nodiscard"));
    fb.returns(TypeName::primitive("int"));
    fb.body(CodeBlock::of("return 42;", ()).unwrap());
    let output = render_fun(&fb.build().unwrap(), &cpp, DeclarationContext::TopLevel);
    assert!(output.contains("[[nodiscard]]"));
}

#[test]
fn test_cpp_deprecated_with_reason() {
    let cpp = CppLang::new();
    let mut fb = FunSpec::<CppLang>::builder("oldFunc");
    fb.annotate(AnnotationSpec::new("deprecated").arg("\"use newFunc instead\""));
    fb.returns(TypeName::primitive("void"));
    fb.body(CodeBlock::of("// noop", ()).unwrap());
    let output = render_fun(&fb.build().unwrap(), &cpp, DeclarationContext::TopLevel);
    assert!(output.contains("[[deprecated(\"use newFunc instead\")]]"));
}

// ── C ────────────────────────────────────────────────────

#[test]
fn test_c_attribute_annotation() {
    let c = CLang::new();
    let mut fb = FunSpec::<CLang>::builder("init");
    fb.annotate(AnnotationSpec::new("constructor"));
    fb.returns(TypeName::primitive("void"));
    fb.body(CodeBlock::of("// startup", ()).unwrap());
    let output = render_fun(&fb.build().unwrap(), &c, DeclarationContext::TopLevel);
    assert!(output.contains("__attribute__((constructor))"));
}

#[test]
fn test_c_attribute_with_args() {
    let c = CLang::new();
    let mut fb = FunSpec::<CLang>::builder("alloc");
    fb.annotate(AnnotationSpec::new("malloc").arg("free, 1"));
    fb.returns(TypeName::primitive("void*"));
    fb.body(CodeBlock::of("return malloc(size);", ()).unwrap());
    let output = render_fun(&fb.build().unwrap(), &c, DeclarationContext::TopLevel);
    assert!(output.contains("__attribute__((malloc(free, 1)))"));
}

// ── Java ─────────────────────────────────────────────────

#[test]
fn test_java_override_annotation() {
    let java = JavaLang::new();
    let mut fb = FunSpec::<JavaLang>::builder("toString");
    fb.annotate(AnnotationSpec::new("Override"));
    fb.visibility(Visibility::Public);
    fb.returns(TypeName::primitive("String"));
    fb.body(CodeBlock::of("return \"\";", ()).unwrap());
    let output = render_fun(&fb.build().unwrap(), &java, DeclarationContext::Member);
    assert!(output.contains("@Override\n"));
    assert!(output.contains("public String toString()"));
}

#[test]
fn test_java_suppress_warnings() {
    let java = JavaLang::new();
    let mut fb = FunSpec::<JavaLang>::builder("process");
    fb.annotate(AnnotationSpec::new("SuppressWarnings").arg("\"unchecked\""));
    fb.body(CodeBlock::of("// raw types", ()).unwrap());
    let output = render_fun(&fb.build().unwrap(), &java, DeclarationContext::TopLevel);
    assert!(output.contains("@SuppressWarnings(\"unchecked\")"));
}

// ── Kotlin ───────────────────────────────────────────────

#[test]
fn test_kotlin_jvm_static() {
    let kt = Kotlin::new();
    let mut fb = FunSpec::<Kotlin>::builder("getInstance");
    fb.annotate(AnnotationSpec::new("JvmStatic"));
    fb.returns(TypeName::primitive("Singleton"));
    fb.body(CodeBlock::of("return INSTANCE", ()).unwrap());
    let output = render_fun(&fb.build().unwrap(), &kt, DeclarationContext::Member);
    assert!(output.contains("@JvmStatic\n"));
}

// ── Python ───────────────────────────────────────────────

#[test]
fn test_python_decorator() {
    let py = Python::new();
    let mut fb = FunSpec::<Python>::builder("my_method");
    fb.annotate(AnnotationSpec::new("staticmethod"));
    fb.body(CodeBlock::of("pass", ()).unwrap());
    let output = render_fun(&fb.build().unwrap(), &py, DeclarationContext::Member);
    assert!(output.contains("@staticmethod\n"));
}

#[test]
fn test_python_dataclass_decorator() {
    let py = Python::new();
    let mut tb = TypeSpec::<Python>::builder("Config", TypeKind::Class);
    tb.annotate(AnnotationSpec::new("dataclass").arg("frozen=True"));
    let output = render_type(&tb.build().unwrap(), &py);
    assert!(output.contains("@dataclass(frozen=True)\n"));
    assert!(output.contains("class Config:"));
}

// ── Mixed annotations (both spec + CodeBlock) ──────────

#[test]
fn test_mixed_annotation_and_codeblock() {
    let rs = RustLang::new();
    let mut fb = FunSpec::<RustLang>::builder("test_it");
    // Structured annotation first.
    fb.annotate(AnnotationSpec::new("cfg").arg("test"));
    // Raw CodeBlock annotation second.
    fb.annotation(CodeBlock::of("#[ignore]", ()).unwrap());
    fb.body(CodeBlock::of("// test body", ()).unwrap());
    let output = render_fun(&fb.build().unwrap(), &rs, DeclarationContext::TopLevel);
    // Structured specs appear before raw CodeBlock annotations.
    let cfg_pos = output
        .find("#[cfg(test)]")
        .expect("should contain #[cfg(test)]");
    let ignore_pos = output.find("#[ignore]").expect("should contain #[ignore]");
    assert!(cfg_pos < ignore_pos, "spec annotations should come first");
}

// ── Importable annotation ─────────────────────────────

#[test]
fn test_importable_annotation_ts() {
    let mut fb = FileSpec::<TypeScript>::builder("app.ts");
    let decorator_type = TypeName::<TypeScript>::importable("./decorators", "Component");
    let mut fun_b = FunSpec::<TypeScript>::builder("init");
    fun_b.annotate(AnnotationSpec::importable(decorator_type).arg("{ selector: 'app' }"));
    fun_b.body(CodeBlock::of("// init", ()).unwrap());
    fb.add_function(fun_b.build().unwrap());
    let output = fb.build().unwrap().render(80).unwrap();
    // The import should be tracked.
    assert!(output.contains("import { Component } from './decorators'"));
    // The annotation should use the resolved name.
    assert!(output.contains("@Component({ selector: 'app' })"));
}

#[test]
fn test_importable_annotation_java() {
    let mut fb = FileSpec::<JavaLang>::builder("App.java");
    let nullable = TypeName::<JavaLang>::importable("javax.annotation", "Nullable");
    let mut fun_b = FunSpec::<JavaLang>::builder("getUser");
    fun_b.annotate(AnnotationSpec::importable(nullable));
    fun_b.returns(TypeName::primitive("User"));
    fun_b.body(CodeBlock::of("return null;", ()).unwrap());
    fb.add_function(fun_b.build().unwrap());
    let output = fb.build().unwrap().render(80).unwrap();
    assert!(output.contains("import javax.annotation.Nullable;"));
    assert!(output.contains("@Nullable"));
}

// ── Multiple annotations ──────────────────────────────

#[test]
fn test_multiple_annotations_on_fun() {
    let rs = RustLang::new();
    let mut fb = FunSpec::<RustLang>::builder("bench_it");
    fb.annotate(AnnotationSpec::new("cfg").arg("test"));
    fb.annotate(AnnotationSpec::new("bench"));
    fb.body(CodeBlock::of("// benchmark", ()).unwrap());
    let output = render_fun(&fb.build().unwrap(), &rs, DeclarationContext::TopLevel);
    assert!(output.contains("#[cfg(test)]"));
    assert!(output.contains("#[bench]"));
    let cfg_pos = output.find("#[cfg(test)]").unwrap();
    let bench_pos = output.find("#[bench]").unwrap();
    assert!(cfg_pos < bench_pos);
}

#[test]
fn test_multiple_args() {
    let rs = RustLang::new();
    let mut fb = FunSpec::<RustLang>::builder("handler");
    fb.annotate(
        AnnotationSpec::new("cfg")
            .arg("feature = \"web\"")
            .arg("not(test)"),
    );
    fb.body(CodeBlock::of("// handler", ()).unwrap());
    let output = render_fun(&fb.build().unwrap(), &rs, DeclarationContext::TopLevel);
    assert!(output.contains("#[cfg(feature = \"web\", not(test))]"));
}
