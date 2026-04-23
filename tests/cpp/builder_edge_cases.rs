use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::cpp_lang::CppLang;
use sigil_stitch::spec::annotation_spec::AnnotationSpec;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::DeclarationContext;
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

/// Helper: emit a FunSpec as a CodeBlock for embedding in extra_member.
fn emit_fun(fun: &FunSpec<CppLang>) -> CodeBlock<CppLang> {
    let lang = CppLang::new();
    fun.emit(&lang, DeclarationContext::Member).unwrap()
}

/// Helper: emit a FieldSpec as a CodeBlock for embedding in extra_member.
fn emit_field(field: &FieldSpec<CppLang>) -> CodeBlock<CppLang> {
    let lang = CppLang::new();
    field.emit(&lang, DeclarationContext::Member).unwrap()
}

#[test]
fn test_namespace_wrapping() {
    let mut b = CodeBlock::<CppLang>::builder();
    b.add("int square(int x) {", ());
    b.add_line();
    b.add("%>", ());
    b.add("return x * x;", ());
    b.add_line();
    b.add("%<", ());
    b.add("}", ());
    b.add_line();
    let block = b.build().unwrap();

    let mut fb = FileSpec::builder_with("math.hpp", CppLang::header());
    fb.header(CodeBlock::<CppLang>::of("#pragma once", ()).unwrap());
    fb.add_raw("namespace math {\n");
    fb.add_code(block);
    fb.add_raw("\n} // namespace math\n");
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("cpp/namespace_wrapping.cpp", &output);
}

#[test]
fn test_full_header() {
    let iostream = TypeName::<CppLang>::importable("iostream", "std::cout");
    let string_h = TypeName::<CppLang>::importable("string", "std::string");

    // Logger class inheriting from Base.
    let mut tb = sigil_stitch::spec::type_spec::TypeSpec::<CppLang>::builder(
        "Logger",
        sigil_stitch::spec::modifiers::TypeKind::Class,
    );
    tb.extends(TypeName::primitive("Base"));
    tb.doc("Application logger.");

    // private: section
    let mut priv_section = CodeBlock::<CppLang>::builder();
    priv_section.add("%<", ());
    priv_section.add("private:", ());
    priv_section.add_line();
    priv_section.add("%>", ());
    let field = FieldSpec::builder("name_", TypeName::primitive("std::string"))
        .build()
        .unwrap();
    priv_section.add_code(emit_field(&field));
    tb.extra_member(priv_section.build().unwrap());

    // public: section
    let mut pub_section = CodeBlock::<CppLang>::builder();
    pub_section.add_line();
    pub_section.add("%<", ());
    pub_section.add("public:", ());
    pub_section.add_line();
    pub_section.add("%>", ());

    // Constructor
    let ctor_body = CodeBlock::<CppLang>::of("name_ = name;", ()).unwrap();
    let mut ctor = FunSpec::<CppLang>::builder("Logger");
    ctor.add_param(ParameterSpec::new("name", TypeName::primitive("const std::string&")).unwrap());
    ctor.body(ctor_body);
    pub_section.add_code(emit_fun(&ctor.build().unwrap()));

    // log method using imports
    pub_section.add_line();
    let log_body = CodeBlock::<CppLang>::of(
        "%T << name_ << \": \" << %T(msg) << std::endl;",
        (iostream, string_h),
    )
    .unwrap();
    let mut log = FunSpec::<CppLang>::builder("log");
    log.add_param(ParameterSpec::new("msg", TypeName::primitive("const char*")).unwrap());
    log.returns(TypeName::primitive("void"));
    log.body(log_body);
    pub_section.add_code(emit_fun(&log.build().unwrap()));

    // name getter — const
    pub_section.add_line();
    let name_body = CodeBlock::<CppLang>::of("return name_;", ()).unwrap();
    let mut name_fn = FunSpec::<CppLang>::builder("name");
    name_fn.returns(TypeName::primitive("const std::string&"));
    name_fn.suffix("const");
    name_fn.body(name_body);
    pub_section.add_code(emit_fun(&name_fn.build().unwrap()));

    tb.extra_member(pub_section.build().unwrap());

    let ts = tb.build().unwrap();

    // Trigger base.hpp import.
    let import_trigger = CodeBlock::<CppLang>::of(
        "// Uses %T",
        (TypeName::<CppLang>::importable("./base.hpp", "Base"),),
    )
    .unwrap();

    let mut fb = FileSpec::builder_with("logger.hpp", CppLang::header());
    fb.header(CodeBlock::<CppLang>::of("#pragma once", ()).unwrap());
    fb.add_code(import_trigger);
    fb.add_type(ts);
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("cpp/full_header.cpp", &output);
}

#[test]
fn test_annotation_attribute() {
    let mut fb = FunSpec::<CppLang>::builder("compute");
    fb.annotate(AnnotationSpec::new("nodiscard"));
    fb.returns(TypeName::primitive("int"));
    fb.body(CodeBlock::of("return 42;", ()).unwrap());
    let fun = fb.build().unwrap();

    let rendered = emit_fun(&fun);
    let mut file_b = FileSpec::builder_with("compute.hpp", CppLang::header());
    file_b.add_code(rendered);
    let file = file_b.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("cpp/annotation_attribute.cpp", &output);
}
