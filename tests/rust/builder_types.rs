use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::rust::Rust;
use sigil_stitch::spec::enum_variant_spec::EnumVariantSpec;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::{TypeKind, Visibility};
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::spec::where_spec::TypeParamSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

#[test]
fn test_struct_with_impl() {
    let derive = CodeBlock::of(
        "#[derive(%T, %T)]",
        (
            TypeName::importable("serde", "Serialize"),
            TypeName::importable("serde", "Deserialize"),
        ),
    )
    .unwrap();

    let body = CodeBlock::of(
        "Self { name: name.to_string(), values: HashMap::new() }",
        (),
    )
    .unwrap();

    let tb = TypeSpec::builder("Config", TypeKind::Struct)
        .visibility(Visibility::Public)
        .doc("Application configuration.")
        .annotation(derive)
        .add_field(
            FieldSpec::builder("name", TypeName::primitive("String"))
                .visibility(Visibility::Public)
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder(
                "values",
                TypeName::generic(
                    TypeName::importable("std::collections", "HashMap"),
                    vec![TypeName::primitive("String"), TypeName::primitive("i64")],
                ),
            )
            .visibility(Visibility::Public)
            .build()
            .unwrap(),
        )
        .add_method(
            FunSpec::builder("new")
                .visibility(Visibility::Public)
                .add_param(ParameterSpec::new("name", TypeName::primitive("&str")).unwrap())
                .returns(TypeName::primitive("Self"))
                .body(body)
                .build()
                .unwrap(),
        );

    let output = FileSpec::builder_with("config.rs", Rust::new())
        .add_type(tb.build().unwrap())
        .build()
        .unwrap()
        .render(100)
        .unwrap();

    golden::assert_golden("rust/struct_with_impl.rs", &output);
}

#[test]
fn test_generic_struct() {
    let tp = TypeParamSpec::new("T")
        .with_bound(TypeName::primitive("Clone"))
        .with_bound(TypeName::primitive("Send"));

    let body = CodeBlock::of("self.items.len()", ()).unwrap();
    let tb = TypeSpec::builder("Container", TypeKind::Struct)
        .visibility(Visibility::Public)
        .add_type_param(tp)
        .add_field(
            FieldSpec::builder(
                "items",
                TypeName::generic(TypeName::primitive("Vec"), vec![TypeName::primitive("T")]),
            )
            .visibility(Visibility::Public)
            .build()
            .unwrap(),
        )
        .add_method(
            FunSpec::builder("len")
                .visibility(Visibility::Public)
                .add_param(ParameterSpec::new("&self", TypeName::primitive("")).unwrap())
                .returns(TypeName::primitive("usize"))
                .body(body)
                .build()
                .unwrap(),
        );

    let output = FileSpec::builder_with("container.rs", Rust::new())
        .add_type(tb.build().unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    golden::assert_golden("rust/generic_struct.rs", &output);
}

#[test]
fn test_enum() {
    let derive = CodeBlock::of("#[derive(Debug, Clone, Copy)]", ()).unwrap();
    let tb = TypeSpec::builder("Color", TypeKind::Enum)
        .visibility(Visibility::Public)
        .annotation(derive)
        .add_variant(EnumVariantSpec::new("Red").unwrap())
        .add_variant(EnumVariantSpec::new("Green").unwrap())
        .add_variant(EnumVariantSpec::new("Blue").unwrap());

    let output = FileSpec::builder_with("color.rs", Rust::new())
        .add_type(tb.build().unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    golden::assert_golden("rust/enum.rs", &output);
}

#[test]
fn test_enum_tuple_variants() {
    let derive = CodeBlock::of("#[derive(Debug, Clone)]", ()).unwrap();
    let tb = TypeSpec::builder("Expr", TypeKind::Enum)
        .visibility(Visibility::Public)
        .annotation(derive)
        .add_variant(EnumVariantSpec::new("Unit").unwrap())
        .add_variant(
            EnumVariantSpec::builder("Literal")
                .associated_type(TypeName::primitive("i64"))
                .build()
                .unwrap(),
        )
        .add_variant(
            EnumVariantSpec::builder("Add")
                .associated_type(TypeName::generic(
                    TypeName::primitive("Box"),
                    vec![TypeName::primitive("Expr")],
                ))
                .associated_type(TypeName::generic(
                    TypeName::primitive("Box"),
                    vec![TypeName::primitive("Expr")],
                ))
                .build()
                .unwrap(),
        );

    let output = FileSpec::builder_with("expr.rs", Rust::new())
        .add_type(tb.build().unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    golden::assert_golden("rust/enum_tuple.rs", &output);
}

#[test]
fn test_enum_struct_variants() {
    let derive = CodeBlock::of("#[derive(Debug)]", ()).unwrap();
    let tb = TypeSpec::builder("Message", TypeKind::Enum)
        .visibility(Visibility::Public)
        .annotation(derive)
        .add_variant(EnumVariantSpec::new("Quit").unwrap())
        .add_variant(
            EnumVariantSpec::builder("Move")
                .add_field(
                    FieldSpec::builder("x", TypeName::primitive("i32"))
                        .build()
                        .unwrap(),
                )
                .add_field(
                    FieldSpec::builder("y", TypeName::primitive("i32"))
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .add_variant(
            EnumVariantSpec::builder("Write")
                .associated_type(TypeName::primitive("String"))
                .build()
                .unwrap(),
        )
        .add_variant(
            EnumVariantSpec::builder("ChangeColor")
                .add_field(
                    FieldSpec::builder("r", TypeName::primitive("u8"))
                        .build()
                        .unwrap(),
                )
                .add_field(
                    FieldSpec::builder("g", TypeName::primitive("u8"))
                        .build()
                        .unwrap(),
                )
                .add_field(
                    FieldSpec::builder("b", TypeName::primitive("u8"))
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap(),
        );

    let output = FileSpec::builder_with("message.rs", Rust::new())
        .add_type(tb.build().unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    golden::assert_golden("rust/enum_struct.rs", &output);
}
