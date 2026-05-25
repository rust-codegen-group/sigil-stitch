use sigil_stitch::code_block::{CodeBlock, StringLitArg};
use sigil_stitch::lang::c::C;
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
fn test_arrow_and_pointer_spacing() {
    let mut b = CodeBlock::builder();
    b.add_statement("struct Config* cfg = create_config()", ());
    b.add_statement("cfg->host = %S", (StringLitArg("localhost".to_string()),));
    b.add_statement("cfg->port = 8080", ());
    let block = b.build().unwrap();

    let file = FileSpec::builder_with("test.c", C::new())
        .add_code(block)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("c/builder_arrow_pointer.c", &output);
}

#[test]
fn test_struct_basic() {
    let mut b = CodeBlock::builder();
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

    let file = FileSpec::builder_with("point.c", C::new())
        .add_code(block)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("c/struct_basic.c", &output);
}

#[test]
fn test_struct_with_function() {
    // Struct definition + a separate function that uses it.
    let ts = TypeSpec::builder("Point", TypeKind::Struct)
        .add_field(
            FieldSpec::builder("x", TypeName::primitive("int"))
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("y", TypeName::primitive("int"))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let body = CodeBlock::of("return p.x + p.y;", ()).unwrap();
    let fun = FunSpec::builder("point_sum")
        .add_param(ParameterSpec::new("p", TypeName::primitive("struct Point")).unwrap())
        .returns(TypeName::primitive("int"))
        .body(body)
        .build()
        .unwrap();

    let file = FileSpec::builder_with("point.c", C::new())
        .add_type(ts)
        .add_function(fun)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("c/struct_with_function.c", &output);
}

#[test]
fn test_top_level_with_includes() {
    // Full file with #pragma once, includes, struct, function declarations.
    let stdio = TypeName::importable("stdio.h", "printf");
    let config = TypeName::importable("./config.h", "Config");

    let ts = TypeSpec::builder("Server", TypeKind::Struct)
        .doc("HTTP server configuration.")
        .add_field(
            FieldSpec::builder("port", TypeName::primitive("int"))
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("host", TypeName::primitive("const char*"))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    // Function that uses imports.
    let body = CodeBlock::of(
        "%T(%S, srv.host, srv.port, %T());",
        (
            stdio,
            StringLitArg("Starting %s:%d with config %p\\n".to_string()),
            config,
        ),
    )
    .unwrap();
    let fun = FunSpec::builder("server_start")
        .add_param(ParameterSpec::new("srv", TypeName::primitive("struct Server")).unwrap())
        .returns(TypeName::primitive("void"))
        .body(body)
        .build()
        .unwrap();

    let file = FileSpec::builder_with("server.h", C::header())
        .header(CodeBlock::of("#pragma once", ()).unwrap())
        .add_type(ts)
        .add_function(fun)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("c/top_level_with_includes.c", &output);
}

#[test]
fn test_annotation_attribute() {
    let ts = TypeSpec::builder("PackedData", TypeKind::Struct)
        .annotate(AnnotationSpec::new("packed"))
        .add_field(
            FieldSpec::builder("flags", TypeName::primitive("uint8_t"))
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("value", TypeName::primitive("uint32_t"))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("packed.h", C::header())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("c/annotation_attribute.c", &output);
}
