use sigil_stitch::code_block::{CodeBlock, StringLitArg};
use sigil_stitch::lang::c::C;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::Visibility;
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

#[test]
fn test_function_with_params() {
    let body = CodeBlock::of("return a + b;", ()).unwrap();
    let fun = FunSpec::builder("add")
        .add_param(ParameterSpec::new("a", TypeName::primitive("int")).unwrap())
        .add_param(ParameterSpec::new("b", TypeName::primitive("int")).unwrap())
        .returns(TypeName::primitive("int"))
        .body(body)
        .build()
        .unwrap();

    let file = FileSpec::builder_with("math.c", C::new())
        .add_function(fun)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("c/function_with_params.c", &output);
}

#[test]
fn test_void_function() {
    let printf_type = TypeName::importable("stdio.h", "printf");
    let body = CodeBlock::of(
        "%T(%S, name);",
        (printf_type, StringLitArg("Hello, %s!\n".to_string())),
    )
    .unwrap();
    let fun = FunSpec::builder("greet")
        .add_param(ParameterSpec::new("name", TypeName::primitive("const char*")).unwrap())
        .returns(TypeName::primitive("void"))
        .body(body)
        .build()
        .unwrap();

    let file = FileSpec::builder_with("greet.c", C::new())
        .add_function(fun)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("c/void_function.c", &output);
}

#[test]
fn test_static_function() {
    let body = CodeBlock::of("return x * x;", ()).unwrap();
    let fun = FunSpec::builder("square")
        .visibility(Visibility::Private)
        .is_static()
        .add_param(ParameterSpec::new("x", TypeName::primitive("int")).unwrap())
        .returns(TypeName::primitive("int"))
        .body(body)
        .build()
        .unwrap();

    let file = FileSpec::builder_with("helpers.c", C::new())
        .add_function(fun)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("c/static_function.c", &output);
}

#[test]
fn test_function_declaration() {
    // Forward declaration — no body, should end with semicolon.
    let fun = FunSpec::builder("process")
        .add_param(ParameterSpec::new("data", TypeName::primitive("const char*")).unwrap())
        .add_param(ParameterSpec::new("len", TypeName::primitive("size_t")).unwrap())
        .returns(TypeName::primitive("int"))
        .build()
        .unwrap();

    let file = FileSpec::builder_with("api.h", C::header())
        .add_function(fun)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("c/function_declaration.c", &output);
}

#[test]
fn test_function_with_doc() {
    let body = CodeBlock::of("return a + b;", ()).unwrap();
    let fun = FunSpec::builder("add")
        .doc("Add two integers.")
        .add_param(ParameterSpec::new("a", TypeName::primitive("int")).unwrap())
        .add_param(ParameterSpec::new("b", TypeName::primitive("int")).unwrap())
        .returns(TypeName::primitive("int"))
        .body(body)
        .build()
        .unwrap();

    let file = FileSpec::builder_with("math_doc.c", C::new())
        .add_function(fun)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("c/function_with_doc.c", &output);
}
