use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::go::Go;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::TypeKind;
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::spec::where_spec::TypeParamSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

#[test]
fn test_struct_with_tags() {
    let file = FileSpec::builder_with("config.go", Go::new())
        .header(CodeBlock::of("package config", ()).unwrap())
        .add_type(
            TypeSpec::builder("Config", TypeKind::Struct)
                .doc("Config holds application configuration.")
                .add_field(
                    FieldSpec::builder("Name", TypeName::primitive("string"))
                        .tag("json:\"name\"")
                        .build()
                        .unwrap(),
                )
                .add_field(
                    FieldSpec::builder("Port", TypeName::primitive("int"))
                        .tag("json:\"port\" yaml:\"port\"")
                        .build()
                        .unwrap(),
                )
                .add_field(
                    FieldSpec::builder("Debug", TypeName::primitive("bool"))
                        .tag("json:\"debug,omitempty\"")
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("go/struct_with_tags.go", &output);
}

#[test]
fn test_struct_with_methods() {
    let json_marshal = TypeName::importable("encoding/json", "Marshal");

    // Struct definition.
    let tb = TypeSpec::builder("Server", TypeKind::Struct)
        .doc("Server is an HTTP server.")
        .add_field(
            FieldSpec::builder("host", TypeName::primitive("string"))
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("port", TypeName::primitive("int"))
                .build()
                .unwrap(),
        );

    // Method 1: Start.
    let m1 = FunSpec::builder("Start")
        .receiver(
            ParameterSpec::new("s", TypeName::pointer(TypeName::primitive("Server"))).unwrap(),
        )
        .returns(TypeName::primitive("error"))
        .body(CodeBlock::of("return nil", ()).unwrap());

    // Method 2: ToJSON.
    let file = FileSpec::builder_with("server.go", Go::new())
        .header(CodeBlock::of("package server", ()).unwrap())
        .add_type(tb.build().unwrap())
        .add_function(m1.build().unwrap())
        .add_function(
            FunSpec::builder("ToJSON")
                .receiver(
                    ParameterSpec::new("s", TypeName::pointer(TypeName::primitive("Server")))
                        .unwrap(),
                )
                .returns(TypeName::raw("([]byte, error)"))
                .body(CodeBlock::of("return %T(s)", (json_marshal,)).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("go/struct_with_methods.go", &output);
}

#[test]
fn test_interface() {
    let file = FileSpec::builder_with("repo.go", Go::new())
        .header(CodeBlock::of("package repo", ()).unwrap())
        .add_type(
            TypeSpec::builder("Repository", TypeKind::Interface)
                .doc("Repository defines data access methods.")
                .add_method(
                    FunSpec::builder("FindByID")
                        .add_param(ParameterSpec::new("id", TypeName::primitive("string")).unwrap())
                        .returns(TypeName::raw("(Entity, error)"))
                        .build()
                        .unwrap(),
                )
                .add_method(
                    FunSpec::builder("Save")
                        .add_param(
                            ParameterSpec::new("entity", TypeName::primitive("Entity")).unwrap(),
                        )
                        .returns(TypeName::primitive("error"))
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("go/interface.go", &output);
}

#[test]
fn test_generic_function() {
    let tp = TypeParamSpec::new("T").with_bound(TypeName::primitive("comparable"));

    let mut body_b = CodeBlock::builder();
    body_b.begin_control_flow("if a > b", ());
    body_b.add_statement("return a", ());
    body_b.end_control_flow();
    body_b.add_statement("return b", ());
    let body = body_b.build().unwrap();

    let fb = FunSpec::builder("Max")
        .add_type_param(tp)
        .add_param(ParameterSpec::new("a", TypeName::primitive("T")).unwrap())
        .add_param(ParameterSpec::new("b", TypeName::primitive("T")).unwrap())
        .returns(TypeName::primitive("T"))
        .body(body);

    let file = FileSpec::builder_with("max.go", Go::new())
        .header(CodeBlock::of("package main", ()).unwrap())
        .add_function(fb.build().unwrap())
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("go/generic_function.go", &output);
}

#[test]
fn test_embedded_struct() {
    let file = FileSpec::builder_with("admin.go", Go::new())
        .header(CodeBlock::of("package models", ()).unwrap())
        .add_type(
            TypeSpec::builder("UserAdmin", TypeKind::Struct)
                .doc("UserAdmin composes User and Admin.")
                .add_embedded(TypeName::primitive("User"))
                .add_embedded(TypeName::primitive("Admin"))
                .add_field(
                    FieldSpec::builder("Role", TypeName::primitive("string"))
                        .tag("json:\"role\"")
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("go/embedded_struct.go", &output);
}
