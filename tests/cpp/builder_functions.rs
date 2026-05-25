use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::cpp::Cpp;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

#[test]
fn test_const_method() {
    let fun = FunSpec::builder("size")
        .returns(TypeName::primitive("int"))
        .suffix("const")
        .suffix("noexcept")
        .build()
        .unwrap();

    let file = FileSpec::builder_with("api.hpp", Cpp::header())
        .add_function(fun)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("cpp/const_method.cpp", &output);
}

#[test]
fn test_template_function() {
    let body = CodeBlock::of("return (a > b) ? a : b;", ()).unwrap();
    let fun = FunSpec::builder("max_of")
        .annotation(CodeBlock::of("template<typename T>", ()).unwrap())
        .add_param(ParameterSpec::new("a", TypeName::primitive("const T&")).unwrap())
        .add_param(ParameterSpec::new("b", TypeName::primitive("const T&")).unwrap())
        .returns(TypeName::primitive("T"))
        .body(body)
        .build()
        .unwrap();

    let file = FileSpec::builder_with("algo.hpp", Cpp::header())
        .add_function(fun)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("cpp/template_function.cpp", &output);
}

#[test]
fn test_static_method() {
    let body = CodeBlock::of("return instance_count_;", ()).unwrap();
    let fun = FunSpec::builder("count")
        .is_static()
        .returns(TypeName::primitive("int"))
        .body(body)
        .build()
        .unwrap();

    let file = FileSpec::builder_with("helpers.cpp", Cpp::new())
        .add_function(fun)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("cpp/static_method.cpp", &output);
}

#[test]
fn test_function_with_doc() {
    let body = CodeBlock::of("return (a > b) ? a : b;", ()).unwrap();
    let fun = FunSpec::builder("max_val")
        .doc("Return the larger of two values.")
        .add_param(ParameterSpec::new("a", TypeName::primitive("int")).unwrap())
        .add_param(ParameterSpec::new("b", TypeName::primitive("int")).unwrap())
        .returns(TypeName::primitive("int"))
        .body(body)
        .build()
        .unwrap();

    let file = FileSpec::builder_with("math_doc.cpp", Cpp::new())
        .add_function(fun)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("cpp/function_with_doc.cpp", &output);
}
