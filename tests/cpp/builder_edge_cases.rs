use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::cpp::Cpp;
use sigil_stitch::spec::annotation_spec::AnnotationSpec;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::DeclarationContext;
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

/// Helper: emit a FunSpec as a CodeBlock for embedding in extra_member.
fn emit_fun(fun: &FunSpec) -> CodeBlock {
    let lang = Cpp::new();
    fun.emit(&lang, DeclarationContext::Member).unwrap()
}

/// Helper: emit a FieldSpec as a CodeBlock for embedding in extra_member.
fn emit_field(field: &FieldSpec) -> CodeBlock {
    let lang = Cpp::new();
    field.emit(&lang, DeclarationContext::Member).unwrap()
}

#[test]
fn test_namespace_wrapping() {
    let mut b = CodeBlock::builder();
    b.add("int square(int x) {", ());
    b.add_line();
    b.add("%>", ());
    b.add("return x * x;", ());
    b.add_line();
    b.add("%<", ());
    b.add("}", ());
    b.add_line();
    let block = b.build().unwrap();

    let file = FileSpec::builder_with("math.hpp", Cpp::header())
        .header(CodeBlock::of("#pragma once", ()).unwrap())
        .add_raw("namespace math {\n")
        .add_code(block)
        .add_raw("\n} // namespace math\n")
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("cpp/namespace_wrapping.cpp", &output);
}

#[test]
fn test_full_header() {
    let iostream = TypeName::importable("iostream", "std::cout");
    let string_h = TypeName::importable("string", "std::string");

    // Logger class inheriting from Base.
    let tb = sigil_stitch::spec::type_spec::TypeSpec::builder(
        "Logger",
        sigil_stitch::spec::modifiers::TypeKind::Class,
    )
    .extends(TypeName::primitive("Base"))
    .doc("Application logger.");

    // private: section
    let mut priv_section = CodeBlock::builder();
    priv_section.add("%<", ());
    priv_section.add("private:", ());
    priv_section.add_line();
    priv_section.add("%>", ());
    let field = FieldSpec::builder("name_", TypeName::primitive("std::string"))
        .build()
        .unwrap();
    priv_section.add_code(emit_field(&field));
    let tb = tb.extra_member(priv_section.build().unwrap());

    // public: section
    let mut pub_section = CodeBlock::builder();
    pub_section.add_line();
    pub_section.add("%<", ());
    pub_section.add("public:", ());
    pub_section.add_line();
    pub_section.add("%>", ());

    // Constructor
    let ctor_body = CodeBlock::of("name_ = name;", ()).unwrap();
    pub_section.add_code(emit_fun(
        &FunSpec::builder("Logger")
            .add_param(
                ParameterSpec::new("name", TypeName::primitive("const std::string&")).unwrap(),
            )
            .body(ctor_body)
            .build()
            .unwrap(),
    ));

    // log method using imports
    pub_section.add_line();
    let log_body = CodeBlock::of(
        "%T << name_ << \": \" << %T(msg) << std::endl;",
        (iostream, string_h),
    )
    .unwrap();
    pub_section.add_code(emit_fun(
        &FunSpec::builder("log")
            .add_param(ParameterSpec::new("msg", TypeName::primitive("const char*")).unwrap())
            .returns(TypeName::primitive("void"))
            .body(log_body)
            .build()
            .unwrap(),
    ));

    // name getter — const
    pub_section.add_line();
    let name_body = CodeBlock::of("return name_;", ()).unwrap();
    pub_section.add_code(emit_fun(
        &FunSpec::builder("name")
            .returns(TypeName::primitive("const std::string&"))
            .suffix("const")
            .body(name_body)
            .build()
            .unwrap(),
    ));

    let ts = tb
        .extra_member(pub_section.build().unwrap())
        .build()
        .unwrap();

    // Trigger base.hpp import.
    let import_trigger =
        CodeBlock::of("// Uses %T", (TypeName::importable("./base.hpp", "Base"),)).unwrap();

    let file = FileSpec::builder_with("logger.hpp", Cpp::header())
        .header(CodeBlock::of("#pragma once", ()).unwrap())
        .add_code(import_trigger)
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("cpp/full_header.cpp", &output);
}

#[test]
fn test_annotation_attribute() {
    let fun = FunSpec::builder("compute")
        .annotate(AnnotationSpec::new("nodiscard"))
        .returns(TypeName::primitive("int"))
        .body(CodeBlock::of("return 42;", ()).unwrap())
        .build()
        .unwrap();

    let rendered = emit_fun(&fun);
    let file = FileSpec::builder_with("compute.hpp", Cpp::header())
        .add_code(rendered)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("cpp/annotation_attribute.cpp", &output);
}
