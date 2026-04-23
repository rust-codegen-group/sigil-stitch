use sigil_stitch::code_block::{CodeBlock, StringLitArg};
use sigil_stitch::lang::c_lang::CLang;
use sigil_stitch::spec::annotation_spec::AnnotationSpec;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::TypeKind;
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

#[test]
fn test_struct_basic() {
    let mut b = CodeBlock::<CLang>::builder();
    b.add("struct Point", ());
    b.add(" {", ());
    b.add_line();
    b.add("%>", ());
    b.add_statement("int x", ());
    b.add_statement("int y", ());
    b.add("%<", ());
    b.add("};", ());
    b.add_line();
    let block = b.build().unwrap();

    let mut fb = FileSpec::builder_with("point.c", CLang::new());
    fb.add_code(block);
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("c/struct_basic.c", &output);
}

#[test]
fn test_struct_with_function() {
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
fn test_top_level_with_includes() {
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

#[test]
fn test_annotation_attribute() {
    let mut tb = TypeSpec::<CLang>::builder("PackedData", TypeKind::Struct);
    tb.annotate(AnnotationSpec::new("packed"));
    tb.add_field(
        FieldSpec::builder("flags", TypeName::primitive("uint8_t"))
            .build()
            .unwrap(),
    );
    tb.add_field(
        FieldSpec::builder("value", TypeName::primitive("uint32_t"))
            .build()
            .unwrap(),
    );
    let ts = tb.build().unwrap();

    let mut file_b = FileSpec::builder_with("packed.h", CLang::header());
    file_b.add_type(ts);
    let file = file_b.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("c/annotation_attribute.c", &output);
}
