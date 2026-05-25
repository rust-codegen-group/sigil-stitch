use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::c::C;
use sigil_stitch::lang::cpp::Cpp;
use sigil_stitch::lang::java::Java;
use sigil_stitch::lang::kotlin::Kotlin;
use sigil_stitch::lang::python::Python;
use sigil_stitch::lang::rust::Rust;
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

fn render_fun(
    spec: &FunSpec,
    lang: &dyn sigil_stitch::lang::CodeLang,
    ctx: DeclarationContext,
) -> String {
    let block = spec.emit(lang, ctx).unwrap();
    let imports = sigil_stitch::import::ImportGroup::new();
    let mut renderer = sigil_stitch::code_renderer::CodeRenderer::new(lang, &imports, 80);
    renderer.render(&block).unwrap()
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

fn render_field(
    spec: &FieldSpec,
    lang: &dyn sigil_stitch::lang::CodeLang,
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
    let output = render_fun(
        &FunSpec::builder("handleRequest")
            .annotate(AnnotationSpec::new("deprecated"))
            .returns(TypeName::primitive("void"))
            .body(CodeBlock::of("// todo", ()).unwrap())
            .build()
            .unwrap(),
        &ts,
        DeclarationContext::TopLevel,
    );
    assert!(output.contains("@deprecated\n"));
    assert!(output.contains("function handleRequest(): void {"));
}

#[test]
fn test_ts_annotation_with_args() {
    let ts = TypeScript::new();
    let output = render_fun(
        &FunSpec::builder("myMethod")
            .annotate(AnnotationSpec::new("Component").arg("selector: 'app-root'"))
            .body(CodeBlock::of("// body", ()).unwrap())
            .build()
            .unwrap(),
        &ts,
        DeclarationContext::Member,
    );
    assert!(output.contains("@Component(selector: 'app-root')\n"));
}

#[test]
fn test_ts_annotation_on_class() {
    let ts = TypeScript::new();
    let output = render_type(
        &TypeSpec::builder("AppComponent", TypeKind::Class)
            .annotate(AnnotationSpec::new("Component").arg("{ selector: 'app-root' }"))
            .visibility(Visibility::Public)
            .build()
            .unwrap(),
        &ts,
    );
    assert!(output.contains("@Component({ selector: 'app-root' })\n"));
    assert!(output.contains("export class AppComponent {"));
}

#[test]
fn test_ts_annotation_on_field() {
    let ts = TypeScript::new();
    let output = render_field(
        &FieldSpec::builder("name", TypeName::primitive("string"))
            .annotate(AnnotationSpec::new("Required"))
            .build()
            .unwrap(),
        &ts,
        DeclarationContext::Member,
    );
    assert!(output.contains("@Required\n"));
    assert!(output.contains("name: string;"));
}

// ── Rust ─────────────────────────────────────────────────

#[test]
fn test_rust_derive_annotation() {
    let rs = Rust::new();
    let output = render_type(
        &TypeSpec::builder("Config", TypeKind::Struct)
            .annotate(AnnotationSpec::new("derive").arg("Debug, Clone, Serialize"))
            .visibility(Visibility::Public)
            .build()
            .unwrap(),
        &rs,
    );
    assert!(output.contains("#[derive(Debug, Clone, Serialize)]"));
    assert!(output.contains("pub struct Config {"));
}

#[test]
fn test_rust_allow_annotation() {
    let rs = Rust::new();
    let output = render_fun(
        &FunSpec::builder("old_func")
            .annotate(AnnotationSpec::new("allow").arg("dead_code"))
            .body(CodeBlock::of("// noop", ()).unwrap())
            .build()
            .unwrap(),
        &rs,
        DeclarationContext::TopLevel,
    );
    assert!(output.contains("#[allow(dead_code)]"));
    assert!(output.contains("fn old_func()"));
}

#[test]
fn test_rust_no_args_annotation() {
    let rs = Rust::new();
    let output = render_fun(
        &FunSpec::builder("my_test")
            .annotate(AnnotationSpec::new("test"))
            .body(CodeBlock::of("assert!(true)", ()).unwrap())
            .build()
            .unwrap(),
        &rs,
        DeclarationContext::TopLevel,
    );
    assert!(output.contains("#[test]"));
}

#[test]
fn test_rust_annotation_on_field() {
    let rs = Rust::new();
    let output = render_field(
        &FieldSpec::builder("name", TypeName::primitive("String"))
            .annotate(AnnotationSpec::new("serde").arg("rename = \"user_name\""))
            .visibility(Visibility::Public)
            .build()
            .unwrap(),
        &rs,
        DeclarationContext::Member,
    );
    assert!(output.contains("#[serde(rename = \"user_name\")]"));
    assert!(output.contains("pub name: String,"));
}

#[test]
fn test_rust_annotation_on_enum_variant() {
    let rs = Rust::new();
    let output = render_type(
        &TypeSpec::builder("Color", TypeKind::Enum)
            .add_variant(
                EnumVariantSpec::builder("Default")
                    .annotate(AnnotationSpec::new("default"))
                    .build()
                    .unwrap(),
            )
            .add_variant(EnumVariantSpec::new("Custom").unwrap())
            .build()
            .unwrap(),
        &rs,
    );
    assert!(output.contains("#[default]"));
    assert!(output.contains("Default,"));
    assert!(output.contains("Custom,"));
}

// ── C++ ──────────────────────────────────────────────────

#[test]
fn test_cpp_nodiscard_annotation() {
    let cpp = Cpp::new();
    let output = render_fun(
        &FunSpec::builder("compute")
            .annotate(AnnotationSpec::new("nodiscard"))
            .returns(TypeName::primitive("int"))
            .body(CodeBlock::of("return 42;", ()).unwrap())
            .build()
            .unwrap(),
        &cpp,
        DeclarationContext::TopLevel,
    );
    assert!(output.contains("[[nodiscard]]"));
}

#[test]
fn test_cpp_deprecated_with_reason() {
    let cpp = Cpp::new();
    let output = render_fun(
        &FunSpec::builder("oldFunc")
            .annotate(AnnotationSpec::new("deprecated").arg("\"use newFunc instead\""))
            .returns(TypeName::primitive("void"))
            .body(CodeBlock::of("// noop", ()).unwrap())
            .build()
            .unwrap(),
        &cpp,
        DeclarationContext::TopLevel,
    );
    assert!(output.contains("[[deprecated(\"use newFunc instead\")]]"));
}

// ── C ────────────────────────────────────────────────────

#[test]
fn test_c_attribute_annotation() {
    let c = C::new();
    let output = render_fun(
        &FunSpec::builder("init")
            .annotate(AnnotationSpec::new("constructor"))
            .returns(TypeName::primitive("void"))
            .body(CodeBlock::of("// startup", ()).unwrap())
            .build()
            .unwrap(),
        &c,
        DeclarationContext::TopLevel,
    );
    assert!(output.contains("__attribute__((constructor))"));
}

#[test]
fn test_c_attribute_with_args() {
    let c = C::new();
    let output = render_fun(
        &FunSpec::builder("alloc")
            .annotate(AnnotationSpec::new("malloc").arg("free, 1"))
            .returns(TypeName::primitive("void*"))
            .body(CodeBlock::of("return malloc(size);", ()).unwrap())
            .build()
            .unwrap(),
        &c,
        DeclarationContext::TopLevel,
    );
    assert!(output.contains("__attribute__((malloc(free, 1)))"));
}

// ── Java ─────────────────────────────────────────────────

#[test]
fn test_java_override_annotation() {
    let java = Java::new();
    let output = render_fun(
        &FunSpec::builder("toString")
            .annotate(AnnotationSpec::new("Override"))
            .visibility(Visibility::Public)
            .returns(TypeName::primitive("String"))
            .body(CodeBlock::of("return \"\";", ()).unwrap())
            .build()
            .unwrap(),
        &java,
        DeclarationContext::Member,
    );
    assert!(output.contains("@Override\n"));
    assert!(output.contains("public String toString()"));
}

#[test]
fn test_java_suppress_warnings() {
    let java = Java::new();
    let output = render_fun(
        &FunSpec::builder("process")
            .annotate(AnnotationSpec::new("SuppressWarnings").arg("\"unchecked\""))
            .body(CodeBlock::of("// raw types", ()).unwrap())
            .build()
            .unwrap(),
        &java,
        DeclarationContext::TopLevel,
    );
    assert!(output.contains("@SuppressWarnings(\"unchecked\")"));
}

// ── Kotlin ───────────────────────────────────────────────

#[test]
fn test_kotlin_jvm_static() {
    let kt = Kotlin::new();
    let output = render_fun(
        &FunSpec::builder("getInstance")
            .annotate(AnnotationSpec::new("JvmStatic"))
            .returns(TypeName::primitive("Singleton"))
            .body(CodeBlock::of("return INSTANCE", ()).unwrap())
            .build()
            .unwrap(),
        &kt,
        DeclarationContext::Member,
    );
    assert!(output.contains("@JvmStatic\n"));
}

// ── Python ───────────────────────────────────────────────

#[test]
fn test_python_decorator() {
    let py = Python::new();
    let output = render_fun(
        &FunSpec::builder("my_method")
            .annotate(AnnotationSpec::new("staticmethod"))
            .body(CodeBlock::of("pass", ()).unwrap())
            .build()
            .unwrap(),
        &py,
        DeclarationContext::Member,
    );
    assert!(output.contains("@staticmethod\n"));
}

#[test]
fn test_python_dataclass_decorator() {
    let py = Python::new();
    let output = render_type(
        &TypeSpec::builder("Config", TypeKind::Class)
            .annotate(AnnotationSpec::new("dataclass").arg("frozen=True"))
            .build()
            .unwrap(),
        &py,
    );
    assert!(output.contains("@dataclass(frozen=True)\n"));
    assert!(output.contains("class Config:"));
}

// ── Mixed annotations (both spec + CodeBlock) ──────────

#[test]
fn test_mixed_annotation_and_codeblock() {
    let rs = Rust::new();
    let fb = FunSpec::builder("test_it")
        .annotate(AnnotationSpec::new("cfg").arg("test"))
        .annotation(CodeBlock::of("#[ignore]", ()).unwrap())
        .body(CodeBlock::of("// test body", ()).unwrap());
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
    let decorator_type = TypeName::importable("./decorators", "Component");
    let output = FileSpec::builder("app.ts")
        .add_function(
            FunSpec::builder("init")
                .annotate(AnnotationSpec::importable(decorator_type).arg("{ selector: 'app' }"))
                .body(CodeBlock::of("// init", ()).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap()
        .render(80)
        .unwrap();
    // The import should be tracked.
    assert!(output.contains("import { Component } from './decorators'"));
    // The annotation should use the resolved name.
    assert!(output.contains("@Component({ selector: 'app' })"));
}

#[test]
fn test_importable_annotation_java() {
    let nullable = TypeName::importable("javax.annotation", "Nullable");
    let output = FileSpec::builder("App.java")
        .add_function(
            FunSpec::builder("getUser")
                .annotate(AnnotationSpec::importable(nullable))
                .returns(TypeName::primitive("User"))
                .body(CodeBlock::of("return null;", ()).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap()
        .render(80)
        .unwrap();
    assert!(output.contains("import javax.annotation.Nullable;"));
    assert!(output.contains("@Nullable"));
}

// ── Multiple annotations ──────────────────────────────

#[test]
fn test_multiple_annotations_on_fun() {
    let rs = Rust::new();
    let output = render_fun(
        &FunSpec::builder("bench_it")
            .annotate(AnnotationSpec::new("cfg").arg("test"))
            .annotate(AnnotationSpec::new("bench"))
            .body(CodeBlock::of("// benchmark", ()).unwrap())
            .build()
            .unwrap(),
        &rs,
        DeclarationContext::TopLevel,
    );
    assert!(output.contains("#[cfg(test)]"));
    assert!(output.contains("#[bench]"));
    let cfg_pos = output.find("#[cfg(test)]").unwrap();
    let bench_pos = output.find("#[bench]").unwrap();
    assert!(cfg_pos < bench_pos);
}

#[test]
fn test_multiple_args() {
    let rs = Rust::new();
    let output = render_fun(
        &FunSpec::builder("handler")
            .annotate(
                AnnotationSpec::new("cfg")
                    .arg("feature = \"web\"")
                    .arg("not(test)"),
            )
            .body(CodeBlock::of("// handler", ()).unwrap())
            .build()
            .unwrap(),
        &rs,
        DeclarationContext::TopLevel,
    );
    assert!(output.contains("#[cfg(feature = \"web\", not(test))]"));
}

// ── .args() bulk method ─────────────────────────────────

#[test]
fn test_rust_args_bulk_derive() {
    let rs = Rust::new();
    let ann = AnnotationSpec::new("derive").args(["Debug", "Clone", "Serialize"]);
    let block = ann.emit(&rs).unwrap();
    let imports = sigil_stitch::import::ImportGroup::new();
    let mut renderer = sigil_stitch::code_renderer::CodeRenderer::new(&rs, &imports, 80);
    let output = renderer.render(&block).unwrap();
    assert_eq!(output, "#[derive(Debug, Clone, Serialize)]");
}

#[test]
fn test_args_combined_with_arg() {
    let rs = Rust::new();
    let ann = AnnotationSpec::new("serde")
        .arg("rename_all = \"camelCase\"")
        .args(["deny_unknown_fields"]);
    let block = ann.emit(&rs).unwrap();
    let imports = sigil_stitch::import::ImportGroup::new();
    let mut renderer = sigil_stitch::code_renderer::CodeRenderer::new(&rs, &imports, 80);
    let output = renderer.render(&block).unwrap();
    assert_eq!(
        output,
        "#[serde(rename_all = \"camelCase\", deny_unknown_fields)]"
    );
}

#[test]
fn test_args_empty_iter_no_parens() {
    let rs = Rust::new();
    let ann = AnnotationSpec::new("test").args(Vec::<&str>::new());
    let block = ann.emit(&rs).unwrap();
    let imports = sigil_stitch::import::ImportGroup::new();
    let mut renderer = sigil_stitch::code_renderer::CodeRenderer::new(&rs, &imports, 80);
    let output = renderer.render(&block).unwrap();
    assert_eq!(output, "#[test]");
}

// ── %N keyword escaping ─────────────────────────────────

#[test]
fn test_name_ref_escapes_rust_keyword() {
    let rs = Rust::new();
    let block = CodeBlock::of(
        "let %N = 1",
        sigil_stitch::code_block::NameArg("type".into()),
    )
    .unwrap();
    let imports = sigil_stitch::import::ImportGroup::new();
    let mut renderer = sigil_stitch::code_renderer::CodeRenderer::new(&rs, &imports, 80);
    let output = renderer.render(&block).unwrap();
    assert_eq!(output, "let r#type = 1");
}

#[test]
fn test_name_ref_escapes_go_keyword() {
    use sigil_stitch::lang::go::Go;
    let go = Go::new();
    let block = CodeBlock::of(
        "var %N int",
        sigil_stitch::code_block::NameArg("func".into()),
    )
    .unwrap();
    let imports = sigil_stitch::import::ImportGroup::new();
    let mut renderer = sigil_stitch::code_renderer::CodeRenderer::new(&go, &imports, 80);
    let output = renderer.render(&block).unwrap();
    assert_eq!(output, "var func_ int");
}

#[test]
fn test_name_ref_no_escape_non_keyword() {
    let rs = Rust::new();
    let block = CodeBlock::of(
        "let %N = 1",
        sigil_stitch::code_block::NameArg("myVar".into()),
    )
    .unwrap();
    let imports = sigil_stitch::import::ImportGroup::new();
    let mut renderer = sigil_stitch::code_renderer::CodeRenderer::new(&rs, &imports, 80);
    let output = renderer.render(&block).unwrap();
    assert_eq!(output, "let myVar = 1");
}
