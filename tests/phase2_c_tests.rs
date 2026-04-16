mod golden;

use sigil_stitch::code_block::{CodeBlock, StringLitArg};
use sigil_stitch::lang::c_lang::CLang;
use sigil_stitch::spec::enum_variant_spec::EnumVariantSpec;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::{TypeKind, Visibility};
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

#[test]
fn test_c_struct_with_fields() {
    let mut tb = TypeSpec::<CLang>::builder("Config", TypeKind::Struct);
    tb.doc("Application configuration.");
    tb.add_field(
        FieldSpec::builder("timeout", TypeName::primitive("int"))
            .build()
            .unwrap(),
    );
    tb.add_field(
        FieldSpec::builder("name", TypeName::primitive("char*"))
            .build()
            .unwrap(),
    );
    tb.add_field(
        FieldSpec::builder("verbose", TypeName::primitive("int"))
            .build()
            .unwrap(),
    );
    let ts = tb.build().unwrap();

    let mut fb = FileSpec::builder_with("config.h", CLang::header());
    fb.header(CodeBlock::<CLang>::of("#pragma once", ()).unwrap());
    fb.add_type(ts);
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("c/struct_with_fields.c", &output);
}

#[test]
fn test_c_function_with_params() {
    let body = CodeBlock::<CLang>::of("return a + b;", ()).unwrap();
    let mut fb = FunSpec::<CLang>::builder("add");
    fb.add_param(ParameterSpec::new("a", TypeName::primitive("int")).unwrap());
    fb.add_param(ParameterSpec::new("b", TypeName::primitive("int")).unwrap());
    fb.returns(TypeName::primitive("int"));
    fb.body(body);
    let fun = fb.build().unwrap();

    let mut file_b = FileSpec::builder_with("math.c", CLang::new());
    file_b.add_function(fun);
    let file = file_b.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("c/function_with_params.c", &output);
}

#[test]
fn test_c_void_function() {
    let printf_type = TypeName::<CLang>::importable("stdio.h", "printf");
    let body = CodeBlock::<CLang>::of(
        "%T(%S, name);",
        (printf_type, StringLitArg("Hello, %s!\\n".to_string())),
    )
    .unwrap();
    let mut fb = FunSpec::<CLang>::builder("greet");
    fb.add_param(ParameterSpec::new("name", TypeName::primitive("const char*")).unwrap());
    fb.returns(TypeName::primitive("void"));
    fb.body(body);
    let fun = fb.build().unwrap();

    let mut file_b = FileSpec::builder_with("greet.c", CLang::new());
    file_b.add_function(fun);
    let file = file_b.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("c/void_function.c", &output);
}

#[test]
fn test_c_enum() {
    let mut tb = TypeSpec::<CLang>::builder("Direction", TypeKind::Enum);
    tb.doc("Cardinal directions.");
    tb.add_variant(EnumVariantSpec::new("UP").unwrap());
    tb.add_variant(EnumVariantSpec::new("DOWN").unwrap());
    tb.add_variant(EnumVariantSpec::new("LEFT").unwrap());
    tb.add_variant(EnumVariantSpec::new("RIGHT").unwrap());
    let ts = tb.build().unwrap();

    let mut fb = FileSpec::builder_with("direction.h", CLang::header());
    fb.add_type(ts);
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("c/enum.c", &output);
}

#[test]
fn test_c_static_function() {
    let body = CodeBlock::<CLang>::of("return x * x;", ()).unwrap();
    let mut fb = FunSpec::<CLang>::builder("square");
    fb.visibility(Visibility::Private);
    fb.is_static();
    fb.add_param(ParameterSpec::new("x", TypeName::primitive("int")).unwrap());
    fb.returns(TypeName::primitive("int"));
    fb.body(body);
    let fun = fb.build().unwrap();

    let mut file_b = FileSpec::builder_with("helpers.c", CLang::new());
    file_b.add_function(fun);
    let file = file_b.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("c/static_function.c", &output);
}

#[test]
fn test_c_function_declaration() {
    // Forward declaration — no body, should end with semicolon.
    let mut fb = FunSpec::<CLang>::builder("process");
    fb.add_param(ParameterSpec::new("data", TypeName::primitive("const char*")).unwrap());
    fb.add_param(ParameterSpec::new("len", TypeName::primitive("size_t")).unwrap());
    fb.returns(TypeName::primitive("int"));
    let fun = fb.build().unwrap();

    let mut file_b = FileSpec::builder_with("api.h", CLang::header());
    file_b.add_function(fun);
    let file = file_b.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("c/function_declaration.c", &output);
}

#[test]
fn test_c_struct_with_function() {
    // Struct definition + a separate function that uses it.
    let mut tb = TypeSpec::<CLang>::builder("Point", TypeKind::Struct);
    tb.add_field(
        FieldSpec::builder("x", TypeName::primitive("int"))
            .build()
            .unwrap(),
    );
    tb.add_field(
        FieldSpec::builder("y", TypeName::primitive("int"))
            .build()
            .unwrap(),
    );
    let ts = tb.build().unwrap();

    let body = CodeBlock::<CLang>::of("return p.x + p.y;", ()).unwrap();
    let mut fb = FunSpec::<CLang>::builder("point_sum");
    fb.add_param(ParameterSpec::new("p", TypeName::primitive("struct Point")).unwrap());
    fb.returns(TypeName::primitive("int"));
    fb.body(body);
    let fun = fb.build().unwrap();

    let mut file_b = FileSpec::builder_with("point.c", CLang::new());
    file_b.add_type(ts);
    file_b.add_function(fun);
    let file = file_b.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("c/struct_with_function.c", &output);
}

#[test]
fn test_c_top_level_with_includes() {
    // Full file with #pragma once, includes, struct, function declarations.
    let stdio = TypeName::<CLang>::importable("stdio.h", "printf");
    let config = TypeName::<CLang>::importable("./config.h", "Config");

    let mut tb = TypeSpec::<CLang>::builder("Server", TypeKind::Struct);
    tb.doc("HTTP server configuration.");
    tb.add_field(
        FieldSpec::builder("port", TypeName::primitive("int"))
            .build()
            .unwrap(),
    );
    tb.add_field(
        FieldSpec::builder("host", TypeName::primitive("const char*"))
            .build()
            .unwrap(),
    );
    let ts = tb.build().unwrap();

    // Function that uses imports.
    let body = CodeBlock::<CLang>::of(
        "%T(%S, srv.host, srv.port, %T());",
        (
            stdio,
            StringLitArg("Starting %s:%d with config %p\\n".to_string()),
            config,
        ),
    )
    .unwrap();
    let mut fb = FunSpec::<CLang>::builder("server_start");
    fb.add_param(ParameterSpec::new("srv", TypeName::primitive("struct Server")).unwrap());
    fb.returns(TypeName::primitive("void"));
    fb.body(body);
    let fun = fb.build().unwrap();

    let mut file_b = FileSpec::builder_with("server.h", CLang::header());
    file_b.header(CodeBlock::<CLang>::of("#pragma once", ()).unwrap());
    file_b.add_type(ts);
    file_b.add_function(fun);
    let file = file_b.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("c/top_level_with_includes.c", &output);
}
